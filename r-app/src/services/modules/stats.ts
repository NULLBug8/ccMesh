import type { UnlistenFn } from "@tauri-apps/api/event";

import { Events, request, subscribe } from "../request";

export interface EndpointStat {
  endpointName: string;
  requests: number;
  errors: number;
  inputTokens: number;
  outputTokens: number;
}

export interface PeriodStats {
  requests: number;
  errors: number;
  inputTokens: number;
  outputTokens: number;
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
}

export const statsApi = {
  getStats: () => request<StatsOverview>("get_stats"),
  getArchiveMonths: () => request<string[]>("get_archive_months"),
  getMonthlyArchive: (month: string) =>
    request<DailyStat[]>("get_monthly_archive", { month }),
  deleteMonthlyStats: (month: string) =>
    request<number>("delete_monthly_stats", { month }),
  /** 订阅统计更新事件（零延迟刷新）。 */
  onUpdated: (cb: () => void): Promise<UnlistenFn> =>
    subscribe(Events.statsUpdated, () => cb()),
};
