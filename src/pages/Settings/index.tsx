import { useRef, useState, type ReactNode } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useTheme } from "next-themes";
import { disable, enable, isEnabled } from "@tauri-apps/plugin-autostart";
import { toast } from "sonner";

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
import { TokenCounter } from "./_components/TokenCounter";
import { UpdateSection } from "./_components/UpdateSection";

function Row({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="flex items-center justify-between px-5 py-3">
      <span className="text-sm">{label}</span>
      {children}
    </div>
  );
}

/** 带副标题的设置行（左标题+说明，右控件），用于启动行为开关。 */
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

const errMsg = (e: unknown) => (e instanceof Error ? e.message : String(e));

export function Settings() {
  const qc = useQueryClient();
  const { setTheme } = useTheme();
  const { data: cfg } = useQuery({ queryKey: ["config"], queryFn: configApi.getConfig });

  const save = async (patch: Record<string, string>) => {
    try {
      await configApi.setConfig(patch);
      qc.invalidateQueries({ queryKey: ["config"] });
      // 配置文件管理页用 ["app-config"] 读取端口生成端点 URL，端口等变更后需同步刷新
      qc.invalidateQueries({ queryKey: ["app-config"] });
    } catch (e) {
      toast.error(`保存失败：${errMsg(e)}`);
    }
  };

  // 自启动状态真相源为系统态（autostart 插件），不落库，避免与系统注册不一致。
  const autostartQ = useQuery({
    queryKey: ["autostart-enabled"],
    queryFn: () => isEnabled(),
  });
  const toggleAutostart = async (on: boolean) => {
    try {
      if (on) await enable();
      else await disable();
      qc.invalidateQueries({ queryKey: ["autostart-enabled"] });
    } catch (e) {
      toast.error(`设置开机自启失败：${errMsg(e)}`);
      qc.invalidateQueries({ queryKey: ["autostart-enabled"] });
    }
  };

  const [testingProxy, setTestingProxy] = useState(false);
  const proxyRef = useRef<HTMLInputElement>(null);
  const testProxy = async () => {
    const url = (proxyRef.current?.value ?? "").trim() || cfg?.proxyUrl || "";
    setTestingProxy(true);
    try {
      const r = await configApi.testProxy(url);
      if (r.success) toast.success(`${r.message}（${r.latencyMs}ms）`);
      else toast.error(r.message);
    } catch (e) {
      toast.error(`测试失败：${errMsg(e)}`);
    } finally {
      setTestingProxy(false);
    }
  };

  if (!cfg) {
    return <p className="text-sm text-ink-mute">加载中…</p>;
  }

  return (
    <div className="mx-auto flex max-w-2xl flex-col gap-6">
      <h1 className="text-2xl font-light tracking-tight">设置</h1>

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
            onBlur={(e) => save({ port: e.target.value })}
          />
        </Row>

        <Row label="主题">
          <Select
            value={cfg.theme}
            onValueChange={(v) => {
              setTheme(v);
              save({ theme: v });
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
            onCheckedChange={(v) => save({ themeAuto: String(v) })}
          />
        </Row>

        {cfg.themeAuto && (
          <Row label="浅色 / 深色起始时间">
            <div className="flex items-center gap-2">
              <Input
                type="time"
                className="w-28"
                defaultValue={cfg.autoLightStart}
                onBlur={(e) => save({ autoLightStart: e.target.value })}
              />
              <Input
                type="time"
                className="w-28"
                defaultValue={cfg.autoDarkStart}
                onBlur={(e) => save({ autoDarkStart: e.target.value })}
              />
            </div>
          </Row>
        )}

        <Row label="语言">
          <Select
            value={cfg.language}
            onValueChange={(v) => {
              windowApi.setLanguage(v).catch(() => undefined);
              save({ language: v });
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
            onValueChange={(v) => save({ closeWindowBehavior: v })}
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
            onValueChange={(v) => {
              logsApi.setLevel(v).catch(() => undefined);
              qc.invalidateQueries({ queryKey: ["config"] });
            }}
          >
            <SelectTrigger className="w-36">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {["trace", "debug", "info", "warn", "error"].map((l) => (
                <SelectItem key={l} value={l}>
                  {l}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </Row>
        </div>
      </section>

      <section className="flex flex-col gap-2">
        <div>
          <h2 className="text-sm font-medium text-ink-secondary">启动行为</h2>
          <p className="text-xs text-ink-mute">应用开启与随系统启动行为</p>
        </div>
        <div className="flex flex-col divide-y divide-edge-subtle rounded-lg border border-edge">
          <DescRow title="自启动" desc="跟随系统自启动">
            <Switch
              checked={autostartQ.data ?? false}
              disabled={autostartQ.isLoading}
              onCheckedChange={toggleAutostart}
              aria-label="自启动"
            />
          </DescRow>
          <DescRow title="静默启动" desc="后台启动，启动时不展示窗口，常驻托盘运行">
            <Switch
              checked={cfg.silentStart}
              onCheckedChange={(v) => save({ silentStart: String(v) })}
              aria-label="静默启动"
            />
          </DescRow>
          <DescRow title="自动运行" desc="应用打开时自动启动代理服务">
            <Switch
              checked={cfg.autoRun}
              onCheckedChange={(v) => save({ autoRun: String(v) })}
              aria-label="自动运行"
            />
          </DescRow>
        </div>
      </section>

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
                onCheckedChange={(v) => save({ proxyEnabled: String(v) })}
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
                onBlur={(e) => save({ proxyUrl: e.target.value })}
              />
              <Button
                variant="outline"
                size="sm"
                onClick={testProxy}
                disabled={testingProxy}
              >
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
                onCheckedChange={(v) => save({ proxyForUpdate: String(v) })}
                aria-label="代理更新"
              />
            </div>
          </Row>
        </div>
        <p className="px-1 text-xs text-ink-mute">例如 127.0.0.1:7890 或 http://proxy:8080</p>
      </section>

      <section className="flex flex-col gap-2">
        <div>
          <h2 className="text-sm font-medium text-ink-secondary">系统 / 高级</h2>
          <p className="text-xs text-ink-mute">伪装上游 User-Agent（清空则透传客户端 UA）</p>
        </div>
        <div className="flex flex-col divide-y divide-edge-subtle rounded-lg border border-edge">
          <div className="flex flex-col gap-1.5 px-5 py-3">
            <div className="flex items-baseline justify-between gap-3">
              <span className="text-sm">OpenAI / Codex 端点 UA</span>
              <span className="truncate font-mono text-xs text-ink-mute">
                默认 Codex CLI UA，适配 new-api 客户端限制
              </span>
            </div>
            <Input
              placeholder="清空后透传客户端 UA"
              defaultValue={cfg.openaiUa}
              onBlur={(e) => save({ openaiUa: e.target.value })}
            />
          </div>
          <div className="flex flex-col gap-1.5 px-5 py-3">
            <div className="flex items-baseline justify-between gap-3">
              <span className="text-sm">Claude 端点 UA</span>
              <span className="truncate font-mono text-xs text-ink-mute">
                例 claude-cli/2.1.2 (external, cli)
              </span>
            </div>
            <Input
              placeholder="清空后透传客户端 UA"
              defaultValue={cfg.claudeCliUa}
              onBlur={(e) => save({ claudeCliUa: e.target.value })}
            />
          </div>
        </div>
      </section>

      <UpdateSection />
      <TokenCounter />
    </div>
  );
}
