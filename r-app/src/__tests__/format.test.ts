import { describe, expect, it } from "vitest";

import { formatTokenCompact } from "@/lib/format";

describe("formatTokenCompact", () => {
  it("不足一万：原样千分位，不加单位与约等号", () => {
    expect(formatTokenCompact(0)).toBe("0");
    expect(formatTokenCompact(999)).toBe("999");
    expect(formatTokenCompact(9999)).toBe("9,999");
  });

  it("万档：≈ + 两位小数 + 万", () => {
    expect(formatTokenCompact(10000)).toBe("≈1.00万");
    expect(formatTokenCompact(20000)).toBe("≈2.00万");
    expect(formatTokenCompact(9_000_000)).toBe("≈900.00万");
    // 1000 万仍属万档（< 1 亿）
    expect(formatTokenCompact(10_000_000)).toBe("≈1000.00万");
    // 接近 1 亿但仍 < 1e8：留在万档，两位小数四舍五入
    expect(formatTokenCompact(99_999_999)).toBe("≈10000.00万");
  });

  it("亿档：≈ + 两位小数 + 亿", () => {
    expect(formatTokenCompact(100_000_000)).toBe("≈1.00亿");
    expect(formatTokenCompact(125_000_000)).toBe("≈1.25亿");
  });

  it("非有限值按 0 处理", () => {
    expect(formatTokenCompact(Number.NaN)).toBe("0");
    expect(formatTokenCompact(Number.POSITIVE_INFINITY)).toBe("0");
  });

  it("负数取绝对值折算并保留负号", () => {
    expect(formatTokenCompact(-20000)).toBe("-≈2.00万");
    expect(formatTokenCompact(-500)).toBe("-500");
  });
});
