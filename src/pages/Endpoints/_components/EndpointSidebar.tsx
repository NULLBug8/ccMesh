import { useMemo } from "react";

import { useEndpoints } from "@/hooks/useEndpoints";
import { advertisedModels } from "@/services/modules/endpoint";

import { ModelList } from "./ModelList";

/**
 * 端点管理右侧侧栏：端点统计（总数/启用/禁用 + 可用模型数）+ 可用模型列表。
 * 作为 flex 子项参与中右布局，宽度由调用方通过 flex 比例控制。
 */
export function EndpointSidebar() {
  const { data: endpoints } = useEndpoints();

  const stats = useMemo(() => {
    const all = endpoints ?? [];
    const enabled = all.filter((e) => e.enabled);
    const modelSet = new Set<string>();
    enabled.forEach((e) => advertisedModels(e).forEach((m) => modelSet.add(m)));
    return {
      total: all.length,
      enabled: enabled.length,
      disabled: all.length - enabled.length,
      modelCount: modelSet.size,
    };
  }, [endpoints]);

  return (
    <aside className="flex min-h-0 min-w-0 flex-[1] flex-col gap-4">
      {/* 端点统计 */}
      <section className="shrink-0 rounded-lg border border-edge bg-surface p-4">
        <h2 className="mb-3 text-sm font-medium text-ink-secondary">端点统计</h2>
        <dl className="grid grid-cols-3 gap-2 text-center">
          <div className="flex flex-col gap-0.5">
            <dt className="text-xs text-ink-mute">总数</dt>
            <dd className="text-lg font-medium tabular text-ink-primary">{stats.total}</dd>
          </div>
          <div className="flex flex-col gap-0.5">
            <dt className="text-xs text-ink-mute">启用</dt>
            <dd className="text-lg font-medium tabular text-success">{stats.enabled}</dd>
          </div>
          <div className="flex flex-col gap-0.5">
            <dt className="text-xs text-ink-mute">禁用</dt>
            <dd className="text-lg font-medium tabular text-ink-disabled">{stats.disabled}</dd>
          </div>
        </dl>
        <div className="mt-3 flex items-center justify-between border-t border-edge-subtle pt-3 text-xs">
          <span className="text-ink-mute">可用模型</span>
          <span className="tabular font-medium text-ink-secondary">{stats.modelCount}</span>
        </div>
      </section>

      {/* 可用模型（按端点）：标题固定、内容内部滚动 */}
      <div className="flex min-h-0 flex-1 flex-col">
        <ModelList />
      </div>
    </aside>
  );
}
