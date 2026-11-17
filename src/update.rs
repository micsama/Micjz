//! 网页一键更新：fetch → 覆盖到 upstream → `./build.sh build` → 停服后 exec `./build.sh start`。契约：docs/blueprints/self-update.md。
use std::{path::PathBuf, process::Stdio, sync::Mutex, time::Duration};

use serde::Serialize;
use tokio::{
    io::{AsyncBufReadExt, AsyncRead, BufReader},
    process::Command,
    sync::Notify,
};

/// `git fetch` 各次的超时，次数即长度：代理首连常失败，下一次通常就好。
const PULL_TIMEOUTS: [u64; 5] = [20, 30, 40, 50, 60];
const OUTPUT_BYTES: usize = 64 * 1024;

pub struct Updater {
    repo: PathBuf,
    /// 传给 build.sh 的配置文件路径。
    config: PathBuf,
    /// 构建成功后由 main 停服并 exec。
    pub restart: Notify,
    task: Mutex<Task>,
}

#[derive(Serialize)]
pub struct Task {
    #[serde(flatten)]
    phase: Phase,
    output: String,
}

#[derive(Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
enum Phase {
    Idle,
    Checking,
    Pulling { from: String, attempt: usize },
    Building { from: String, to: String },
    Restarting { from: String, to: String },
    UpToDate { commit: String },
    Failed { stage: &'static str, error: String },
}

impl Phase {
    fn stage(&self) -> Option<&'static str> {
        match self {
            Self::Checking => Some("checking"),
            Self::Pulling { .. } => Some("pulling"),
            Self::Building { .. } => Some("building"),
            Self::Restarting { .. } => Some("restarting"),
            Self::Idle | Self::UpToDate { .. } | Self::Failed { .. } => None,
        }
    }
}

impl Updater {
    pub fn new(repo: PathBuf, config: PathBuf) -> Self {
        Self {
            repo,
            config,
            restart: Notify::new(),
            task: Mutex::new(Task {
                phase: Phase::Idle,
                output: String::new(),
            }),
        }
    }

    /// 停服后 exec 它：和手动更新同一条路，备份、按需转换库后启动新版本。
    pub fn start_command(&self) -> std::process::Command {
        let mut cmd = std::process::Command::new(self.repo.join("build.sh"));
        cmd.arg("start").arg(&self.config);
        cmd
    }

    pub fn view<R>(&self, f: impl FnOnce(&Task) -> R) -> R {
        f(&self.task.lock().unwrap())
    }

    /// 已有任务在跑返回 false。
    pub fn start(self: &std::sync::Arc<Self>) -> bool {
        {
            let mut task = self.task.lock().unwrap();
            if task.phase.stage().is_some() {
                return false;
            }
            *task = Task {
                phase: Phase::Checking,
                output: String::new(),
            };
        }
        tracing::info!(repo = %self.repo.display(), "开始更新");
        let this = self.clone();
        tokio::spawn(async move {
            if let Err(error) = this.run().await {
                tracing::warn!(%error, "更新失败");
                let mut task = this.task.lock().unwrap();
                let stage = task.phase.stage().unwrap_or("checking");
                task.phase = Phase::Failed { stage, error };
            }
        });
        true
    }

