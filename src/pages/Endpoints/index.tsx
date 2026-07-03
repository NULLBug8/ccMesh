import { useMemo, useState } from "react";

import { PageLayoutEditor } from "@/components/business/page-layout/PageLayoutEditor";
import { useEndpoints } from "@/hooks/useEndpoints";
import { useEndpointHealthEvents } from "@/hooks/useEndpointHealth";
import type { Endpoint } from "@/services/modules/endpoint";
import { useFilterStore, useLayoutStore } from "@/stores";
import { DnDList } from "./_components/DnDList";
import { EndpointForm } from "./_components/EndpointForm";
import { FilterBar } from "./_components/FilterBar";
import { ModelList } from "./_components/ModelList";
import { endpointsLayoutDefinition } from "./layout";

export function Endpoints() {
  const { data: endpoints, isLoading } = useEndpoints();
  const search = useFilterStore((state) => state.search);
  const enabledOnly = useFilterStore((state) => state.enabledOnly);
  const transformer = useFilterStore((state) => state.transformer);
  const isActive = useFilterStore((state) => state.isActive);
  const view = useLayoutStore((state) => state.endpointView);

  useEndpointHealthEvents();

  const [formOpen, setFormOpen] = useState(false);
  const [editing, setEditing] = useState<Endpoint | null>(null);

  const filtered = useMemo(
    () =>
      (endpoints ?? []).filter(
        (endpoint) =>
          (!enabledOnly || endpoint.enabled) &&
          (transformer === "all" || endpoint.transformer === transformer) &&
          (search === "" ||
            endpoint.name.toLowerCase().includes(search.toLowerCase()) ||
            endpoint.apiUrl.toLowerCase().includes(search.toLowerCase())),
      ),
    [enabledOnly, endpoints, search, transformer],
  );

  const dragEnabled = !isActive();
  const total = endpoints?.length ?? 0;
  const enabledCount = (endpoints ?? []).filter((endpoint) => endpoint.enabled).length;
  const availableCount = (endpoints ?? []).filter((endpoint) => endpoint.testStatus === "available")
    .length;
  const unavailableCount = (endpoints ?? []).filter(
    (endpoint) => endpoint.testStatus === "unavailable",
  ).length;

  const openCreate = () => {
    setEditing(null);
    setFormOpen(true);
  };

  const openEdit = (endpoint: Endpoint) => {
    setEditing(endpoint);
    setFormOpen(true);
  };

  return (
    <div className="flex h-full w-full min-w-0 flex-col gap-4">
      <PageLayoutEditor view="endpoints" definition={endpointsLayoutDefinition} />

      <div
        data-testid="endpoint-workbench"
        className="grid min-h-0 flex-1 gap-4 xl:grid-cols-[minmax(0,1fr)_360px]"
      >
        <section
          data-testid="endpoint-workbench-main"
          className="flex min-w-0 flex-col overflow-hidden rounded-2xl border border-edge bg-surface/70 shadow-sm"
        >
          <div className="border-b border-edge-subtle bg-surface/90 p-4">
            <div className="mb-4 flex flex-col gap-2 lg:flex-row lg:items-end lg:justify-between">
              <div>
                <div className="text-xs font-medium uppercase tracking-[0.18em] text-primary">
                  Endpoint Ops
                </div>
                <h1 className="mt-1 text-2xl font-semibold tracking-tight">端点管理</h1>
                <p className="mt-1 text-sm text-ink-mute">
                  管理路由入口、模型映射、余额探测和单端点连通性。
                </p>
              </div>
              <div className="grid grid-cols-2 gap-2 text-xs sm:grid-cols-4">
                <Metric label="全部" value={total} />
                <Metric label="启用" value={enabledCount} tone="good" />
                <Metric label="可用" value={availableCount} tone="good" />
                <Metric label="不可用" value={unavailableCount} tone="bad" />
              </div>
            </div>
            <div className="xl:col-span-12">
              <div>
                <FilterBar onCreate={openCreate} />
              </div>
            </div>
          </div>

          <div className="min-h-0 flex-1 overflow-auto p-4">
            {isLoading ? (
              <p className="text-sm text-ink-mute">加载中...</p>
            ) : filtered.length === 0 ? (
              <div className="flex min-h-60 flex-col items-center justify-center rounded-xl border border-dashed border-edge bg-surface/50 p-8 text-center">
                <p className="text-sm font-medium">暂无匹配端点</p>
                <p className="mt-1 text-sm text-ink-mute">
                  调整筛选条件，或点击“新建端点”添加一个上游站点。
                </p>
              </div>
            ) : (
              <DnDList
                endpoints={filtered}
                draggable={dragEnabled}
                view={view}
                onEdit={openEdit}
              />
            )}
          </div>
        </section>

        <aside
          data-testid="endpoint-workbench-inspector"
          className="flex min-h-0 min-w-0 flex-col gap-4 xl:sticky xl:top-4 xl:max-h-[calc(100dvh-7rem)]"
        >
          <section className="rounded-2xl border border-edge bg-surface/80 p-4 shadow-sm">
            <div className="text-xs font-medium uppercase tracking-[0.16em] text-ink-mute">
              Inspector
            </div>
            <h2 className="mt-1 text-base font-semibold">运行态概览</h2>
            <div className="mt-4 grid grid-cols-2 gap-2">
              <Metric label="筛选结果" value={filtered.length} />
              <Metric label="排序" value={dragEnabled ? "可拖动" : "筛选中"} />
            </div>
            <p className="mt-3 rounded-lg border border-edge-subtle bg-background/60 px-3 py-2 text-xs leading-5 text-ink-mute">
              单端点测试仍在卡片按钮中选择本次测试模型；这里保留全局观察信息，避免列表区域挤压。
            </p>
          </section>
          <div className="min-h-0 flex-1 overflow-hidden rounded-2xl border border-edge bg-surface/80 p-4 shadow-sm">
            <ModelList />
          </div>
        </aside>
      </div>

      <EndpointForm open={formOpen} onOpenChange={setFormOpen} editing={editing} />
    </div>
  );
}

function Metric({
  label,
  value,
  tone,
}: {
  label: string;
  value: number | string;
  tone?: "good" | "bad";
}) {
  const toneClass =
    tone === "good"
      ? "text-success"
      : tone === "bad"
        ? "text-destructive"
        : "text-foreground";
  return (
    <div className="rounded-xl border border-edge-subtle bg-background/60 px-3 py-2">
      <div className="text-[11px] text-ink-mute">{label}</div>
      <div className={`mt-1 text-lg font-semibold tabular-nums ${toneClass}`}>{value}</div>
    </div>
  );
}
