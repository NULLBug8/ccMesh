import { useEffect, useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { RotateCcwIcon, SaveIcon } from "lucide-react";
import { toast } from "sonner";

import { PageLayoutEditor } from "@/components/business/page-layout/PageLayoutEditor";
import { PageSectionHost } from "@/components/business/page-layout/PageSectionHost";
import { Button } from "@/components/ui/button";
import { rulesApi, type RulesConfig } from "@/services/modules/rules";
import { resolveViewLayout, usePageLayoutStore } from "@/stores";
import { RulesForm } from "./_components/RulesForm";
import { rulesLayoutDefinition } from "./layout";

const errMsg = (error: unknown) => (error instanceof Error ? error.message : String(error));

export function Rules() {
  const qc = useQueryClient();
  const savedLayout = usePageLayoutStore((state) => state.getLayout("rules"));
  const layout = resolveViewLayout(rulesLayoutDefinition, savedLayout);
  const { data, isLoading } = useQuery({
    queryKey: ["rules-config"],
    queryFn: rulesApi.getConfig,
  });
  const [draft, setDraft] = useState<RulesConfig | null>(null);
  const [saving, setSaving] = useState(false);
  const [resetting, setResetting] = useState(false);

  useEffect(() => {
    if (data) {
      setDraft(data);
    }
  }, [data]);

  const save = async () => {
    if (!draft) return;
    setSaving(true);
    try {
      const next = await rulesApi.setConfig(draft);
      setDraft(next);
      qc.invalidateQueries({ queryKey: ["rules-config"] });
      qc.invalidateQueries({ queryKey: ["config"] });
      toast.success("规则配置已保存");
    } catch (error) {
      toast.error(`保存规则失败：${errMsg(error)}`);
    } finally {
      setSaving(false);
    }
  };

  const reset = async () => {
    setResetting(true);
    try {
      const next = await rulesApi.resetConfig();
      setDraft(next);
      qc.invalidateQueries({ queryKey: ["rules-config"] });
      qc.invalidateQueries({ queryKey: ["config"] });
      toast.success("规则配置已恢复默认值");
    } catch (error) {
      toast.error(`重置规则失败：${errMsg(error)}`);
    } finally {
      setResetting(false);
    }
  };

  if (isLoading || !draft) {
    return <p className="text-sm text-ink-mute">加载规则配置中...</p>;
  }

  return (
    <div className="flex w-full min-w-0 flex-col gap-6">
      <PageLayoutEditor view="rules" definition={rulesLayoutDefinition} />
      <PageSectionHost
        layout={layout}
        registry={{
          header: {
            title: "标题",
            render: () => (
              <div className="flex flex-wrap items-start justify-between gap-4">
                <div className="flex flex-col gap-1">
                  <h1 className="text-2xl font-light tracking-tight">规则配置</h1>
                  <p className="text-sm text-ink-mute">
                    单独管理路由、熔断与降级策略，保存后会写入配置；运行中的代理将在重启后使用最新规则。
                  </p>
                </div>
                <div className="flex items-center gap-2">
                  <Button
                    variant="outline"
                    onClick={reset}
                    disabled={resetting || saving}
                  >
                    <RotateCcwIcon className="size-4" />
                    恢复默认
                  </Button>
                  <Button onClick={save} disabled={saving || resetting} aria-label="保存规则">
                    <SaveIcon className="size-4" />
                    保存规则
                  </Button>
                </div>
              </div>
            ),
          },
          routing: {
            title: "路由规则",
            render: () => <RulesForm section="routing" value={draft} onChange={setDraft} />,
          },
          circuitBreaker: {
            title: "熔断规则",
            render: () => (
              <RulesForm section="circuitBreaker" value={draft} onChange={setDraft} />
            ),
          },
          degradation: {
            title: "降级规则",
            render: () => (
              <RulesForm section="degradation" value={draft} onChange={setDraft} />
            ),
          },
        }}
      />
    </div>
  );
}
