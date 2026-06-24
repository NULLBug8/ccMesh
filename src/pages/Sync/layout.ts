import type { PageLayoutDefinition } from "@/components/business/page-layout/pageLayoutTypes";

export const syncLayoutDefinition: PageLayoutDefinition = {
  defaultMode: "stack",
  sections: [
    { id: "header", title: "标题" },
    { id: "webdav", title: "WebDAV 配置" },
    { id: "backup", title: "云端备份" },
    { id: "local", title: "本地导入导出" },
  ],
};
