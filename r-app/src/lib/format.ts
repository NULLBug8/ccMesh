/**
 * Token 数量的辅助单位文案：就近取量级，约等号 + 两位小数 + 中文单位。
 * - ≥ 1 亿：`≈1.25亿`
 * - ≥ 1 万：`≈900.00万`
 * - 否则：原始数字（千分位），不加单位与约等号
 *
 * 主数值仍应展示精确值，本函数仅产出"辅助小字"文案。
 * 非有限值按 `"0"` 处理；负数取绝对值折算并保留负号。
 */
export function formatTokenCompact(n: number): string {
  if (!Number.isFinite(n)) return "0";
  const sign = n < 0 ? "-" : "";
  const abs = Math.abs(n);
  if (abs >= 1e8) return `${sign}≈${(abs / 1e8).toFixed(2)}亿`;
  if (abs >= 1e4) return `${sign}≈${(abs / 1e4).toFixed(2)}万`;
  return n.toLocaleString();
}
