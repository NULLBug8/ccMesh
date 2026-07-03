import { describe, expect, it } from "vitest";

import { circuitDot } from "@/services/modules/health";

describe("circuitDot", () => {
  it("熔断态映射到状态点颜色", () => {
    expect(circuitDot("open")).toBe("danger");
    expect(circuitDot("halfOpen")).toBe("warning");
    expect(circuitDot("closed")).toBe("success");
  });
});
