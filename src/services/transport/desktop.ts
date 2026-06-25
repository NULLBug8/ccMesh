import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

import type { AppTransport } from "./types";

export const desktopTransport: AppTransport = {
  kind: "desktop",
  async request<T>(command: string, args?: Record<string, unknown>): Promise<T> {
    return invoke<T>(command, args);
  },
  subscribe<T>(event: string, cb: (event: { payload: T }) => void): Promise<() => void> {
    return listen<T>(event, (tauriEvent) => cb({ payload: tauriEvent.payload }));
  },
};
