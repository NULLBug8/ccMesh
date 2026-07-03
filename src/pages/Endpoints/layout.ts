import type { PageLayoutDefinition } from "@/components/business/page-layout/pageLayoutTypes";

export const endpointsLayoutDefinition: PageLayoutDefinition = {
  defaultMode: "split",
  sections: [
    { id: "header", title: "标题与筛选" },
    { id: "endpoints", title: "端点列表" },
    { id: "models", title: "可用模型" },
  ],
};
