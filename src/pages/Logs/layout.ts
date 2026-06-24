import type { PageLayoutDefinition } from "@/components/business/page-layout/pageLayoutTypes";

export const logsLayoutDefinition: PageLayoutDefinition = {
  defaultMode: "stack",
  sections: [
    { id: "header", title: "标题" },
    { id: "toolbar", title: "日志工具栏" },
    { id: "stream", title: "日志流" },
  ],
};
