import { PageLayoutEditor } from "@/components/business/page-layout/PageLayoutEditor";
import { PageSectionHost } from "@/components/business/page-layout/PageSectionHost";
import { resolveViewLayout, usePageLayoutStore } from "@/stores";
import { EndpointStatsPanel } from "./_components/EndpointStatsPanel";
import { statisticsLayoutDefinition } from "./layout";

export function Statistics() {
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
          endpoint: {
            title: "端点统计",
            render: () => <EndpointStatsPanel />,
          },
        }}
      />
    </div>
  );
}
