import type { PageLayoutDefinition } from "@/components/business/page-layout/pageLayoutTypes";

export const statisticsLayoutDefinition: PageLayoutDefinition = {
  defaultMode: "stack",
  sections: [
    { id: "header", title: "标题" },
    { id: "endpoint", title: "端点统计" },
  ],
};
