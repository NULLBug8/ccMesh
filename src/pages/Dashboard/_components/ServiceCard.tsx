import { CopyIcon } from "lucide-react";
import { useEffect, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { useTheme } from "next-themes";
import { toast } from "sonner";

import { StatusDot, TabularText } from "@/components/ui";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent } from "@/components/ui/card";
import { Switch } from "@/components/ui/switch";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { useEndpointHealth, useEndpointHealthEvents } from "@/hooks/useEndpointHealth";
import { cn } from "@/lib/utils";
import { circuitDot, healthApi } from "@/services/modules/health";
import { proxyApi } from "@/services/modules/proxy";
import { statsApi } from "@/services/modules/stats";
import { useLayoutStore } from "@/stores";
import { useProxyStore } from "@/stores/modules/proxy";
import { ProxyScene } from "./ProxyScene";

/**
 * 仪表盘首卡（左 2/3 / 右 1/3 双卡片）：
 * 左卡=启用端点列表（当前工作端点蓝色高亮）；
 * 右卡=本地代理信息 + 开关 + 端口跳设置，叠加雪山日落场景（开启太阳升起、关闭落下）。
 */
export function ServiceCard() {
  const status = useProxyStore((s) => s.status);
  const setStatus = useProxyStore((s) => s.setStatus);
  const setActiveView = useLayoutStore((s) => s.setActiveView);
  const { resolvedTheme } = useTheme();
  const dark = resolvedTheme === "dark";
  // 最近一条请求明细对应的端点（与实时监控同源，第一时间反映轮换/故障转移）。
  const [liveEndpoint, setLiveEndpoint] = useState<string | null>(null);
  const { data: health } = useQuery({
    queryKey: ["health"],
    queryFn: healthApi.getHealth,
  });
  // 端点实时健康/熔断态；健康/端点变更事件到达即刷新（共享 hook 统一订阅）。
  useEndpointHealthEvents();
  const { data: epHealth } = useEndpointHealth();
  const healthByName = new Map((epHealth ?? []).map((h) => [h.name, h]));

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    proxyApi.status().then(setStatus).catch(() => undefined);
    proxyApi.onStatusChanged(setStatus).then((un) => {
      unlisten = un;
    });
    return () => unlisten?.();
  }, [setStatus]);

  // 实时高亮：新请求明细到达即更新当前工作端点（与下方实时监控同一事件源）。
  useEffect(() => {
    let un: (() => void) | undefined;
    statsApi.onRequestLogged((log) => setLiveEndpoint(log.endpointName)).then((u) => {
      un = u;
    });
    return () => un?.();
  }, []);

  const running = status?.running ?? false;
  // 停机后清空实时端点，避免重启后短暂高亮上次的陈旧端点。
  useEffect(() => {
    if (!running) setLiveEndpoint(null);
  }, [running]);

  // 优先用最近请求明细的端点；回退代理状态；停机不高亮。
  const current = running ? liveEndpoint ?? status?.currentEndpoint ?? null : null;
  const endpoints = (health?.endpoints ?? []).filter((e) => e.enabled);
  const gatewayUrl =
    status?.port != null ? `http://127.0.0.1:${status.port}` : null;

  const copyGateway = async () => {
    if (!gatewayUrl) return;
    try {
      if (navigator.clipboard?.writeText) {
        await navigator.clipboard.writeText(gatewayUrl);
      } else {
        const ta = document.createElement("textarea");
        ta.value = gatewayUrl;
        ta.style.position = "fixed";
        ta.style.opacity = "0";
        document.body.appendChild(ta);
        ta.select();
        document.execCommand("copy");
        document.body.removeChild(ta);
      }
      toast.success("已复制代理信息");
    } catch {
      toast.error("复制失败");
    }
  };

  const toggle = async (next: boolean) => {
    try {
      const s = next ? await proxyApi.start() : await proxyApi.stop();
      setStatus(s);
      toast.success(next ? `代理已启动 · 端口 ${s.port}` : "代理已停止");
    } catch (e) {
      toast.error(`操作失败：${e instanceof Error ? e.message : String(e)}`);
    }
  };

  return (
    <div className="grid min-w-0 grid-cols-1 gap-4 2xl:grid-cols-3">
      {/* 左 2/3：启用端点列表 */}
      <Card className="min-w-0 2xl:col-span-2">
        <CardContent className="flex flex-col gap-3 px-5 py-4">
          <div className="flex flex-col gap-1.5">
            <span className="text-xs text-ink-secondary">
              启用端点 <TabularText>{endpoints.length}</TabularText>
            </span>
            {endpoints.length === 0 ? (
              <span className="text-sm text-ink-mute">暂无启用端点</span>
            ) : (
              <ul className="flex min-w-0 flex-wrap gap-2">
                {endpoints.map((e) => {
                  const active = e.name === current;
                  const h = healthByName.get(e.name);
                  const broken = h && h.circuit !== "closed";
                  const dot = broken
                    ? circuitDot(h.circuit)
                    : active && running
                      ? "info"
                      : "success";
                  const title = broken
                    ? `${h.circuit === "open" ? "熔断中" : "恢复中"}${
                        h.lastError ? ` · ${h.lastError}` : ""
                      }`
                    : undefined;
                  return (
                    <li
                      key={e.name}
                      title={title}
                      className="flex min-w-0 items-center gap-1.5 rounded-md px-2 py-1 text-sm"
                    >
                      <StatusDot status={dot} pulse={active && running} />
                      {active ? (
                        <Badge variant="info">{e.name}</Badge>
                      ) : (
                        <span className="min-w-0 truncate">{e.name}</span>
                      )}
                    </li>
                  );
                })}
              </ul>
            )}
          </div>
        </CardContent>
      </Card>

      {/* 右 1/3：本地代理信息 + 开关 + 端口跳设置 + 雪山日落场景 */}
      <Card className="relative min-w-0 overflow-hidden 2xl:col-span-1">
        <ProxyScene running={running} dark={dark} />
        {/* 文字可读性遮罩：亮色用白雾托底、暗色用深色压底 */}
        <div
          aria-hidden
          className={cn(
            "pointer-events-none absolute inset-0 z-[5]",
            dark
              ? "bg-gradient-to-t from-black/45 via-black/5 to-black/15"
              : "bg-gradient-to-t from-white/60 via-white/5 to-white/40",
          )}
        />
        <CardContent
          className={cn(
            "relative z-10 flex h-full flex-col justify-between gap-3 px-5 py-4",
            dark
              ? "text-white [text-shadow:0_1px_3px_rgba(0,0,0,0.55)]"
              : "text-slate-800 [text-shadow:0_1px_2px_rgba(255,255,255,0.7)]",
          )}
        >
          <div className="flex flex-col gap-1.5">
            <span className="text-sm font-medium">本地代理</span>
            <div className="flex items-center gap-1.5 self-start">
              <button
                type="button"
                onClick={() => setActiveView("settings")}
                className={cn(
                  "cursor-pointer text-xs transition-colors hover:opacity-90",
                  dark
                    ? "text-white/85 hover:text-white"
                    : "text-slate-600 hover:text-slate-900",
                )}
                title="前往设置修改端口"
              >
                端口 <TabularText>{status?.port ?? "—"}</TabularText>
              </button>
              {gatewayUrl ? (
                <Tooltip>
                  <TooltipTrigger asChild>
                    <button
                      type="button"
                      onClick={copyGateway}
                      className={cn(
                        "inline-flex shrink-0 transition-colors",
                        dark
                          ? "text-white/85 hover:text-white"
                          : "text-slate-600 hover:text-slate-900",
                      )}
                      aria-label="复制代理信息"
                    >
                      <CopyIcon className="size-3" />
                    </button>
                  </TooltipTrigger>
                  <TooltipContent>点击复制代理信息</TooltipContent>
                </Tooltip>
              ) : null}
            </div>
          </div>
          <div className="flex items-center justify-between gap-2">
            <span className={cn("text-xs", dark ? "text-white/85" : "text-slate-600")}>
              {running ? "运行中" : "已停止"}
            </span>
            <Switch
              checked={running}
              onCheckedChange={toggle}
              aria-label="代理开关"
            />
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
