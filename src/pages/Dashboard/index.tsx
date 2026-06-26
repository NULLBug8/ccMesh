import { StatCard, TokenHint } from "@/components/business";
import { PageLayoutEditor } from "@/components/business/page-layout/PageLayoutEditor";
import { PageSectionHost } from "@/components/business/page-layout/PageSectionHost";
import { RequestMonitor } from "@/components/business/RequestMonitor";
import { useStats } from "@/hooks/useStats";
import { resolveViewLayout, usePageLayoutStore } from "@/stores";
import { ServiceCard } from "./_components/ServiceCard";
import { dashboardLayoutDefinition } from "./layout";

export function Dashboard() {
  const { data } = useStats();
  const savedLayout = usePageLayoutStore((state) => state.getLayout("dashboard"));
  const layout = resolveViewLayout(dashboardLayoutDefinition, savedLayout);
  const today = data?.today;
  const tokens =
    (today?.inputTokens ?? 0) +
    (today?.outputTokens ?? 0) +
    (today?.cacheCreationTokens ?? 0) +
    (today?.cacheReadTokens ?? 0);

  return (
    <div className="flex w-full min-w-0 flex-col gap-6">
      <PageLayoutEditor view="dashboard" definition={dashboardLayoutDefinition} />
      <PageSectionHost
        layout={layout}
        registry={{
          hero: {
            className: "xl:col-span-12",
            title: "页面标题",
            render: () => (
              <header className="flex flex-col gap-1">
                <span className="text-[10px] font-medium tracking-[0.06em] text-ink-secondary uppercase">
                  Dashboard
                </span>
                <h1 className="text-2xl font-light tracking-tight">仪表盘</h1>
              </header>
            ),
          },
          service: {
            className: "min-w-0 xl:col-span-7",
            title: "服务状态",
            render: () => <ServiceCard />,
          },
          stats: {
            className: "min-w-0 xl:col-span-5",
            title: "今日指标",
            render: () => (
              <div className="grid min-w-0 grid-cols-1 gap-4 md:grid-cols-3 xl:grid-cols-1 2xl:grid-cols-3">
                <StatCard label="请求数（今日）" value={today?.requests ?? 0} />
                <StatCard label="失败数（今日）" value={today?.errors ?? 0} />
                <StatCard
                  label="Token（今日）"
                  value={tokens.toLocaleString()}
                  hint={<TokenHint value={tokens} />}
                  hintBelow
                />
              </div>
            ),
          },
          requests: {
            className: "min-w-0 xl:col-span-12",
            title: "实时请求",
            render: () => <RequestMonitor mode="live" pageSize={10} />,
          },
        }}
      />
    </div>
  );
}
