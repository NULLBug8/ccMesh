import { useState } from "react";

import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { EndpointStatsPanel } from "./_components/EndpointStatsPanel";
import { UsagePanel } from "./_components/UsagePanel";

const TOP_TABS = [
  { key: "endpoint", label: "端点统计" },
  { key: "usage", label: "用量统计" },
] as const;

type TopKey = (typeof TOP_TABS)[number]["key"];

export function Statistics() {
  const [tab, setTab] = useState<TopKey>("endpoint");

  return (
    <div className="mx-auto flex max-w-5xl flex-col gap-6">
      <h1 className="text-2xl font-light tracking-tight">统计</h1>

      <Tabs value={tab} onValueChange={(v) => setTab(v as TopKey)}>
        <TabsList>
          {TOP_TABS.map((t) => (
            <TabsTrigger key={t.key} value={t.key}>
              {t.label}
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
    </div>
  );
}
