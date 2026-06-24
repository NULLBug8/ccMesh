import { useState } from "react";

import { PageLayoutEditor } from "@/components/business/page-layout/PageLayoutEditor";
import { PageSectionHost } from "@/components/business/page-layout/PageSectionHost";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { resolveViewLayout, usePageLayoutStore } from "@/stores";
import { ClaudeWorkspace } from "./_components/ClaudeWorkspace";
import { CodexWorkspace } from "./_components/CodexWorkspace";
import { configProfilesLayoutDefinition } from "./layout";

type Tab = "claude" | "codex";

export function ConfigProfiles() {
  const [tab, setTab] = useState<Tab>("claude");
  const savedLayout = usePageLayoutStore((state) => state.getLayout("configProfiles"));
  const layout = resolveViewLayout(configProfilesLayoutDefinition, savedLayout);

  return (
    <div className="flex h-full min-h-0 flex-col gap-4">
      <PageLayoutEditor
        view="configProfiles"
        definition={configProfilesLayoutDefinition}
      />
      <PageSectionHost
        layout={layout}
        registry={{
          header: {
            title: "标题与工具栏",
            render: () => (
              <div className="flex items-center justify-between">
                <h1 className="text-2xl font-light tracking-tight">配置文件</h1>
                <Tabs value={tab} onValueChange={(value) => setTab(value as Tab)}>
                  <TabsList>
                    <TabsTrigger value="claude">Claude Code</TabsTrigger>
                    <TabsTrigger value="codex">Codex</TabsTrigger>
                  </TabsList>
                </Tabs>
              </div>
            ),
          },
          workspace: {
            title: "配置工作区",
            className: "min-h-0 flex-1",
            render: () => (
              <div className="min-h-0 flex-1">
                {tab === "claude" ? (
                  <ClaudeWorkspace key="claude" />
                ) : (
                  <CodexWorkspace key="codex" />
                )}
              </div>
            ),
          },
        }}
      />
    </div>
  );
}
