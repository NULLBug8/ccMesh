import { useMutation } from "@tanstack/react-query";
import { RefreshCwIcon } from "lucide-react";
import { toast } from "sonner";

import { PageLayoutEditor } from "@/components/business/page-layout/PageLayoutEditor";
import { PageSectionHost } from "@/components/business/page-layout/PageSectionHost";
import { Button } from "@/components/ui/button";
import { useEndpoints } from "@/hooks/useEndpoints";
import { endpointApi, type EndpointBalanceResult } from "@/services/modules/endpoint";
import { resolveViewLayout, usePageLayoutStore } from "@/stores";
import { balancesLayoutDefinition } from "./layout";

const errMsg = (e: unknown) => (e instanceof Error ? e.message : String(e));

function BalanceSummary({ result }: { result?: EndpointBalanceResult }) {
  if (!result) return <span className="rounded-full bg-surface-hover px-2 py-1 text-xs text-ink-mute">未查询</span>;
  if (!result.success) return <span className="text-xs font-medium text-danger">{result.message}</span>;
  return (
    <span className="rounded-full bg-success/10 px-2 py-1 text-sm font-medium text-success">
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
              <div className="rounded-2xl border border-edge-subtle bg-gradient-to-br from-surface-raised to-background p-5 shadow-sm">
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
                <div className="mt-4 grid gap-3 sm:grid-cols-3">
                  <div className="rounded-xl border border-edge-subtle bg-background/70 p-3">
                    <div className="text-xs text-ink-mute">站点数</div>
                    <div className="mt-1 text-xl font-semibold">{rows.length}</div>
                  </div>
                  <div className="rounded-xl border border-edge-subtle bg-background/70 p-3">
                    <div className="text-xs text-ink-mute">可查询</div>
                    <div className="mt-1 text-xl font-semibold">{enabledRows.length}</div>
                  </div>
                  <div className="rounded-xl border border-edge-subtle bg-background/70 p-3">
                    <div className="text-xs text-ink-mute">已返回</div>
                    <div className="mt-1 text-xl font-semibold">{Object.keys(results).length}</div>
                  </div>
                </div>
              </div>
            ),
          },
          balanceList: {
            className: "xl:col-span-12",
            title: "余额列表",
            render: () => (
              <div className="overflow-hidden rounded-2xl border border-edge-subtle bg-surface-raised/60 shadow-sm">
                {isLoading ? (
                  <div className="p-5 text-sm text-ink-mute">加载中...</div>
                ) : (
                  <div className="min-w-[980px]">
                    <div className="grid grid-cols-[1.4fr_1.6fr_1.8fr_1.1fr_140px] gap-4 border-b border-edge-subtle bg-background/60 px-5 py-3 text-xs font-medium text-ink-mute">
                      <div>站点</div>
                      <div>地址</div>
                      <div>模板 / 路径</div>
                      <div>余额</div>
                      <div className="text-right">操作</div>
                    </div>
                    {rows.map((ep) => {
                      const enabled = ep.balanceQuery?.enabled ?? true;
                      return (
                        <div
                          key={ep.id}
                          className="grid grid-cols-[1.4fr_1.6fr_1.8fr_1.1fr_140px] items-center gap-4 border-b border-edge-subtle px-5 py-4 last:border-b-0"
                        >
                          <div className="min-w-0">
                            <div className="truncate text-sm font-semibold">{ep.name}</div>
                            <div className="mt-1 text-xs text-ink-mute">
                              {enabled ? "默认模板可查询" : "模板未启用"}
                            </div>
                          </div>
                          <div className="truncate text-xs text-ink-mute" title={ep.apiUrl}>
                            {ep.apiUrl}
                          </div>
                          <div className="min-w-0 text-xs">
                            <div className="truncate font-medium">
                              {ep.balanceQuery?.templateId || "openai-credit-grants"}
                            </div>
                            <div className="mt-1 truncate text-ink-mute" title={ep.balanceQuery?.path}>
                              {ep.balanceQuery?.path || "/dashboard/billing/credit_grants"}
                            </div>
                            <div className="mt-1 truncate text-ink-mute">
                              提取 {ep.balanceQuery?.extraction?.balancePath || "$.balance"}
                            </div>
                          </div>
                          <BalanceSummary result={results[ep.id]} />
                          <div className="flex justify-end">
                            <Button
                              variant="outline"
                              aria-label={`查询 ${ep.name} 余额`}
                              disabled={!enabled || query.isPending}
                              onClick={() => query.mutate([ep.id])}
                            >
                              查询
                            </Button>
                          </div>
                        </div>
                      );
                    })}
                  </div>
                )}
              </div>
            ),
          },
        }}
      />
    </div>
  );
}
