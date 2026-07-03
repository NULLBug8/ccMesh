import type { PageLayoutDefinition } from "@/components/business/page-layout/pageLayoutTypes";

export const logsLayoutDefinition: PageLayoutDefinition = {
  defaultMode: "split",
  sections: [
    { id: "header", title: "标题" },
    { id: "toolbar", title: "日志工具栏" },
    { id: "requests", title: "最近请求" },
    { id: "stream", title: "日志流" },
  ],
};
