import { request } from "../request";

export interface HealthInfo {
  status: string;
  deviceId: string;
  enabledEndpoints: number;
}

/** 健康检查（脱敏端点列表等在阶段 4/8 补全）。 */
export const healthApi = {
  getHealth: () => request<HealthInfo>("get_health"),
};
