import { useMemo, useState } from "react";

import { PageLayoutEditor } from "@/components/business/page-layout/PageLayoutEditor";
import { PageSectionHost } from "@/components/business/page-layout/PageSectionHost";
import { useEndpoints } from "@/hooks/useEndpoints";
import { useEndpointHealthEvents } from "@/hooks/useEndpointHealth";
import type { Endpoint } from "@/services/modules/endpoint";
import { resolveViewLayout, useFilterStore, useLayoutStore, usePageLayoutStore } from "@/stores";
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
  const savedLayout = usePageLayoutStore((state) => state.getLayout("endpoints"));
  const layout = resolveViewLayout(endpointsLayoutDefinition, savedLayout);

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

  const openCreate = () => {
    setEditing(null);
    setFormOpen(true);
  };

  const openEdit = (endpoint: Endpoint) => {
    setEditing(endpoint);
    setFormOpen(true);
  };

  return (
    <div className="flex h-full w-full min-w-0 flex-col gap-5">
      <PageLayoutEditor view="endpoints" definition={endpointsLayoutDefinition} />
      <PageSectionHost
        layout={layout}
        registry={{
          header: {
            title: "标题与筛选",
            className: "xl:col-span-12",
            render: () => (
              <div className="flex shrink-0 flex-col gap-5">
                <h1 className="text-2xl font-light tracking-tight">端点管理</h1>
                <FilterBar onCreate={openCreate} />
              </div>
            ),
          },
          endpoints: {
            title: "端点列表",
            className: "min-h-0 xl:col-span-7",
            render: () => (
              <div className="min-h-0 overflow-auto pr-1">
                {isLoading ? (
                  <p className="text-sm text-ink-mute">加载中...</p>
                ) : filtered.length === 0 ? (
                  <p className="text-sm text-ink-mute">暂无端点，点击“新建端点”添加。</p>
                ) : (
                  <DnDList
                    endpoints={filtered}
                    draggable={dragEnabled}
                    view={view}
                    onEdit={openEdit}
                  />
                )}
              </div>
            ),
          },
          models: {
            title: "可用模型",
            className: "min-h-0 xl:col-span-5",
            render: () => (
              <div className="flex min-h-0 flex-col">
                <ModelList />
              </div>
            ),
          },
        }}
      />

      <EndpointForm open={formOpen} onOpenChange={setFormOpen} editing={editing} />
    </div>
  );
}
