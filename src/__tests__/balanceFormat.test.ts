import { describe, expect, it } from "vitest";

import { fmtBalanceValue } from "@/lib/balanceFormat";

describe("fmtBalanceValue", () => {
  it("rounds balance display to whole numbers", () => {
    expect(fmtBalanceValue("10.417924550000002")).toBe("10");
    expect(fmtBalanceValue("18.5")).toBe("19");
    expect(fmtBalanceValue("18.320000")).toBe("18");
    expect(fmtBalanceValue(null)).toBe("-");
  });
});
