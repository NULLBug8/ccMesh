import type { PageLayoutDefinition } from "@/components/business/page-layout/pageLayoutTypes";

export const balancesLayoutDefinition: PageLayoutDefinition = {
  defaultMode: "stack",
  sections: [
    { id: "header", title: "标题" },
    { id: "balanceList", title: "余额列表" },
  ],
};
