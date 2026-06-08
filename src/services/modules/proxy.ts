import type { UnlistenFn } from "@tauri-apps/api/event";

import { Events, request, subscribe } from "../request";

export interface ProxyStatus {
  running: boolean;
  port: number;
  currentEndpoint: string | null;
  enabledEndpointCount: number;
}

export const proxyApi = {
  start: () => request<ProxyStatus>("start_proxy"),
  stop: () => request<ProxyStatus>("stop_proxy"),
  status: () => request<ProxyStatus>("get_proxy_status"),
  switchEndpoint: (name: string) =>
    request<ProxyStatus>("switch_endpoint", { name }),
  /** 订阅代理状态变更事件，返回取消订阅函数。 */
  onStatusChanged: (cb: (status: ProxyStatus) => void): Promise<UnlistenFn> =>
    subscribe<ProxyStatus>(Events.proxyStatusChanged, (e) => cb(e.payload)),
};
