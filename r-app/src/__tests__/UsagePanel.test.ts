import { describe, expect, it } from "vitest";

import { groupByDate } from "@/pages/Statistics/_components/UsagePanel";
import type { DayModelUsage } from "@/services/modules/usage";

const row = (date: string, model: string): DayModelUsage => ({
  date,
  appType: "claude",
  model,
  requests: 1,
  inputTokens: 1,
  outputTokens: 1,
  cacheCreationTokens: 0,
  cacheReadTokens: 0,
});

describe("groupByDate", () => {
  it("连续同日期行聚为一组，保持后端排序", () => {
    const groups = groupByDate([
      row("2026-06-08", "opus"),
      row("2026-06-08", "mimo"),
      row("2026-06-07", "opus"),
    ]);
    expect(groups).toHaveLength(2);
    expect(groups[0].date).toBe("2026-06-08");
    expect(groups[0].rows.map((r) => r.model)).toEqual(["opus", "mimo"]);
    expect(groups[1].date).toBe("2026-06-07");
    expect(groups[1].rows).toHaveLength(1);
  });

  it("空输入返回空组", () => {
    expect(groupByDate([])).toEqual([]);
  });
});
