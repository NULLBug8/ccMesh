import { useMutation } from "@tanstack/react-query";
import { RefreshCwIcon } from "lucide-react";
import { toast } from "sonner";

import { PageLayoutEditor } from "@/components/business/page-layout/PageLayoutEditor";
import { PageSectionHost } from "@/components/business/page-layout/PageSectionHost";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { useEndpoints } from "@/hooks/useEndpoints";
import { endpointApi, type EndpointBalanceResult } from "@/services/modules/endpoint";
import { resolveViewLayout, usePageLayoutStore } from "@/stores";
import { balancesLayoutDefinition } from "./layout";

const errMsg = (e: unknown) => (e instanceof Error ? e.message : String(e));

function BalanceSummary({ result }: { result?: EndpointBalanceResult }) {
  if (!result) return <span className="text-xs text-ink-mute">未查询</span>;
  if (!result.success) return <span className="text-xs text-danger">{result.message}</span>;
  return (
    <span className="text-sm font-medium text-success">
      {result.balance}
      {result.currency ? ` ${result.currency}` : ""}
    </span>
  );
}

export function Balances() {
  const { data: endpoints, isLoading } = useEndpoints();
  const savedLayout = usePageLayoutStore((s) => s.getLayout("balances"));
  const layout = resolveViewLayout(balancesLayoutDefinition, savedLayout);
  const query = useMutation({
    mutationFn: async (ids: number[]) => {
      const pairs = await Promise.all(
        ids.map(async (id) => [id, await endpointApi.queryBalance(id)] as const),
      );
      return Object.fromEntries(pairs) as Record<number, EndpointBalanceResult>;
    },
    onSuccess: () => toast.success("余额查询完成"),
    onError: (e) => toast.error(errMsg(e)),
  });

  const rows = endpoints ?? [];
  const enabledRows = rows.filter((ep) => ep.balanceQuery?.enabled);
  const results = query.data ?? {};

  return (
    <div className="flex w-full min-w-0 flex-col gap-6">
      <PageLayoutEditor view="balances" definition={balancesLayoutDefinition} />
      <PageSectionHost
        layout={layout}
        registry={{
          header: {
            className: "xl:col-span-12",
            title: "标题",
            render: () => (
              <div className="flex flex-wrap items-end justify-between gap-4">
                <div>
                  <div className="text-xs font-semibold uppercase tracking-[0.22em] text-primary">
                    BALANCE
                  </div>
                  <h1 className="text-2xl font-semibold tracking-tight">余额查询</h1>
                  <p className="mt-1 text-sm text-ink-mute">
                    集中查看中转站点余额，模板支持 URL、Method、Headers、Body 和 JSON Path 提取。
                  </p>
                </div>
                <Button
                  aria-label="查询全部余额"
                  disabled={query.isPending || enabledRows.length === 0}
                  onClick={() => query.mutate(enabledRows.map((ep) => ep.id))}
                >
                  <RefreshCwIcon className="size-4" />
                  查询全部余额
                </Button>
              </div>
            ),
          },
          balanceList: {
            className: "xl:col-span-12",
            title: "余额列表",
            render: () => (
              <div className="grid gap-4 2xl:grid-cols-2">
                {isLoading ? (
                  <Card>
                    <CardContent className="p-5 text-sm text-ink-mute">加载中...</CardContent>
                  </Card>
                ) : (
                  rows.map((ep) => (
                    <Card key={ep.id} className="overflow-hidden">
                      <CardContent className="flex flex-col gap-4 p-5">
                        <div className="flex items-start justify-between gap-4">
                          <div>
                            <div className="text-base font-semibold">{ep.name}</div>
                            <div className="mt-1 text-xs text-ink-mute">{ep.apiUrl}</div>
                          </div>
                          <BalanceSummary result={results[ep.id]} />
                        </div>
                        <div className="grid gap-2 rounded-lg border border-edge-subtle bg-background/70 p-3 text-xs">
                          <div className="flex justify-between gap-3">
                            <span className="text-ink-mute">模板</span>
                            <span>{ep.balanceQuery?.templateId || "未配置"}</span>
                          </div>
                          <div className="flex justify-between gap-3">
                            <span className="text-ink-mute">路径</span>
                            <span className="break-all text-right">{ep.balanceQuery?.path || "-"}</span>
                          </div>
                          <div className="flex justify-between gap-3">
                            <span className="text-ink-mute">提取</span>
                            <span>{ep.balanceQuery?.extraction?.balancePath || "-"}</span>
                          </div>
                        </div>
                        <Button
                          variant="outline"
                          disabled={!ep.balanceQuery?.enabled || query.isPending}
                          onClick={() => query.mutate([ep.id])}
                        >
                          查询此站点
                        </Button>
                      </CardContent>
                    </Card>
                  ))
                )}
              </div>
            ),
          },
        }}
      />
    </div>
  );
}
