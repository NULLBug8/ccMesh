import type { UnlistenFn } from "@/services/request";

import { Events, request, subscribe } from "../request";

/** 单条模型映射：入站模型名 from → 出站（上游真实）模型名 to。 */
export interface ModelMapping {
  from: string;
  to: string;
}

export interface BalanceHeader {
  name: string;
  value: string;
}

export interface BalanceExtraction {
  balancePath: string;
  currencyPath: string;
  usedPath: string;
  expiresAtPath: string;
  limits?: BalanceLimitExtraction[];
}

export interface BalanceLimitExtraction {
  label: string;
  balancePath: string;
  usedPath: string;
  expiresAtPath: string;
}

export interface BalanceQueryConfig {
  enabled: boolean;
  templateId: string;
  method: string;
  path: string;
  headers: BalanceHeader[];
  body: string;
  extraction: BalanceExtraction;
}

export interface EndpointBalanceResult {
  success: boolean;
  status: number;
  latencyMs: number;
  balance: string | null;
  currency: string | null;
  used: string | null;
  expiresAt: string | null;
  limits: BalanceLimitResult[];
  message: string;
  raw: string;
}

export interface BalanceLimitResult {
  label: string;
  balance: string | null;
  used: string | null;
  expiresAt: string | null;
}

export interface BalanceProbeTemplateResult {
  templateId: string;
  path: string;
  success: boolean;
  urlReachable: boolean;
  statusCode: number | null;
  latencyMs: number;
  message: string;
  sample: string | null;
  config: BalanceQueryConfig | null;
  balance: string | null;
}

export interface BalanceProbeResult {
  status: "matched" | "sampleAvailable" | "allFailed";
  results: BalanceProbeTemplateResult[];
  matched: BalanceProbeTemplateResult | null;
  usableSamples: BalanceProbeTemplateResult[];
}

