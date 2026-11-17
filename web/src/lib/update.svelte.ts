// 设置页的一键更新：发起、轮询进度；服务重启后重载页面，用新版本号核对目标提交。
import { api, ApiError, type UpdateStatus } from "./api";

const TARGET = "micjz.update.to";

export const update = $state({ status: null as UpdateStatus | null, error: "" });

let polling = false;

const running = (s: UpdateStatus | null) => !!s && ["checking", "pulling", "building", "restarting"].includes(s.state);

/** 设置页打开时取一次；任务在跑就接着轮询。 */
export async function loadUpdate() {
  try {
    update.status = await api.updateStatus();
    if (running(update.status)) void poll();
  } catch (e) {
    update.error = (e as ApiError).message;
  }
}

export async function startUpdate() {
  update.error = "";
  try {
    await api.startUpdate();
  } catch (e) {
    update.error = (e as ApiError).message;
    return;
  }
  void poll();
}

/** 每秒取一次。开始构建后连不上都当作正在重启，等新进程回来就重载页面。 */
async function poll() {
  if (polling) return;
  polling = true;
  for (;;) {
    await new Promise((r) => setTimeout(r, 1000));
    let s: UpdateStatus;
    try {
      s = await api.updateStatus();
    } catch {
      if (sessionStorage.getItem(TARGET)) continue;
      break;
    }
    if (s.state === "building" || s.state === "restarting") sessionStorage.setItem(TARGET, s.to);
    // 重启前的进程最后一个状态是 restarting；新进程是 idle。
    if (s.state === "idle" && sessionStorage.getItem(TARGET)) return location.reload();
    update.status = s;
    if (!running(s)) {
      sessionStorage.removeItem(TARGET);
      break;
    }
  }
  polling = false;
}

/** 重载后由 boot 调用：有待核对的目标就给出结果文案。 */
export function checkUpdated(version: string): string | null {
  const to = sessionStorage.getItem(TARGET);
  if (!to) return null;
  sessionStorage.removeItem(TARGET);
  return version === to ? `已更新到 ${to}` : `更新后版本是 ${version}，不是目标 ${to}`;
}
