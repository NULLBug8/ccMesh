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
import { windowApi } from "@/services/modules/window";
import { isWebRuntime } from "@/services/runtime";
import { resolveViewLayout, usePageLayoutStore } from "@/stores";
import { TokenCounter } from "./_components/TokenCounter";
import { UpdateSection } from "./_components/UpdateSection";
import { settingsLayoutDefinition } from "./layout";

function Row({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="flex items-center justify-between px-5 py-3">
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
  const webRuntime = isWebRuntime();
  const qc = useQueryClient();
  const { setTheme } = useTheme();
  const { data: cfg } = useQuery({ queryKey: ["config"], queryFn: configApi.getConfig });
  const savedLayout = usePageLayoutStore((state) => state.getLayout("settings"));
  const layout = resolveViewLayout(settingsLayoutDefinition, savedLayout);

  const save = async (patch: Record<string, string>) => {
    try {
      await configApi.setConfig(patch);
      qc.invalidateQueries({ queryKey: ["config"] });
      qc.invalidateQueries({ queryKey: ["app-config"] });
    } catch (error) {
      toast.error(`保存失败：${errMsg(error)}`);
    }
  };

  const autostartQ = useQuery({
    queryKey: ["autostart-enabled"],
    queryFn: async () => {
      if (webRuntime) return false;
      const { isEnabled } = await import("@tauri-apps/plugin-autostart");
      return isEnabled();
    },
  });

  const toggleAutostart = async (enabled: boolean) => {
    try {
      if (webRuntime) {
        toast.info("Web 端不支持系统自启动");
        return;
      }
      const { enable, disable } = await import("@tauri-apps/plugin-autostart");
      if (enabled) await enable();
      else await disable();
      qc.invalidateQueries({ queryKey: ["autostart-enabled"] });
    } catch (error) {
      toast.error(`设置开机自启失败：${errMsg(error)}`);
      qc.invalidateQueries({ queryKey: ["autostart-enabled"] });
    }
  };

  const [testingProxy, setTestingProxy] = useState(false);
  const proxyRef = useRef<HTMLInputElement>(null);

  const testProxy = async () => {
    const url = (proxyRef.current?.value ?? "").trim() || cfg?.proxyUrl || "";
    setTestingProxy(true);
    try {
      const result = await configApi.testProxy(url);
      if (result.success) toast.success(`${result.message}，${result.latencyMs}ms`);
      else toast.error(result.message);
    } catch (error) {
      toast.error(`测试失败：${errMsg(error)}`);
    } finally {
      setTestingProxy(false);
    }
  };

  if (!cfg) {
    return <p className="text-sm text-ink-mute">加载中...</p>;
  }

  const generalSection = (
    <section className="flex flex-col gap-2">
      <div>
        <h2 className="text-sm font-medium text-ink-secondary">常规</h2>
        <p className="text-xs text-ink-mute">端口、外观与窗口行为</p>
      </div>
      <div className="flex flex-col divide-y divide-edge-subtle rounded-lg border border-edge">
        <Row label="代理端口">
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
          <Switch
            checked={cfg.themeAuto}
            onCheckedChange={(value) => save({ themeAuto: String(value) })}
          />
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

        <Row label="语言">
          <Select
            value={cfg.language}
            onValueChange={(value) => {
              windowApi.setLanguage(value).catch(() => undefined);
              save({ language: value });
            }}
          >
            <SelectTrigger className="w-36">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="zh">中文</SelectItem>
              <SelectItem value="en">English</SelectItem>
            </SelectContent>
          </Select>
        </Row>

        <Row label="关闭窗口行为">
          <Select
            value={cfg.closeWindowBehavior}
            onValueChange={(value) => save({ closeWindowBehavior: value })}
          >
            <SelectTrigger className="w-36">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="ask">每次询问</SelectItem>
              <SelectItem value="minimize">最小化到托盘</SelectItem>
              <SelectItem value="quit">直接退出</SelectItem>
            </SelectContent>
          </Select>
        </Row>

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

  const startupSection = (
    <section className="flex flex-col gap-2">
      <div>
        <h2 className="text-sm font-medium text-ink-secondary">启动行为</h2>
        <p className="text-xs text-ink-mute">应用启动与随系统启动行为</p>
      </div>
      <div className="flex flex-col divide-y divide-edge-subtle rounded-lg border border-edge">
        <DescRow title="自启动" desc="跟随系统自动启动">
          <Switch
            checked={autostartQ.data ?? false}
            disabled={autostartQ.isLoading || webRuntime}
            onCheckedChange={toggleAutostart}
            aria-label="自启动"
          />
        </DescRow>
        <DescRow title="静默启动" desc="后台启动，启动时不显示窗口，常驻托盘运行">
          <Switch
            checked={cfg.silentStart}
            onCheckedChange={(value) => save({ silentStart: String(value) })}
            aria-label="静默启动"
          />
        </DescRow>
        <DescRow title="自动运行" desc="应用打开时自动启动代理服务">
          <Switch
            checked={cfg.autoRun}
            onCheckedChange={(value) => save({ autoRun: String(value) })}
            aria-label="自动运行"
          />
        </DescRow>
      </div>
    </section>
  );

  const proxySection = (
    <section className="flex flex-col gap-2">
      <div>
        <h2 className="text-sm font-medium text-ink-secondary">代理</h2>
        <p className="text-xs text-ink-mute">网络代理设置</p>
      </div>
      <div className="flex flex-col divide-y divide-edge-subtle rounded-lg border border-edge">
        <Row label="启用代理">
          <div className="flex items-center gap-3">
            <span className="text-xs text-ink-mute">通过代理路由所有网络请求</span>
            <Switch
              checked={cfg.proxyEnabled}
              onCheckedChange={(value) => save({ proxyEnabled: String(value) })}
              aria-label="启用代理"
            />
          </div>
        </Row>
        <Row label="代理服务器">
          <div className="flex items-center gap-2">
            <Input
              ref={proxyRef}
              className="w-56"
              placeholder="http://127.0.0.1:7890"
              defaultValue={cfg.proxyUrl}
              onBlur={(event) => save({ proxyUrl: event.target.value })}
            />
            <Button variant="outline" size="sm" onClick={testProxy} disabled={testingProxy}>
              测试
            </Button>
          </div>
        </Row>
        <Row label="代理更新">
          <div className="flex items-center gap-3">
            <span className="text-xs text-ink-mute">通过代理检查和下载应用更新</span>
            <Switch
              checked={cfg.proxyForUpdate}
              disabled={!cfg.proxyEnabled}
              onCheckedChange={(value) => save({ proxyForUpdate: String(value) })}
              aria-label="代理更新"
            />
          </div>
        </Row>
      </div>
      <p className="px-1 text-xs text-ink-mute">例如 127.0.0.1:7890 或 http://proxy:8080</p>
    </section>
  );

  const advancedSection = (
    <section className="flex flex-col gap-2">
      <div>
        <h2 className="text-sm font-medium text-ink-secondary">系统 / 高级</h2>
        <p className="text-xs text-ink-mute">
          伪装上游 User-Agent，清空后透传客户端 UA
        </p>
      </div>
      <div className="flex flex-col divide-y divide-edge-subtle rounded-lg border border-edge">
        <div className="flex flex-col gap-1.5 px-5 py-3">
          <div className="flex items-baseline justify-between gap-3">
            <span className="text-sm">OpenAI / Codex 端点 UA</span>
            <span className="truncate font-mono text-xs text-ink-mute">
              codex_cli_rs/0.114.0 (Mac OS 14.2.0; x86_64) vscode/1.111.0
            </span>
          </div>
          <Input
            placeholder="清空后透传客户端 UA"
            defaultValue={cfg.openaiUa}
            onBlur={(event) => save({ openaiUa: event.target.value })}
          />
        </div>
        <div className="flex flex-col gap-1.5 px-5 py-3">
          <div className="flex items-baseline justify-between gap-3">
            <span className="text-sm">Claude 端点 UA</span>
            <span className="truncate font-mono text-xs text-ink-mute">
              claude-cli/2.1.185 (external, sdk-cli)
            </span>
          </div>
          <Input
            placeholder="清空后透传客户端 UA"
            defaultValue={cfg.claudeCliUa}
            onBlur={(event) => save({ claudeCliUa: event.target.value })}
          />
        </div>
      </div>
    </section>
  );

  return (
    <div className="mx-auto flex max-w-5xl flex-col gap-6">
      <PageLayoutEditor view="settings" definition={settingsLayoutDefinition} />
      <PageSectionHost
        layout={layout}
        registry={{
          header: {
            title: "标题",
            render: () => <h1 className="text-2xl font-light tracking-tight">设置</h1>,
          },
          general: {
            title: "常规设置",
            render: () => generalSection,
          },
          startup: {
            title: "启动行为",
            render: () => startupSection,
          },
          proxy: {
            title: "代理设置",
            render: () => proxySection,
          },
          advanced: {
            title: "系统与高级",
            render: () => advancedSection,
          },
          update: {
            title: "更新",
            render: () => <UpdateSection />,
          },
          tokens: {
            title: "Token 统计",
            render: () => <TokenCounter />,
          },
        }}
      />
    </div>
  );
}
