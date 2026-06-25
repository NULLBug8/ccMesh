import { isWebRuntime } from "@/services/runtime";

import { request } from "../request";

export interface ImportSummary {
  endpointsAdded: number;
  endpointsUpdated: number;
  endpointsSkipped: number;
  credentials: number;
  configKeys: number;
}

export type ImportStrategy = "overwrite" | "skip";

const JSON_FILTER = [{ name: "ccmesh 配置", extensions: ["json"] }];

function defaultName(): string {
  const d = new Date();
  const p = (n: number) => String(n).padStart(2, "0");
  return `ccmesh-config-${d.getFullYear()}${p(d.getMonth() + 1)}${p(d.getDate())}.json`;
}

function ensureDesktopCapability(feature: string) {
  if (isWebRuntime()) {
    throw new Error(`${feature} 仅桌面端支持`);
  }
}

export const backupApi = {
  exportConfig: async (): Promise<string | null> => {
    ensureDesktopCapability("本地导出");
    const { save } = await import("@tauri-apps/plugin-dialog");
    const path = await save({ defaultPath: defaultName(), filters: JSON_FILTER });
    if (!path) return null;
    await request<void>("export_config", { path });
    return path;
  },
  importConfig: async (strategy: ImportStrategy): Promise<ImportSummary | null> => {
    ensureDesktopCapability("本地导入");
    const { open } = await import("@tauri-apps/plugin-dialog");
    const selected = await open({
      multiple: false,
      directory: false,
      filters: JSON_FILTER,
    });
    if (!selected || typeof selected !== "string") return null;
    return request<ImportSummary>("import_config", { path: selected, strategy });
  },
};
