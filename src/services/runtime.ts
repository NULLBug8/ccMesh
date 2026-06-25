import { desktopTransport } from "@/services/transport/desktop";
import { webTransport } from "@/services/transport/web";
import type { AppTransport } from "@/services/transport/types";

declare global {
  interface Window {
    __CCMESH_WEB__?: boolean;
    __TAURI_INTERNALS__?: unknown;
  }
}

function detectTauriRuntime(): boolean {
  return typeof window !== "undefined" && typeof window.__TAURI_INTERNALS__ !== "undefined";
}

export function isWebRuntime(): boolean {
  if (typeof window === "undefined") return false;
  if (typeof window.__CCMESH_WEB__ === "boolean") return window.__CCMESH_WEB__;
  return !detectTauriRuntime();
}

export function createTransport(): AppTransport {
  return isWebRuntime() ? webTransport : desktopTransport;
}
