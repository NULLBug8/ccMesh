import type { PageLayoutDefinition } from "@/components/business/page-layout/pageLayoutTypes";

export const settingsLayoutDefinition: PageLayoutDefinition = {
  defaultMode: "stack",
  sections: [
    { id: "header", title: "标题" },
    { id: "general", title: "常规设置" },
    { id: "startup", title: "启动行为" },
    { id: "proxy", title: "代理设置" },
    { id: "advanced", title: "系统与高级" },
    { id: "update", title: "更新" },
    { id: "tokens", title: "Token 统计" },
  ],
};
