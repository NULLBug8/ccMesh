import { request } from "../request";

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
  remark?: string;
}

export type UpdateEndpointRequest = Partial<CreateEndpointRequest>;

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
};
