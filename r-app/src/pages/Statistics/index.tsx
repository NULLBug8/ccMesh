import { useState } from "react";

import { StatCard } from "@/components/business";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { useStats } from "@/hooks/useStats";
import type { PeriodStats } from "@/services/modules/stats";
import { EndpointStatsTable } from "./_components/EndpointStatsTable";
import { HistoryPanel } from "./_components/HistoryPanel";
import { TrendBadge } from "./_components/TrendBadge";

const PERIODS = [
  { key: "today", label: "今日" },
  { key: "yesterday", label: "昨日" },
  { key: "thisWeek", label: "本周" },
  { key: "thisMonth", label: "本月" },
] as const;

type PeriodKey = (typeof PERIODS)[number]["key"];

export function Statistics() {
  const { data, isLoading } = useStats();
  const [period, setPeriod] = useState<PeriodKey>("today");

  const stats: PeriodStats | undefined = data?.[period];
  const trend = data?.trend;
  const showTrend = period === "today" && trend;

  return (
    <div className="mx-auto flex max-w-4xl flex-col gap-6">
      <h1 className="text-2xl font-light tracking-tight">统计</h1>

      <Tabs value={period} onValueChange={(v) => setPeriod(v as PeriodKey)}>
        <TabsList>
          {PERIODS.map((p) => (
            <TabsTrigger key={p.key} value={p.key}>
              {p.label}
            </TabsTrigger>
          ))}
        </TabsList>
      </Tabs>

      {isLoading ? (
        <p className="text-sm text-ink-mute">加载中…</p>
      ) : (
        <>
          <div className="grid grid-cols-4 gap-4">
            <StatCard
              label="请求"
              value={stats?.requests ?? 0}
              hint={showTrend ? <TrendBadge pct={trend.requestsPct} /> : undefined}
            />
            <StatCard label="错误" value={stats?.errors ?? 0} />
            <StatCard
              label="输入 Token"
              value={stats?.inputTokens ?? 0}
              hint={showTrend ? <TrendBadge pct={trend.inputTokensPct} /> : undefined}
            />
            <StatCard
              label="输出 Token"
              value={stats?.outputTokens ?? 0}
              hint={showTrend ? <TrendBadge pct={trend.outputTokensPct} /> : undefined}
            />
          </div>

          <EndpointStatsTable rows={stats?.endpoints ?? []} />
          <HistoryPanel />
        </>
      )}
    </div>
  );
}
