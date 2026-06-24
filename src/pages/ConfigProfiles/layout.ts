import type { PageLayoutDefinition } from "@/components/business/page-layout/pageLayoutTypes";

export const configProfilesLayoutDefinition: PageLayoutDefinition = {
  defaultMode: "stack",
  sections: [
    { id: "header", title: "标题与工具栏" },
    { id: "workspace", title: "配置工作区" },
  ],
};
