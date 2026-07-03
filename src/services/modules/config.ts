import { request } from "../request";
import type { RulesConfig } from "./rules";

export interface AppConfig {
  port: number;
  logLevel: string;
  language: string;
  theme: string;
  themeAuto: boolean;
  autoLightStart: string;
  autoDarkStart: string;
  autoRun: boolean;
  modelsCacheTtl: number;
  proxyUrl: string;
  proxyEnabled: boolean;
  openaiUa: string;
  claudeCliUa: string;
  rules: RulesConfig;
}

export interface ProxyTestResult {
  success: boolean;
  status: string;
  latencyMs: number;
  message: string;
}

export const configApi = {
  getConfig: () => request<AppConfig>("get_config"),
  /** 部分更新：键为扁平配置键（如 port / theme / proxyUrl），值为字符串。 */
  setConfig: (patch: Record<string, string>) => request<AppConfig>("set_config", { patch }),
  /** 使用给定代理地址做一次连通性测试。 */
  testProxy: (url: string) => request<ProxyTestResult>("test_proxy", { url }),
};
