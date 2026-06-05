import { ArrowDownIcon, ArrowUpIcon, MinusIcon } from "lucide-react";

import { Badge } from "@/components/ui/badge";

interface Props {
  pct: number;
}

/** 趋势徽标：正绿、负红、零灰，显示百分比绝对值。 */
export function TrendBadge({ pct }: Props) {
  const variant = pct > 0 ? "success" : pct < 0 ? "danger" : "muted";
  const Icon = pct > 0 ? ArrowUpIcon : pct < 0 ? ArrowDownIcon : MinusIcon;
  return (
    <Badge variant={variant}>
      <Icon className="size-3" />
      {Math.abs(pct).toFixed(0)}%
    </Badge>
  );
}
