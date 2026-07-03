import type { PageLayoutDefinition } from "@/components/business/page-layout/pageLayoutTypes";

export const settingsLayoutDefinition: PageLayoutDefinition = {
  defaultMode: "stack",
  sections: [
    { id: "header", title: "标题" },
    { id: "general", title: "常规设置" },
    { id: "proxy", title: "代理设置" },
    { id: "advanced", title: "高级设置" },
    { id: "tokens", title: "Token 统计" },
  ],
};
