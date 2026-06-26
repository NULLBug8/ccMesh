import { useState } from "react";

import { PageLayoutEditor } from "@/components/business/page-layout/PageLayoutEditor";
import { PageSectionHost } from "@/components/business/page-layout/PageSectionHost";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { resolveViewLayout, usePageLayoutStore } from "@/stores";
import { EndpointStatsPanel } from "./_components/EndpointStatsPanel";
import { UsagePanel } from "./_components/UsagePanel";
import { statisticsLayoutDefinition } from "./layout";

const TOP_TABS = [
  { key: "endpoint", label: "端点统计" },
  { key: "usage", label: "用量统计" },
] as const;

type TopKey = (typeof TOP_TABS)[number]["key"];

export function Statistics() {
  const [tab, setTab] = useState<TopKey>("endpoint");
  const savedLayout = usePageLayoutStore((state) => state.getLayout("statistics"));
  const layout = resolveViewLayout(statisticsLayoutDefinition, savedLayout);

  return (
    <div className="flex w-full min-w-0 flex-col gap-6">
      <PageLayoutEditor view="statistics" definition={statisticsLayoutDefinition} />
      <PageSectionHost
        layout={layout}
        registry={{
          header: {
            title: "标题",
            render: () => <h1 className="text-2xl font-light tracking-tight">统计</h1>,
          },
          tabs: {
            title: "统计面板",
            render: () => (
              <Tabs value={tab} onValueChange={(value) => setTab(value as TopKey)}>
                <TabsList>
                  {TOP_TABS.map((item) => (
                    <TabsTrigger key={item.key} value={item.key}>
                      {item.label}
                    </TabsTrigger>
                  ))}
                </TabsList>

                <TabsContent value="endpoint">
                  <EndpointStatsPanel />
                </TabsContent>
                <TabsContent value="usage">
                  <UsagePanel />
                </TabsContent>
              </Tabs>
            ),
          },
        }}
      />
    </div>
  );
}
