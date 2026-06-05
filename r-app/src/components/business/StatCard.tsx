import type { ReactNode } from "react";

import { TabularText } from "@/components/ui";
import { Card, CardContent } from "@/components/ui/card";

interface Props {
  label: string;
  value: number | string;
  hint?: ReactNode;
}

/** 跨页面业务卡片：标签 + 大号数值 + 可选提示（Statistics / Dashboard 共用）。 */
export function StatCard({ label, value, hint }: Props) {
  return (
    <Card>
      <CardContent className="flex flex-col gap-1.5 px-5 py-4">
        <span className="text-xs text-ink-secondary">{label}</span>
        <div className="flex items-center justify-between gap-2">
          <TabularText className="text-2xl text-foreground">{value}</TabularText>
          {hint}
        </div>
      </CardContent>
    </Card>
  );
}
