import type { EventCallback, UnlistenFn } from "@tauri-apps/api/event";

import { createTransport } from "./runtime";

const transport = createTransport();

/**
 * 统一调用后端命令。约定：命令名 snake_case，参数键 camelCase。
 * 桌面端走 Tauri invoke，Web 端走管理接口。
 */
export async function request<T>(
  command: string,
  args?: Record<string, unknown>,
): Promise<T> {
  try {
    return await transport.request<T>(command, args);
  } catch (error) {
    const message =
      typeof error === "string"
        ? error
        : error instanceof Error
          ? error.message
          : JSON.stringify(error);
    throw new Error(message);
  }
}

/**
 * 统一订阅后端事件。桌面端走 Tauri listen，Web 端走 SSE。
 */
export async function subscribe<T>(
  event: string,
  handler: EventCallback<T>,
): Promise<UnlistenFn> {
  const unlisten = await transport.subscribe<T>(event, (wrapped) =>
    handler(wrapped as Parameters<EventCallback<T>>[0]),
  );
  return unlisten as UnlistenFn;
}

export const Events = {
  statsUpdated: "stats-updated",
  requestLogged: "request-logged",
  proxyStatusChanged: "proxy-status-changed",
  endpointHealthChanged: "endpoint-health-changed",
  endpointsChanged: "endpoints-changed",
  logLine: "log-line",
  updateProgress: "update-progress",
} as const;
