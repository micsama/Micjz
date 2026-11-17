/** 名字开头的 emoji 拆出来当图标，剩下的当文字；没有 emoji 就用首字。只管显示，名字本身由用户编辑。 */
export function badge(name: string): { icon: string; text: string } {
  const [first = ""] = Array.from(new Intl.Segmenter().segment(name), (s) => s.segment);
  if (!/\p{Extended_Pictographic}/u.test(first)) return { icon: first, text: name };
  return { icon: first, text: name.slice(first.length).trim() || name };
}

const WEEK = ["周日", "周一", "周二", "周三", "周四", "周五", "周六"];

export function money(cents: number, sign = false): string {
  if (!Number.isFinite(cents)) return "";
  const s = (Math.abs(cents) / 100).toLocaleString("en-US", { minimumFractionDigits: 2, maximumFractionDigits: 2 });
  return `${sign ? (cents < 0 ? "−" : "+") : ""}¥${s}`;
}

/** 金额输入框里的文本：去掉多余的 0。 */
export const yuanInput = (cents: number) => (cents / 100).toFixed(2).replace(/\.?0+$/, "");

/** 不带符号的金额文本转分，最多两位小数；「12.」算 12 元（键盘输到一半）。不合规为空。 */
export function yuanCents(s: string): number | null {
  const m = s.match(/^(\d+)(?:\.(\d{0,2}))?$/);
  if (!m) return null;
  const cents = Number(m[1]) * 100 + Number((m[2] ?? "").padEnd(2, "0"));
  return Number.isSafeInteger(cents) ? cents : null;
}

/** 金额文本转分；默认要大于 0，`signed` 允许负数（平账），但不能为 0。 */
export function parseCents(s: string, signed = false): number | null {
  const t = s.trim().replace(/^[−–]/, "-");
  const negative = t.startsWith("-");
  const cents = negative && !signed ? null : yuanCents(negative ? t.slice(1) : t);
  return cents ? (negative ? -cents : cents) : null;
}

const ymd = (d: Date) => `${d.getFullYear()}-${d.getMonth() + 1}-${d.getDate()}`;

export function dayLabel(iso: string, now = new Date()): string {
  const d = new Date(iso);
  const y = new Date(now);
  y.setDate(now.getDate() - 1);
  const md = `${d.getMonth() + 1}月${d.getDate()}日`;
  if (ymd(d) === ymd(now)) return `今天 · ${md}`;
  if (ymd(d) === ymd(y)) return `昨天 · ${md}`;
  return `${md} ${WEEK[d.getDay()]}`;
}

export const time = (iso: string) => {
  const d = new Date(iso);
  return `${String(d.getHours()).padStart(2, "0")}:${String(d.getMinutes()).padStart(2, "0")}`;
};

export function dateTime(iso: string): string {
  const d = new Date(iso);
  const year = d.getFullYear() === new Date().getFullYear() ? "" : `${d.getFullYear()}年`;
  return `${year}${d.getMonth() + 1}月${d.getDate()}日 ${WEEK[d.getDay()]} ${time(iso)}`;
}

/** `datetime-local` 输入框的值。 */
export function localInput(iso: string): string {
  const d = new Date(iso);
  const p = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}T${p(d.getHours())}:${p(d.getMinutes())}`;
}

export const dayKey = (iso: string) => ymd(new Date(iso));

const pad = (n: number) => String(n).padStart(2, "0");

/**
 * 记账周期按名义月标识，如 "2026-11"；年 "2026" 是名为 2026-01 到 2026-12 的那 12 期。
 * `day` 为正：本月 `day` 号起算本月（1 就是自然月）；为负：上月 `-day` 号起算本月。
 */
const monthStart = (y: number, m0: number, day: number) => (day > 0 ? new Date(y, m0, day) : new Date(y, m0 - 1, -day));

export function periodOf(iso: string, day: number): string {
  const d = new Date(iso);
  const y = d.getFullYear();
  let m0 = d.getMonth();
  if (d < monthStart(y, m0, day)) m0--;
  else if (d >= monthStart(y, m0 + 1, day)) m0++;
  const m = new Date(y, m0, 1);
  return `${m.getFullYear()}-${pad(m.getMonth() + 1)}`;
}

export const isYear = (key: string) => key.length === 4;

/** 月周期 `month` 是否落在 `key` 里（`key` 可以是月或年）。 */
export const within = (month: string, key: string) => (isYear(key) ? month.startsWith(`${key}-`) : month === key);

/** 周期的起止，`end` 不含。 */
export function periodRange(key: string, day: number): { start: Date; end: Date; days: number } {
  const [y, m = 1] = key.split("-").map(Number);
  const start = monthStart(y, m - 1, day);
  const end = isYear(key) ? monthStart(y + 1, 0, day) : monthStart(y, m, day);
  return { start, end, days: Math.round((end.getTime() - start.getTime()) / 86400000) };
}

/** 设置里周期的说法。 */
export const cycleText = (day: number) => (day === 1 ? "自然月" : day > 0 ? `本月 ${day} 号起` : `上月 ${-day} 号起`);

/** `title` 是大标题；`range` 只在周期不从 1 号开始时给出实际起止。 */
export function periodLabel(key: string, day: number): { title: string; year: string; range: string } {
  const [y, m] = key.split("-").map(Number);
  const title = isYear(key) ? `${y}年` : `${m}月`;
  if (day === 1) return { title, year: `${y}年`, range: "" };
  const { start, end } = periodRange(key, day);
  const last = new Date(end.getTime() - 86400000);
  return { title, year: `${y}年`, range: `${start.getMonth() + 1}.${start.getDate()} – ${last.getMonth() + 1}.${last.getDate()}` };
}

export function shiftPeriod(key: string, delta: number): string {
  if (isYear(key)) return String(Number(key) + delta);
  const [y, m] = key.split("-").map(Number);
  const d = new Date(y, m - 1 + delta, 1);
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}`;
}

/** 当前期与前一期的同期截止时间；已结束的周期比较整期。 */
export function comparisonEnds(period: string, day: number, now = new Date()) {
  const current = periodRange(period, day);
  const previous = periodRange(shiftPeriod(period, -1), day);
  if (now >= current.end) return { current: current.end, previous: previous.end };
  const target = new Date(
    now.getFullYear() - (isYear(period) ? 1 : 0),
    now.getMonth() - (isYear(period) ? 0 : 1),
    1,
    now.getHours(),
    now.getMinutes(),
    now.getSeconds(),
    now.getMilliseconds(),
  );
  const last = new Date(target.getFullYear(), target.getMonth() + 1, 0).getDate();
  target.setDate(Math.min(now.getDate(), last));
  return { current: now, previous: new Date(Math.min(target.getTime(), previous.end.getTime())) };
}

/** 时间格式可混用 UTC 和时区偏移，排序必须比较实际时刻。 */
export const newestFirst = (a: { occurred_at: string }, b: { occurred_at: string }) => Date.parse(b.occurred_at) - Date.parse(a.occurred_at);
