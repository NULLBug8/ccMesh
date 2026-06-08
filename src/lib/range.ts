/**
 * 时间段筛选的稳定计算工具。
 *
 * 关键不变量：区间由"当天 0 点"锚点（`startOfTodayMs`，按天对齐）推导，
 * 不含每帧变化的 `Date.now()`，因此同一 `RangeKey` 在同一天内多次调用结果相等。
 * 这样把结果放进 React Query 的 queryKey 不会逐帧漂移、导致无限重取。
 */

export type RangeKey = "today" | "7d" | "30d" | "all";

export const RANGE_OPTIONS: { key: RangeKey; label: string }[] = [
  { key: "today", label: "今日" },
  { key: "7d", label: "近 7 天" },
  { key: "30d", label: "近 30 天" },
  { key: "all", label: "全部" },
];

const DAY_MS = 86_400_000;

/** 当天 0 点的毫秒时间戳（本地时区）。按天对齐，作为稳定锚点。 */
export function startOfTodayMs(now: number = Date.now()): number {
  const d = new Date(now);
  d.setHours(0, 0, 0, 0);
  return d.getTime();
}

/**
 * 毫秒区间（用于 `request_logs` 的 ts 过滤，Unix 毫秒）。
 * 上界取"次日 0 点"以覆盖一整天且保持稳定。
 */
export function rangeMs(
  key: RangeKey,
  todayStartMs: number,
): { startMs?: number; endMs?: number } {
  const tomorrow = todayStartMs + DAY_MS;
  switch (key) {
    case "today":
      return { startMs: todayStartMs, endMs: tomorrow };
    case "7d":
      return { startMs: todayStartMs - 6 * DAY_MS, endMs: tomorrow };
    case "30d":
      return { startMs: todayStartMs - 29 * DAY_MS, endMs: tomorrow };
    case "all":
      return {};
  }
}

/** 本地日期 `YYYY-MM-DD`。 */
function ymd(ms: number): string {
  const d = new Date(ms);
  const y = d.getFullYear();
  const m = String(d.getMonth() + 1).padStart(2, "0");
  const day = String(d.getDate()).padStart(2, "0");
  return `${y}-${m}-${day}`;
}

/**
 * 本地日期区间 `YYYY-MM-DD`（用于用量统计 `start/end`，后端按本地 date 聚合）。
 * 闭区间：今日为 `[today, today]`，近 N 天为 `[today-(N-1), today]`。
 */
export function rangeDates(
  key: RangeKey,
  todayStartMs: number,
): { start?: string; end?: string } {
  const today = ymd(todayStartMs);
  switch (key) {
    case "today":
      return { start: today, end: today };
    case "7d":
      return { start: ymd(todayStartMs - 6 * DAY_MS), end: today };
    case "30d":
      return { start: ymd(todayStartMs - 29 * DAY_MS), end: today };
    case "all":
      return {};
  }
}
