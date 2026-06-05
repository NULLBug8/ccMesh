import { describe, expect, it } from "vitest";

import { translate } from "@/lib/i18n";

describe("i18n translate", () => {
  it("返回中文导航文案", () => {
    expect(translate("zh", "nav.dashboard")).toBe("仪表盘");
  });

  it("返回英文导航文案", () => {
    expect(translate("en", "nav.dashboard")).toBe("Dashboard");
  });

  it("缺失键回落为 key 本身", () => {
    expect(translate("zh", "missing.key")).toBe("missing.key");
  });
});