export const BALANCE_QUERY_PRESETS: BalanceQueryConfig[] = [
  {
    enabled: true,
    templateId: "openai",
    method: "GET",
    path: "/dashboard/billing/credit_grants",
    headers: [],
    body: "",
    extraction: {
      balancePath: "$.total_available",
      currencyPath: "$.currency",
      usedPath: "$.total_used",
      expiresAtPath: "$.expires_at",
      limits: [],
    },
  },
  {
    enabled: true,
    templateId: "openai-usage",
    method: "GET",
    path: "/dashboard/billing/usage",
    headers: [{ name: "Authorization", value: "Bearer {{apiKey}}" }],
    body: "",
    extraction: {
      balancePath: "",
      currencyPath: "",
      usedPath: "$.total_usage",
      expiresAtPath: "",
      limits: [],
    },
  },
  {
    enabled: true,
    templateId: "apimart",
    method: "GET",
    path: "/v1/user/balance",
    headers: [{ name: "Authorization", value: "Bearer {{apiKey}}" }],
    body: "",
    extraction: {
      balancePath: "$.balance_1d",
      currencyPath: "",
      usedPath: "$.used_1d",
      expiresAtPath: "",
      limits: [
        {
          label: "3小时额度",
          balancePath: "$.balance_3h",
          usedPath: "$.used_3h",
          expiresAtPath: "",
        },
        {
          label: "每日额度",
          balancePath: "$.balance_1d",
          usedPath: "$.used_1d",
          expiresAtPath: "",
        },
      ],
    },
  },
  {
    enabled: true,
    templateId: "apimart-legacy",
    method: "GET",
    path: "/user/balance",
    headers: [{ name: "Authorization", value: "Bearer {{apiKey}}" }],
    body: "",
    extraction: {
      balancePath: "$.balance_1d",
      currencyPath: "",
      usedPath: "$.used_1d",
      expiresAtPath: "",
      limits: [
        {
          label: "3小时额度",
          balancePath: "$.balance_3h",
          usedPath: "$.used_3h",
          expiresAtPath: "",
        },
        {
          label: "每日额度",
          balancePath: "$.balance_1d",
          usedPath: "$.used_1d",
          expiresAtPath: "",
        },
      ],
    },
  },
  {
    enabled: true,
    templateId: "newapi",
    method: "GET",
    path: "/api/user/self",
    headers: [{ name: "Authorization", value: "Bearer {{apiKey}}" }],
    body: "",
    extraction: {
      balancePath: "$.data.quota",
      currencyPath: "$.data.currency",
      usedPath: "$.data.used_quota",
      expiresAtPath: "",
      limits: [],
    },
  },
  {
    enabled: true,
    templateId: "one-api",
    method: "GET",
    path: "/api/user/self",
    headers: [{ name: "Authorization", value: "Bearer {{apiKey}}" }],
    body: "",
    extraction: {
      balancePath: "$.data.quota",
      currencyPath: "",
      usedPath: "$.data.used_quota",
      expiresAtPath: "",
      limits: [],
    },
  },
  {
    enabled: true,
    templateId: "sub2api",
    method: "GET",
    path: "/api/user/self",
    headers: [{ name: "Authorization", value: "Bearer {{apiKey}}" }],
    body: "",
    extraction: {
      balancePath: "$.data.quota",
      currencyPath: "$.data.currency",
      usedPath: "$.data.used_quota",
      expiresAtPath: "$.data.expired_time",
      limits: [],
    },
  },
  {
    enabled: true,
    templateId: "voapi",
    method: "GET",
    path: "/api/user/self",
    headers: [{ name: "Authorization", value: "Bearer {{apiKey}}" }],
    body: "",
    extraction: {
      balancePath: "$.data.quota",
      currencyPath: "",
      usedPath: "$.data.used_quota",
      expiresAtPath: "$.data.expired_time",
      limits: [],
    },
  },
  {
    enabled: true,
    templateId: "newapi-token",
    method: "GET",
    path: "/api/token",
    headers: [{ name: "Authorization", value: "Bearer {{apiKey}}" }],
    body: "",
    extraction: {
      balancePath: "$.data.quota",
      currencyPath: "$.data.currency",
      usedPath: "$.data.used_quota",
      expiresAtPath: "",
      limits: [],
    },
  },
  {
    enabled: true,
    templateId: "one-hub",
    method: "GET",
    path: "/api/user/self",
    headers: [{ name: "Authorization", value: "Bearer {{apiKey}}" }],
    body: "",
    extraction: {
      balancePath: "$.data.quota",
      currencyPath: "$.data.currency",
      usedPath: "$.data.used_quota",
      expiresAtPath: "",
      limits: [],
    },
  },
  {
    enabled: true,
    templateId: "newapi-user-key",
    method: "GET",
    path: "/api/user/self",
    headers: [
      { name: "New-Api-User", value: "{{apiKey}}" },
      { name: "Accept", value: "application/json" },
    ],
    body: "",
    extraction: {
      balancePath: "$.data.quota",
      currencyPath: "$.data.currency",
      usedPath: "$.data.used_quota",
      expiresAtPath: "",
      limits: [],
    },
  },
  {
    enabled: true,
    templateId: "crazyrouter",
    method: "GET",
    path: "/api/user/self",
    headers: [
      { name: "Authorization", value: "Bearer {{apiKey}}" },
      { name: "Accept", value: "application/json" },
    ],
    body: "",
    extraction: {
      balancePath: "$.data.quota / 500000",
      currencyPath: "",
      usedPath: "$.data.used_quota / 500000",
      expiresAtPath: "",
      limits: [],
    },
  },
  {
    enabled: true,
    templateId: "cafecode",
    method: "GET",
    path: "/v1/usage",
    headers: [{ name: "Authorization", value: "Bearer {{apiKey}}" }],
    body: "",
    extraction: {
      balancePath: "$.remaining",
      currencyPath: "$.unit",
      usedPath: "$.usage.today.actual_cost",
      expiresAtPath: "$.subscription.expires_at",
      limits: [
        {
          label: "今日额度",
          balancePath: "$.subscription.daily_limit_usd - $.subscription.daily_usage_usd",
          usedPath: "$.subscription.daily_usage_usd",
          expiresAtPath: "$.subscription.expires_at",
        },
        {
          label: "每周额度",
          balancePath: "$.subscription.weekly_limit_usd - $.subscription.weekly_usage_usd",
          usedPath: "$.subscription.weekly_usage_usd",
          expiresAtPath: "$.subscription.expires_at",
        },
        {
          label: "每月额度",
          balancePath: "$.subscription.monthly_limit_usd - $.subscription.monthly_usage_usd",
          usedPath: "$.subscription.monthly_usage_usd",
          expiresAtPath: "$.subscription.expires_at",
        },
      ],
    },
  },
  {
    enabled: true,
    templateId: "tokenfor-me",
    method: "GET",
    path: "/v1/usage",
    headers: [{ name: "Authorization", value: "Bearer {{apiKey}}" }],
    body: "",
    extraction: {
      balancePath: "$.remaining",
      currencyPath: "$.unit",
      usedPath: "$.usage.today.actual_cost",
      expiresAtPath: "$.subscription.expires_at",
      limits: [
        {
          label: "今日额度",
          balancePath: "$.subscription.daily_limit_usd - $.subscription.daily_usage_usd",
          usedPath: "$.subscription.daily_usage_usd",
          expiresAtPath: "$.subscription.expires_at",
        },
        {
          label: "每周额度",
          balancePath: "$.subscription.weekly_limit_usd - $.subscription.weekly_usage_usd",
          usedPath: "$.subscription.weekly_usage_usd",
          expiresAtPath: "$.subscription.expires_at",
        },
      ],
    },
  },
  {
    enabled: true,
    templateId: "laozhang",
    method: "GET",
    path: "/api/user/self",
    headers: [
      { name: "Authorization", value: "{{apiKey}}" },
      { name: "Accept", value: "application/json" },
      { name: "Content-Type", value: "application/json" },
    ],
    body: "",
    extraction: {
      balancePath: "$.data.quota",
      currencyPath: "",
      usedPath: "$.data.used_quota",
      expiresAtPath: "",
      limits: [],
    },
  },
];