    async fn run(&self) -> Result<(), String> {
        self.git(&["rev-parse", "@{u}"])
            .await
            .map_err(|e| format!("当前分支没有 upstream：{e}"))?;
        // 以运行中的版本为准：上次构建失败时源码已前进，进程仍是旧版。不是 build.sh 构建的（dev）一律重建。
        let from = option_env!("MICJZ_VERSION").unwrap_or("dev").to_owned();
        self.fetch(&from).await?;

        // 服务器上不手改代码：已跟踪文件的改动一律以 upstream 覆盖，先列出来。
        let dirty = self
            .git(&["status", "--porcelain", "--untracked-files=no"])
            .await?;
        if !dirty.is_empty() {
            self.append(&format!("以下本地改动将被覆盖：\n{dirty}"));
        }
        let reset = self.git(&["reset", "--hard", "@{u}"]).await?;
        self.append(&reset);
        let to = self.git(&["rev-parse", "--short", "HEAD"]).await?;
        if to == from {
            self.set(Phase::UpToDate { commit: to });
            return Ok(());
        }

        self.set(Phase::Building {
            from: from.clone(),
            to: to.clone(),
        });
        tracing::info!(%from, %to, "更新：构建");
        let mut build = Command::new("./build.sh");
        build.arg("build").arg(&self.config);
        let ok = self
            .stream(build)
            .await
            .map_err(|e| format!("无法运行 build.sh：{e}"))?;
        if !ok {
            return Err("构建失败，旧服务继续运行".into());
        }
        tracing::info!(%from, %to, "更新：重启");
        self.set(Phase::Restarting { from, to });
        self.restart.notify_one();
        Ok(())
    }

    async fn fetch(&self, from: &str) -> Result<(), String> {
        for (i, secs) in PULL_TIMEOUTS.into_iter().enumerate() {
            self.set(Phase::Pulling {
                from: from.to_owned(),
                attempt: i + 1,
            });
            self.append(&format!(
                "$ git fetch（第 {}/{} 次，{secs} 秒超时）",
                i + 1,
                PULL_TIMEOUTS.len()
            ));
            let mut git = Command::new("git");
            git.arg("fetch");
            match tokio::time::timeout(Duration::from_secs(secs), self.stream(git)).await {
                Ok(Ok(true)) => return Ok(()),
                Ok(Ok(false)) => self.append("git fetch 失败"),
                Ok(Err(e)) => return Err(format!("无法运行 git：{e}")),
                Err(_) => self.append("git fetch 超时，已终止"),
            }
        }
        Err(format!(
            "git fetch {} 次都失败，请检查网络或启动前是否开了代理",
            PULL_TIMEOUTS.len()
        ))
    }

    /// 输出实时并入缓冲；future 被丢弃时子进程随之终止。
    async fn stream(&self, mut cmd: Command) -> std::io::Result<bool> {
        let mut child = cmd
            .current_dir(&self.repo)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()?;
        let out = child.stdout.take().expect("已设为 piped");
        let err = child.stderr.take().expect("已设为 piped");
        let (status, (), ()) = tokio::join!(child.wait(), self.pump(out), self.pump(err));
        Ok(status?.success())
    }

    async fn pump(&self, pipe: impl AsyncRead + Unpin) {
        let mut reader = BufReader::new(pipe);
        let mut line = Vec::new();
        while let Ok(n) = reader.read_until(b'\n', &mut line).await {
            if n == 0 {
                break;
            }
            self.append(&String::from_utf8_lossy(&line));
            line.clear();
        }
    }

    /// 只取 stdout（去首尾空白）；失败时以 stderr 作原因。
    async fn git(&self, args: &[&str]) -> Result<String, String> {
        let out = Command::new("git")
            .args(args)
            .current_dir(&self.repo)
            .stdin(Stdio::null())
            .output()
            .await
            .map_err(|e| format!("无法运行 git：{e}"))?;
        if !out.status.success() {
            let stderr = String::from_utf8_lossy(&out.stderr);
            return Err(format!("git {} 失败：{}", args.join(" "), stderr.trim()));
        }
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_owned())
    }

    fn set(&self, phase: Phase) {
        self.task.lock().unwrap().phase = phase;
    }

    fn append(&self, text: &str) {
        let mut task = self.task.lock().unwrap();
        let output = &mut task.output;
        output.push_str(text);
        if !output.ends_with('\n') {
            output.push('\n');
        }
        if output.len() > OUTPUT_BYTES {
            let mut cut = output.len() - OUTPUT_BYTES;
            while !output.is_char_boundary(cut) {
                cut += 1;
            }
            output.drain(..cut);
        }
    }
}
