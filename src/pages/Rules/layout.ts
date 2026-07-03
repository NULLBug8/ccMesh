import type { PageLayoutDefinition } from "@/components/business/page-layout/pageLayoutTypes";

export const rulesLayoutDefinition: PageLayoutDefinition = {
  defaultMode: "stack",
  sections: [
    { id: "header", title: "标题" },
    { id: "routing", title: "路由规则" },
    { id: "circuitBreaker", title: "熔断规则" },
    { id: "degradation", title: "降级规则" },
  ],
};
