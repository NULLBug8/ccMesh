import type { PageLayoutDefinition } from "@/components/business/page-layout/pageLayoutTypes";

export const logsLayoutDefinition: PageLayoutDefinition = {
  defaultMode: "stack",
  sections: [
    { id: "header", title: "标题" },
    { id: "toolbar", title: "日志工具栏" },
    { id: "requests", title: "最近请求" },
    { id: "trace", title: "请求四阶段详情" },
    { id: "stream", title: "日志流" },
  ],
};