export const DEFAULT_BALANCE_QUERY = BALANCE_QUERY_PRESETS[0];

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
  /** 点亮（对外公布）的模型子集：`models` 的子集。空数组=全部公布（向后兼容旧端点）。 */
  activeModels: string[];
  modelMappings: ModelMapping[];
  balanceQuery: BalanceQueryConfig;
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
  activeModels?: string[];
  modelMappings?: ModelMapping[];
  balanceQuery?: BalanceQueryConfig;
  remark?: string;
}

export type UpdateEndpointRequest = Partial<CreateEndpointRequest>;

/**
 * 点亮过滤后的出站（真实）模型：用于模型映射出站下拉，受点亮模型行为影响。
 * 锁定 model→[model]；否则 activeModels 非空→按 models 顺序取其点亮子集；空→全部 models（兼容旧端点）。
 */
export function litOutboundModels(
  ep: Pick<Endpoint, "model" | "models" | "activeModels">,
): string[] {
  if (ep.model) return [ep.model];
  const models = ep.models ?? [];
  const active = ep.activeModels ?? [];
  return active.length > 0 ? models.filter((m) => active.includes(m)) : models;
}

/**
 * 对外公布的可用模型：基础集（锁定 model 优先；否则点亮子集 activeModels 非空则取它，
 * 空则回退全量 models）并入映射入站名，大小写去重（保留首次出现）。与后端 resolver 一致。
 */
export function advertisedModels(
  ep: Pick<Endpoint, "model" | "models" | "activeModels" | "modelMappings">,
): string[] {
  const base = ep.model
    ? [ep.model]
    : ep.activeModels && ep.activeModels.length > 0
      ? ep.activeModels
      : ep.models ?? [];
  const out: string[] = [];
  const seen = new Set<string>();
  for (const m of [...base, ...(ep.modelMappings ?? []).map((x) => x.from)]) {
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
  test: (id: number, model?: string, mode: "quick" | "deep" = "quick") =>
    request<EndpointTestResult>("test_endpoint", { id, model, mode }),
  queryBalance: (id: number) =>
    request<EndpointBalanceResult>("query_endpoint_balance", { id }),
  testBalanceTemplate: (id: number, balanceQuery: BalanceQueryConfig) =>
    request<EndpointBalanceResult>("test_endpoint_balance_query", { id, balanceQuery }),
  probeBalanceTemplates: (id: number, customPath?: string) =>
    request<BalanceProbeResult>("probe_endpoint_balance_templates", { id, customPath }),
  generateBalanceTemplate: (
    id: number,
    aiModel: string,
    samples: Array<Pick<BalanceProbeTemplateResult, "templateId" | "path" | "statusCode" | "sample">>,
  ) =>
    request<BalanceQueryConfig>("generate_balance_template_with_ai", {
      id,
      aiModel,
      samples,
    }),
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
