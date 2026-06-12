import type { UnlistenFn } from "@tauri-apps/api/event";

import { Events, request, subscribe } from "../request";

/** 单条模型映射：入站模型名 from → 出站（上游真实）模型名 to。 */
export interface ModelMapping {
  from: string;
  to: string;
}

export interface Endpoint {
  id: number;
  name: string;
  apiUrl: string;
  apiKey: string;
  authMode: string;
  enabled: boolean;
  useProxy: boolean;
  transformer: string;
  model: string;
  models: string[];
  modelMappings: ModelMapping[];
  remark: string;
  sortOrder: number;
  testStatus: string;
  createdAt: string;
  updatedAt: string;
}

export interface CreateEndpointRequest {
  name: string;
  apiUrl: string;
  apiKey?: string;
  authMode?: string;
  enabled?: boolean;
  useProxy?: boolean;
  transformer?: string;
  model?: string;
  models?: string[];
  modelMappings?: ModelMapping[];
  remark?: string;
}

export type UpdateEndpointRequest = Partial<CreateEndpointRequest>;

/** 出站（真实）模型：锁定 model 优先，否则 models 清单。用于测试连通性 / 映射出站下拉。 */
export function outboundModels(
  ep: Pick<Endpoint, "model" | "models">,
): string[] {
  return ep.model ? [ep.model] : ep.models ?? [];
}

/** 对外公布的可用模型：出站模型并入映射入站名，大小写去重（保留首次出现）。 */
export function advertisedModels(
  ep: Pick<Endpoint, "model" | "models" | "modelMappings">,
): string[] {
  const out: string[] = [];
  const seen = new Set<string>();
  for (const m of [
    ...outboundModels(ep),
    ...(ep.modelMappings ?? []).map((x) => x.from),
  ]) {
    const key = m.trim().toLowerCase();
    if (!key || seen.has(key)) continue;
    seen.add(key);
    out.push(m);
  }
  return out;
}

export interface EndpointTestResult {
  success: boolean;
  status: string;
  latencyMs: number;
  message: string;
}

export const endpointApi = {
  list: () => request<Endpoint[]>("list_endpoints"),
  create: (req: CreateEndpointRequest) =>
    request<Endpoint>("create_endpoint", { req }),
  update: (id: number, req: UpdateEndpointRequest) =>
    request<Endpoint>("update_endpoint", { id, req }),
  remove: (id: number) => request<void>("delete_endpoint", { id }),
  reorder: (orderedIds: number[]) =>
    request<void>("reorder_endpoints", { orderedIds }),
  clone: (id: number) => request<Endpoint>("clone_endpoint", { id }),
  test: (id: number, model?: string) =>
    request<EndpointTestResult>("test_endpoint", { id, model }),
  fetchModels: (
    apiUrl: string,
    apiKey: string,
    transformer: string,
    useProxy?: boolean,
  ) =>
    request<string[]>("fetch_endpoint_models", {
      apiUrl,
      apiKey,
      transformer,
      useProxy,
    }),
  /** 订阅端点配置/测试状态变更事件（启停、编辑、手动测试后触发）。 */
  onChanged: (cb: () => void): Promise<UnlistenFn> =>
    subscribe(Events.endpointsChanged, () => cb()),
};
