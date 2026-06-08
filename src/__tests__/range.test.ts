import { describe, expect, it } from "vitest";

import { rangeDates, rangeMs, startOfTodayMs } from "@/lib/range";

const DAY = 86_400_000;
// 固定锚点：2026-06-07 00:00 本地时间（用具体时刻推导，避免依赖运行时"今天"）
const anchor = new Date(2026, 5, 7, 0, 0, 0, 0).getTime();

describe("startOfTodayMs", () => {
  it("按天对齐：同一天内任意时刻得到相同的 0 点锚点", () => {
    const morning = new Date(2026, 5, 7, 8, 30, 12, 345).getTime();
    const night = new Date(2026, 5, 7, 23, 59, 59, 999).getTime();
    expect(startOfTodayMs(morning)).toBe(anchor);
    expect(startOfTodayMs(night)).toBe(anchor);
  });
});

describe("rangeMs（稳定性 + 边界）", () => {
  it("同一 key 同一锚点结果相等（防 queryKey 漂移回归）", () => {
    expect(rangeMs("today", anchor)).toEqual(rangeMs("today", anchor));
    expect(rangeMs("7d", anchor)).toEqual(rangeMs("7d", anchor));
  });

  it("today：当日 0 点 → 次日 0 点", () => {
    expect(rangeMs("today", anchor)).toEqual({
      startMs: anchor,
      endMs: anchor + DAY,
    });
  });

  it("7d/30d：以今天为锚按天回溯，上界为次日 0 点", () => {
    expect(rangeMs("7d", anchor)).toEqual({
      startMs: anchor - 6 * DAY,
      endMs: anchor + DAY,
    });
    expect(rangeMs("30d", anchor)).toEqual({
      startMs: anchor - 29 * DAY,
      endMs: anchor + DAY,
    });
  });

  it("all：无界", () => {
    expect(rangeMs("all", anchor)).toEqual({});
  });
});

describe("rangeDates（本地 YYYY-MM-DD）", () => {
  it("today：闭区间为今天", () => {
    expect(rangeDates("today", anchor)).toEqual({
      start: "2026-06-07",
      end: "2026-06-07",
    });
  });

  it("7d：[today-6, today]", () => {
    expect(rangeDates("7d", anchor)).toEqual({
      start: "2026-06-01",
      end: "2026-06-07",
    });
  });

  it("all：无界", () => {
    expect(rangeDates("all", anchor)).toEqual({});
  });
});
