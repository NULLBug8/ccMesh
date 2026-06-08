import { request } from "../request";
import type { WebDavConfig } from "./config";

export interface WebDavTestResult {
  success: boolean;
  message: string;
}

export interface BackupFile {
  filename: string;
  size: number;
  modTime: string;
}

export const webdavApi = {
  test: (config: WebDavConfig) =>
    request<WebDavTestResult>("test_webdav", { config }),
  backup: () => request<string>("webdav_backup"),
  restore: (filename: string, strategy?: string) =>
    request<void>("webdav_restore", { filename, strategy }),
  listBackups: () => request<BackupFile[]>("webdav_list_backups"),
  deleteBackup: (filename: string) =>
    request<void>("webdav_delete_backup", { filename }),
};
