import { createTransport } from "./runtime";

const transport = createTransport();

export type UnlistenFn = () => void;
export type EventCallback<T> = (event: { payload: T }) => void;

/** 调用 Web 后端命令。命令名使用 snake_case，参数键使用 camelCase。 */
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

/** 订阅 Web 后端 SSE 事件。 */
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
} as const;