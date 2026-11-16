//! 网页账号：Argon2 密码、无状态签名会话、登录失败窗口；快捷指令 token 独立。
use anyhow::{Context, ensure};
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use ring::{
    hmac,
    rand::{SecureRandom, SystemRandom},
};
use rusqlite::{Connection, OptionalExtension, params};
use std::{
    collections::VecDeque,
    io::{self, IsTerminal, Write},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

pub const USERNAME: &str = "dzmfg";
pub const SESSION_SECONDS: i64 = 30 * 24 * 60 * 60;
const DAY_MS: i64 = 24 * 60 * 60 * 1000;
const LOGIN_WINDOW: Duration = Duration::from_secs(10 * 60);

#[derive(Default)]
pub struct LoginLimit(VecDeque<Instant>);
impl LoginLimit {
    pub fn blocked(&mut self, now: Instant) -> bool {
        while self
            .0
            .front()
            .is_some_and(|at| now.duration_since(*at) >= LOGIN_WINDOW)
        {
            self.0.pop_front();
        }
        self.0.len() >= 10
    }
    pub fn failed(&mut self, now: Instant) {
        self.0.push_back(now);
    }
}

#[derive(Clone)]
pub struct Admin {
    pub password_hash: String,
    pub session_key: Vec<u8>,
}
pub fn load(conn: &Connection) -> rusqlite::Result<Option<Admin>> {
    conn.query_row(
        "SELECT password_hash, session_key FROM web_admin WHERE id=1",
        [],
        |r| {
            Ok(Admin {
                password_hash: r.get(0)?,
                session_key: r.get(1)?,
            })
        },
    )
    .optional()
}
pub fn check_new_password(password: &str) -> anyhow::Result<()> {
    ensure!(password.chars().count() >= 8, "新密码至少 8 位");
    Ok(())
}
impl Admin {
    pub fn for_password(password: &str) -> anyhow::Result<Self> {
        check_new_password(password)?;
        let password_hash = Argon2::default()
            .hash_password(password.as_bytes())
            .context("生成密码哈希")?
            .to_string();
        let mut session_key = vec![0; 32];
        SystemRandom::new()
            .fill(&mut session_key)
            .map_err(|_| anyhow::anyhow!("生成会话密钥失败"))?;
        Ok(Self {
            password_hash,
            session_key,
        })
    }
    pub fn verify_password(&self, password: &str) -> anyhow::Result<bool> {
        let hash = PasswordHash::new(&self.password_hash).context("读取密码哈希")?;
        match Argon2::default().verify_password(password.as_bytes(), &hash) {
            Ok(()) => Ok(true),
            Err(argon2::password_hash::Error::PasswordInvalid) => Ok(false),
            Err(e) => Err(anyhow::Error::new(e).context("校验密码")),
        }
    }
    pub fn session(&self, now_ms: i64) -> String {
        let expires = (now_ms + SESSION_SECONDS * 1000).to_string();
        let key = hmac::Key::new(hmac::HMAC_SHA256, &self.session_key);
        let signature = hmac::sign(&key, expires.as_bytes());
        format!("{expires}.{}", URL_SAFE_NO_PAD.encode(signature.as_ref()))
    }
    /// 已校验的会话只暴露是否需要续期，不把未验证的时间传给内部。
    pub fn verify_session(&self, value: &str, now_ms: i64) -> Option<bool> {
        let (expires, signature) = value.split_once('.')?;
        let signature = URL_SAFE_NO_PAD.decode(signature).ok()?;
        let key = hmac::Key::new(hmac::HMAC_SHA256, &self.session_key);
        hmac::verify(&key, expires.as_bytes(), &signature).ok()?;
        let expires: i64 = expires.parse().ok()?;
        if expires <= now_ms {
            return None;
        }
        Some(expires - now_ms < SESSION_SECONDS * 1000 - DAY_MS)
    }
}
pub fn store(conn: &Connection, admin: &Admin) -> rusqlite::Result<()> {
    conn.execute("INSERT INTO web_admin(id,password_hash,session_key) VALUES (1,?1,?2)
        ON CONFLICT(id) DO UPDATE SET password_hash=excluded.password_hash, session_key=excluded.session_key",params![admin.password_hash,admin.session_key])?;
    Ok(())
}

pub fn password_cli(db: &crate::db::Db) -> anyhow::Result<()> {
    let first = read_hidden("新密码（至少 8 位）：")?;
    check_new_password(&first)?;
    let second = read_hidden("再输一次：")?;
    ensure!(first == second, "两次输入的密码不一致");
    let admin = Admin::for_password(&first)?;
    store(&db.conn(), &admin).context("保存网页登录密码")?;
    println!("已设置网页登录密码（用户名 {USERNAME}），旧会话已失效");
    Ok(())
}
struct EchoGuard(Option<String>);
impl EchoGuard {
    fn restore(&mut self) -> anyhow::Result<()> {
        if let Some(mode) = self.0.take() {
            ensure!(
                Command::new("stty")
                    .arg(mode)
                    .stdin(Stdio::inherit())
                    .status()?
                    .success(),
                "恢复终端回显失败"
            );
        }
        Ok(())
    }
}
impl Drop for EchoGuard {
    fn drop(&mut self) {
        if let Err(e) = self.restore() {
            tracing::error!(error=%format!("{e:#}"),"恢复终端失败");
        }
    }
}
fn read_hidden(prompt: &str) -> anyhow::Result<String> {
    let terminal = io::stdin().is_terminal();
    let mut guard = EchoGuard(None);
    if terminal {
        print!("{prompt}");
        io::stdout().flush()?;
        let mode = Command::new("stty")
            .arg("-g")
            .stdin(Stdio::inherit())
            .output()?;
        ensure!(mode.status.success(), "读取终端状态失败");
        guard.0 = Some(String::from_utf8(mode.stdout)?.trim().to_owned());
        ensure!(
            Command::new("stty")
                .arg("-echo")
                .stdin(Stdio::inherit())
                .status()?
                .success(),
            "关闭终端回显失败"
        );
    }
    let mut value = String::new();
    let read = io::stdin().read_line(&mut value);
    guard.restore()?;
    if terminal {
        println!();
    }
    ensure!(read? > 0, "没有收到密码输入");
    Ok(value.trim_end_matches(['\r', '\n']).to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn session_expiry_renewal_and_key_rotation() {
        let a = Admin {
            password_hash: String::new(),
            session_key: vec![1; 32],
        };
        let now = 1_800_000_000_000;
        let cookie = a.session(now);
        assert_eq!(a.verify_session(&cookie, now), Some(false));
        assert_eq!(a.verify_session(&cookie, now + DAY_MS + 1), Some(true));
        assert_eq!(
            a.verify_session(&cookie, now + SESSION_SECONDS * 1000),
            None
        );
        assert_eq!(a.verify_session(&(cookie.clone() + "x"), now), None);
        let b = Admin {
            password_hash: String::new(),
            session_key: vec![2; 32],
        };
        assert_eq!(b.verify_session(&cookie, now), None);
    }
    #[test]
    fn login_failure_window_expires() {
        let now = Instant::now();
        let mut limit = LoginLimit::default();
        for _ in 0..10 {
            assert!(!limit.blocked(now));
            limit.failed(now);
        }
        assert!(limit.blocked(now));
        assert!(!limit.blocked(now + LOGIN_WINDOW));
    }
}
