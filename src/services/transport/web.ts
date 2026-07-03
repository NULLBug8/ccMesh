import type { AppTransport } from "./types";

const WEB_API_BASE = "/__admin/api";
const WEB_EVENTS_URL = "/__admin/events";

type Listener = (payload: unknown) => void;

class WebEventHub {
  private listeners = new Map<string, Set<Listener>>();
  private source: EventSource | null = null;

  subscribe<T>(event: string, cb: (payload: T) => void): () => void {
    if (
      !this.source &&
      typeof window !== "undefined" &&
      typeof EventSource !== "undefined"
    ) {
      this.source = new EventSource(WEB_EVENTS_URL);
      this.source.onmessage = (message) => {
        try {
          const parsed = JSON.parse(message.data) as { event: string; payload: unknown };
          const subscribers = this.listeners.get(parsed.event);
          subscribers?.forEach((listener) => listener(parsed.payload));
        } catch {
          // Ignore malformed server messages so one bad event does not break the stream.
        }
      };
    }

    const set = this.listeners.get(event) ?? new Set<Listener>();
    const wrapped: Listener = (payload) => cb(payload as T);
    set.add(wrapped);
    this.listeners.set(event, set);

    return () => {
      const current = this.listeners.get(event);
      current?.delete(wrapped);
      if (current && current.size === 0) {
        this.listeners.delete(event);
      }
      if (this.listeners.size === 0) {
        this.source?.close();
        this.source = null;
      }
    };
  }
}

const eventHub = new WebEventHub();

export const webTransport: AppTransport = {
  kind: "web",
  async request<T>(command: string, args?: Record<string, unknown>): Promise<T> {
    const response = await fetch(`${WEB_API_BASE}/invoke`, {
      method: "POST",
      headers: {
        "content-type": "application/json",
      },
      body: JSON.stringify({
        command,
        args: args ?? {},
      }),
    });

    const payload = (await response.json().catch(() => null)) as
      | { data?: T; error?: string }
      | null;

    if (!response.ok) {
      throw new Error(payload?.error ?? `HTTP ${response.status}`);
    }

    if (payload && "error" in payload && payload.error) {
      throw new Error(payload.error);
    }

    return (payload?.data ?? null) as T;
  },
  async subscribe<T>(event: string, cb: (event: { payload: T }) => void): Promise<() => void> {
    return eventHub.subscribe<T>(event, (payload) => cb({ payload }));
  },
};
