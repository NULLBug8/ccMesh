import type { PageLayoutDefinition } from "@/components/business/page-layout/pageLayoutTypes";

export const dashboardLayoutDefinition: PageLayoutDefinition = {
  defaultMode: "stack",
  sections: [
    { id: "hero", title: "页面标题" },
    { id: "service", title: "服务状态" },
    { id: "stats", title: "今日指标" },
    { id: "requests", title: "实时请求" },
  ],
};
