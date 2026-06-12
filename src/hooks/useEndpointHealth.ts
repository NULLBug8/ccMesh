import { useEffect } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";

import { endpointApi } from "@/services/modules/endpoint";
import { healthApi } from "@/services/modules/health";

/** 端点状态相关查询：健康事件/端点变更事件到达时全部失效，保证跨页面同步。 */
const RELATED_KEYS = [["endpoints"], ["health"], ["endpoint-health"]];

/**
 * 订阅 `endpoint-health-changed`（熔断状态转换）与 `endpoints-changed`（启停/编辑/手动测试），
 * 统一失效端点相关查询。页级挂载一次即可，替代各组件内联的重复订阅。
 */
export function useEndpointHealthEvents() {
  const qc = useQueryClient();
  useEffect(() => {
    const unlistens: Array<() => void> = [];
    const invalidateAll = () =>
      RELATED_KEYS.forEach((queryKey) => qc.invalidateQueries({ queryKey }));
    healthApi.onHealthChanged(invalidateAll).then((un) => unlistens.push(un));
    endpointApi.onChanged(invalidateAll).then((un) => unlistens.push(un));
    return () => unlistens.forEach((un) => un());
  }, [qc]);
}

/** 端点实时健康/熔断态查询（多组件共享同一 queryKey，React Query 自动去重）。 */
export function useEndpointHealth() {
  return useQuery({
    queryKey: ["endpoint-health"],
    queryFn: healthApi.getEndpointHealth,
  });
}
