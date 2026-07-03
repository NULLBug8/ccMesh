import type { UnlistenFn } from "@/services/request";

import { Events, request, subscribe } from "../request";

export interface EndpointStat {
  endpointName: string;
  requests: number;
  errors: number;
  inputTokens: number;
  outputTokens: number;
  cacheCreationTokens: number;
  cacheReadTokens: number;
}

export interface PeriodStats {
  requests: number;
  errors: number;
  inputTokens: number;
  outputTokens: number;
  cacheCreationTokens: number;
  cacheReadTokens: number;
  endpoints: EndpointStat[];
}

export interface TrendCompare {
  requestsPct: number;
  inputTokensPct: number;
  outputTokensPct: number;
}

export interface StatsOverview {
  today: PeriodStats;
  yesterday: PeriodStats;
  thisWeek: PeriodStats;
  thisMonth: PeriodStats;
  trend: TrendCompare;
}

export interface DailyStat {
  endpointName: string;
  date: string;
  requests: number;
  errors: number;
  inputTokens: number;
  outputTokens: number;
  cacheCreationTokens: number;
  cacheReadTokens: number;
}

export interface RequestTraceHeader {
  key: string;
  value: string;
}

export interface RequestTraceStage {
  method: string | null;
  url: string | null;
  statusCode: number | null;
  headers: RequestTraceHeader[];
  body: string | null;
}

export interface RequestTrace {
  receivedRequest: RequestTraceStage;
  forwardRequest: RequestTraceStage;
  receivedForwardedRequest: RequestTraceStage;
  responseRequest: RequestTraceStage;
}

export interface RequestLog {
  id: number;
  ts: number;
  endpointName: string;
  inboundFormat: string;
  /** 端点 transformer 快照（claude/openai/codex 等）。旧行/未记录为 null，前端回退 inboundFormat。 */
  transformer: string | null;
  upstreamUrl: string;
  inboundPath: string;
  upstreamPath: string;
  statusCode: number | null;
  isError: boolean;
  inputTokens: number;
  outputTokens: number;
  cacheCreationTokens: number;
  cacheReadTokens: number;
  model: string | null;
  durationMs: number | null;
  firstByteMs: number | null;
  actualModel: string | null;
  errorBody: string | null;
  trace: RequestTrace | null;
}

export interface RequestLogPage {
  items: RequestLog[];
  total: number;
}

export interface StatsHistoryPage {
  items: DailyStat[];
  total: number;
}

export interface RequestLogQuery {
  startMs?: number;
  endMs?: number;
  endpoint?: string;
  page: number;
  pageSize: number;
}

export const statsApi = {
  getStats: () => request<StatsOverview>("get_stats"),
  getRequestLogs: (q: RequestLogQuery) =>
    request<RequestLogPage>("get_request_logs", {
      startMs: q.startMs,
      endMs: q.endMs,
      endpoint: q.endpoint,
      page: q.page,
      pageSize: q.pageSize,
    }),
  getStatsHistory: (page: number, pageSize: number) =>
    request<StatsHistoryPage>("get_stats_history", { page, pageSize }),
  deleteDailyStat: (endpointName: string, date: string) =>
    request<number>("delete_daily_stat", { endpointName, date }),
  deleteStatsByDate: (date: string) =>
    request<number>("delete_stats_by_date", { date }),
  onUpdated: (cb: () => void): Promise<UnlistenFn> =>
    subscribe(Events.statsUpdated, () => cb()),
  onRequestLogged: (cb: (log: RequestLog) => void): Promise<UnlistenFn> =>
    subscribe<RequestLog>(Events.requestLogged, (event) => cb(event.payload)),
};
