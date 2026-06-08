import { request } from "../request";

export type UsageAppFilter = "all" | "claude" | "codex";

export interface UsageSummary {
  totalRequests: number;
  totalInputTokens: number;
  totalOutputTokens: number;
  totalCacheCreationTokens: number;
  totalCacheReadTokens: number;
}

/** 按天 × 来源 × 模型聚合（多维合并表：前端按 date 行合并展示）。 */
export interface DayModelUsage {
  date: string;
  appType: string;
  model: string;
  requests: number;
  inputTokens: number;
  outputTokens: number;
  cacheCreationTokens: number;
  cacheReadTokens: number;
}

export interface UsageSyncResult {
  imported: number;
  filesScanned: number;
  errors: number;
}

interface UsageFilter {
  start?: string;
  end?: string;
  appType?: string;
}

export const usageApi = {
  /** 触发本机用量增量同步。 */
  sync: () => request<UsageSyncResult>("sync_session_usage"),
  getSummary: (f: UsageFilter = {}) =>
    request<UsageSummary>("get_usage_summary", {
      start: f.start,
      end: f.end,
      appType: f.appType,
    }),
  /** 按天 × 来源 × 模型聚合（date 倒序、组内 token 降序）。 */
  getByDayModel: (f: UsageFilter = {}) =>
    request<DayModelUsage[]>("get_usage_by_day_model", {
      start: f.start,
      end: f.end,
      appType: f.appType,
    }),
};
