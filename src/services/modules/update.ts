import type { UnlistenFn } from "@tauri-apps/api/event";

import { isWebRuntime } from "@/services/runtime";

import { Events, request, subscribe } from "../request";

export interface UpdateInfo {
  available: boolean;
  version: string;
  currentVersion: string;
  notes: string;
}

export interface UpdateSettings {
  autoCheck: boolean;
  checkInterval: number;
  skippedVersion: string;
}

export interface DownloadProgress {
  downloaded: number;
  total: number | null;
}

export const GITHUB_RELEASES_URL = "https://github.com/VkRainB/ccMesh/releases";

export async function openReleases() {
  if (isWebRuntime()) {
    window.open(GITHUB_RELEASES_URL, "_blank", "noopener,noreferrer");
    return;
  }

  const { openUrl } = await import("@tauri-apps/plugin-opener");
  await openUrl(GITHUB_RELEASES_URL);
}

export async function getAppVersion(): Promise<string> {
  if (isWebRuntime()) {
    const info = await request<UpdateInfo>("check_for_updates").catch(() => null);
    return info?.currentVersion ?? "web";
  }

  const { getVersion } = await import("@tauri-apps/api/app");
  return getVersion();
}

export const updateApi = {
  check: () => request<UpdateInfo>("check_for_updates"),
  downloadAndInstall: () => request<void>("download_and_install"),
  getSettings: () => request<UpdateSettings>("get_update_settings"),
  setSettings: (autoCheck: boolean, checkInterval: number) =>
    request<void>("set_update_settings", { autoCheck, checkInterval }),
  skipVersion: (version: string) => request<void>("skip_version", { version }),
  onProgress: (cb: (p: DownloadProgress) => void): Promise<UnlistenFn> =>
    subscribe<DownloadProgress>(Events.updateProgress, (e) => cb(e.payload)),
};
