import type { UnlistenFn } from "@tauri-apps/api/event";

import { Events, request, subscribe } from "../request";

export interface LogField {
  key: string;
  value: string;
}

export interface LogLine {
  time: string;
  level: string;
  target: string;
  message: string;
  fields: LogField[];
}

export const logsApi = {
  recent: () => request<LogLine[]>("get_recent_logs"),
  setLevel: (level: string) => request<void>("set_log_level", { level }),
  onLine: (cb: (line: LogLine) => void): Promise<UnlistenFn> =>
    subscribe<LogLine>(Events.logLine, (e) => cb(e.payload)),
};
