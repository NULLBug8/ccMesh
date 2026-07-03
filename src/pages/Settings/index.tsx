import { useRef, useState, type ReactNode } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useTheme } from "next-themes";
import { toast } from "sonner";

import { PageLayoutEditor } from "@/components/business/page-layout/PageLayoutEditor";
import { PageSectionHost } from "@/components/business/page-layout/PageSectionHost";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Switch } from "@/components/ui/switch";
import { configApi } from "@/services/modules/config";
import { logsApi } from "@/services/modules/logs";
import { resolveViewLayout, usePageLayoutStore } from "@/stores";
import { TokenCounter } from "./_components/TokenCounter";
import { settingsLayoutDefinition } from "./layout";

function Row({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="flex items-center justify-between gap-4 px-5 py-3">
      <span className="text-sm">{label}</span>
      {children}
    </div>
  );
}

function DescRow({
  title,
  desc,
  children,
}: {
  title: string;
  desc: string;
  children: ReactNode;
}) {
  return (
    <div className="flex items-center justify-between gap-3 px-5 py-3">
      <div className="flex flex-col gap-0.5">
        <span className="text-sm">{title}</span>
        <span className="text-xs text-ink-mute">{desc}</span>
      </div>
      {children}
    </div>
  );
}

const errMsg = (error: unknown) => (error instanceof Error ? error.message : String(error));

export function Settings() {
  const qc = useQueryClient();
  const { setTheme } = useTheme();
  const { data: cfg } = useQuery({ queryKey: ["config"], queryFn: configApi.getConfig });
  const savedLayout = usePageLayoutStore((state) => state.getLayout("settings"));
  const layout = resolveViewLayout(settingsLayoutDefinition, savedLayout);
  const [testingProxy, setTestingProxy] = useState(false);
  const proxyRef = useRef<HTMLInputElement>(null);

  const save = async (patch: Record<string, string>) => {
    try {
      await configApi.setConfig(patch);
      qc.invalidateQueries({ queryKey: ["config"] });
      qc.invalidateQueries({ queryKey: ["app-config"] });
    } catch (error) {
      toast.error(`保存失败：${errMsg(error)}`);
    }
  };

  const testProxy = async () => {
    const url = (proxyRef.current?.value ?? "").trim() || cfg?.proxyUrl || "";
    setTestingProxy(true);
    try {
      const result = await configApi.testProxy(url);
      if (result.success) toast.success(`${result.message} (${result.latencyMs}ms)`);
      else toast.error(result.message);
    } catch (error) {
      toast.error(`测试失败：${errMsg(error)}`);
    } finally {
      setTestingProxy(false);
    }
  };

  if (!cfg) return <p className="text-sm text-ink-mute">加载中...</p>;

  const generalSection = (
    <section className="flex flex-col gap-2">
      <div>
        <h2 className="text-sm font-medium text-ink-secondary">常规</h2>
        <p className="text-xs text-ink-mute">服务端口、外观和日志级别</p>
      </div>
      <div className="flex flex-col divide-y divide-edge-subtle rounded-lg border border-edge">
        <Row label="服务端口">
          <Input
            className="w-32"
            defaultValue={String(cfg.port)}
            onBlur={(event) => save({ port: event.target.value })}
          />
        </Row>
        <Row label="主题">
          <Select
            value={cfg.theme}
            onValueChange={(value) => {
              setTheme(value);
              save({ theme: value });
            }}
          >
            <SelectTrigger className="w-36">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="system">跟随系统</SelectItem>
              <SelectItem value="light">浅色</SelectItem>
              <SelectItem value="dark">深色</SelectItem>
            </SelectContent>
          </Select>
        </Row>
        <Row label="定时自动切换主题">
          <Switch checked={cfg.themeAuto} onCheckedChange={(value) => save({ themeAuto: String(value) })} />
        </Row>
        {cfg.themeAuto && (
          <Row label="浅色 / 深色起始时间">
            <div className="flex items-center gap-2">
              <Input
                type="time"
                className="w-28"
                defaultValue={cfg.autoLightStart}
                onBlur={(event) => save({ autoLightStart: event.target.value })}
              />
              <Input
                type="time"
                className="w-28"
                defaultValue={cfg.autoDarkStart}
                onBlur={(event) => save({ autoDarkStart: event.target.value })}
              />
            </div>
          </Row>
        )}
        <Row label="日志级别">
          <Select
            value={cfg.logLevel}
            onValueChange={(value) => {
              logsApi.setLevel(value).catch(() => undefined);
              qc.invalidateQueries({ queryKey: ["config"] });
            }}
          >
            <SelectTrigger className="w-36">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {["trace", "debug", "info", "warn", "error"].map((level) => (
                <SelectItem key={level} value={level}>
                  {level}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </Row>
      </div>
    </section>
  );

  const proxySection = (
    <section className="flex flex-col gap-2">
      <div>
        <h2 className="text-sm font-medium text-ink-secondary">网络代理</h2>
        <p className="text-xs text-ink-mute">控制服务端主动请求和端点转发的代理行为</p>
      </div>
      <div className="flex flex-col divide-y divide-edge-subtle rounded-lg border border-edge">
        <DescRow title="启用全局代理" desc="端点未单独开启 useProxy 时，按此开关决定是否走代理">
          <Switch
            checked={cfg.proxyEnabled}
            onCheckedChange={(value) => save({ proxyEnabled: String(value) })}
          />
        </DescRow>
        <Row label="代理服务器">
          <div className="flex items-center gap-2">
            <Input
              ref={proxyRef}
              className="w-64"
              placeholder="http://127.0.0.1:7890"
              defaultValue={cfg.proxyUrl}
              onBlur={(event) => save({ proxyUrl: event.target.value })}
            />
            <Button variant="outline" size="sm" onClick={testProxy} disabled={testingProxy}>
              测试
            </Button>
          </div>
        </Row>
      </div>
    </section>
  );

  const advancedSection = (
    <section className="flex flex-col gap-2">
      <div>
        <h2 className="text-sm font-medium text-ink-secondary">高级</h2>
        <p className="text-xs text-ink-mute">User-Agent 伪装</p>
      </div>
      <div className="flex flex-col divide-y divide-edge-subtle rounded-lg border border-edge">
        <div className="flex flex-col gap-1.5 px-5 py-3">
          <span className="text-sm">OpenAI / Codex UA</span>
          <Input
            placeholder="留空时使用内置 Codex 探针 UA"
            defaultValue={cfg.openaiUa}
            onBlur={(event) => save({ openaiUa: event.target.value })}
          />
        </div>
        <div className="flex flex-col gap-1.5 px-5 py-3">
          <span className="text-sm">Claude UA</span>
          <Input
            placeholder="留空时使用内置 Claude CLI UA"
            defaultValue={cfg.claudeCliUa}
            onBlur={(event) => save({ claudeCliUa: event.target.value })}
          />
        </div>
      </div>
    </section>
  );

  return (
    <div className="flex w-full min-w-0 flex-col gap-6">
      <PageLayoutEditor view="settings" definition={settingsLayoutDefinition} />
      <PageSectionHost
        layout={layout}
        registry={{
          header: {
            title: "标题",
            render: () => <h1 className="text-2xl font-light tracking-tight">设置</h1>,
          },
          general: { title: "常规设置", render: () => generalSection },
          proxy: { title: "代理设置", render: () => proxySection },
          advanced: { title: "高级设置", render: () => advancedSection },
          tokens: { title: "Token 统计", render: () => <TokenCounter /> },
        }}
      />
    </div>
  );
}
