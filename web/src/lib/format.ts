import type { Fields } from "./types";

const WEEK = ["周日", "周一", "周二", "周三", "周四", "周五", "周六"];

export function money(cents: number, sign = false): string {
  if (!Number.isFinite(cents)) return "";
  const s = (Math.abs(cents) / 100).toLocaleString("en-US", { minimumFractionDigits: 2, maximumFractionDigits: 2 });
  return `${sign ? (cents < 0 ? "−" : "+") : ""}¥${s}`;
}

/** 金额输入框里的文本：去掉多余的 0。 */
export const yuanInput = (cents: number) => (cents / 100).toFixed(2).replace(/\.?0+$/, "");

export function parseCents(s: string): number | null {
  const m = s.trim().match(/^(\d+)(?:\.(\d{1,2}))?$/);
  if (!m) return null;
  const cents = Number(m[1]) * 100 + Number((m[2] ?? "").padEnd(2, "0"));
  return cents > 0 ? cents : null;
}

export const categoryLabel = (f: Pick<Fields, "category" | "subcategory">) =>
  f.subcategory ? `${f.category} › ${f.subcategory}` : f.category;

export const leaf = (f: Pick<Fields, "category" | "subcategory">) => f.subcategory ?? f.category;

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
 * 记账周期：每月 `day` 号开始，到下月 `day` 号前结束。
 * 周期用开始的那个月标识，如 "2026-11"；`day` 为 1 时就是自然月。
 */
export function periodOf(iso: string, day: number): string {
  const d = new Date(iso);
  const m = new Date(d.getFullYear(), d.getMonth() - (d.getDate() < day ? 1 : 0), 1);
  return `${m.getFullYear()}-${pad(m.getMonth() + 1)}`;
}

/** 周期的起止，`end` 不含。 */
export function periodRange(key: string, day: number): { start: Date; end: Date; days: number } {
  const [y, m] = key.split("-").map(Number);
  const start = new Date(y, m - 1, day);
  const end = new Date(y, m, day);
  return { start, end, days: Math.round((end.getTime() - start.getTime()) / 86400000) };
}

export function periodLabel(key: string, day: number): { title: string; sub: string } {
  const [y, m] = key.split("-").map(Number);
  if (day === 1) return { title: `${m}月`, sub: `${y}年` };
  const { start, end } = periodRange(key, day);
  const last = new Date(end.getTime() - 86400000);
  return { title: `${m}月`, sub: `${start.getMonth() + 1}.${start.getDate()} – ${last.getMonth() + 1}.${last.getDate()}` };
}

export function shiftPeriod(key: string, delta: number): string {
  const [y, m] = key.split("-").map(Number);
  const d = new Date(y, m - 1 + delta, 1);
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}`;
}
