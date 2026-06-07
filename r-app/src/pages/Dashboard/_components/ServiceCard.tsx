import { useEffect } from "react";
import { useQuery } from "@tanstack/react-query";
import { toast } from "sonner";

import { StatusDot, TabularText } from "@/components/ui";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent } from "@/components/ui/card";
import { Switch } from "@/components/ui/switch";
import { cn } from "@/lib/utils";
import { healthApi } from "@/services/modules/health";
import { proxyApi } from "@/services/modules/proxy";
import { useLayoutStore } from "@/stores";
import { useProxyStore } from "@/stores/modules/proxy";
import { SeaTide } from "./SeaTide";

/**
 * 仪表盘首卡（左 3 / 右 2）：
 * 左=服务状态 + 启用端点列表 + 当前工作端点高亮；右=本地代理信息 + 开关 + 端口跳设置。
 * 运行时叠加海水涨潮动效。
 */
export function ServiceCard() {
  const status = useProxyStore((s) => s.status);
  const setStatus = useProxyStore((s) => s.setStatus);
  const setActiveView = useLayoutStore((s) => s.setActiveView);
  const { data: health } = useQuery({
    queryKey: ["health"],
    queryFn: healthApi.getHealth,
  });

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    proxyApi.status().then(setStatus).catch(() => undefined);
    proxyApi.onStatusChanged(setStatus).then((un) => {
      unlisten = un;
    });
    return () => unlisten?.();
  }, [setStatus]);

  const running = status?.running ?? false;
  const current = status?.currentEndpoint ?? null;
  const endpoints = (health?.endpoints ?? []).filter((e) => e.enabled);

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
    <Card className="relative overflow-hidden">
      {running && <SeaTide />}
      <CardContent className="relative z-10 grid grid-cols-1 gap-6 px-5 py-4 md:grid-cols-5">
        {/* 左 3：服务状态 + 启用端点列表 + 当前工作端点 */}
        <div className="flex flex-col gap-3 md:col-span-3">
          <div className="flex items-center gap-2">
            <StatusDot status={running ? "success" : "idle"} pulse={running} />
            <span className="text-sm font-medium">
              服务{running ? "运行中" : "已停止"}
            </span>
          </div>
          <div className="flex flex-col gap-1.5">
            <span className="text-xs text-ink-secondary">
              启用端点 <TabularText>{endpoints.length}</TabularText>
            </span>
            {endpoints.length === 0 ? (
              <span className="text-sm text-ink-mute">暂无启用端点</span>
            ) : (
              <ul className="flex flex-col gap-1">
                {endpoints.map((e) => {
                  const active = e.name === current;
                  return (
                    <li
                      key={e.name}
                      className={cn(
                        "flex items-center gap-2 rounded-md px-2 py-1 text-sm",
                        active && "bg-primary/10",
                      )}
                    >
                      <StatusDot
                        status={active && running ? "success" : "idle"}
                        pulse={active && running}
                      />
                      <span className={active ? "font-medium" : undefined}>
                        {e.name}
                      </span>
                      {active && (
                        <Badge variant="info" className="ml-auto">
                          当前
                        </Badge>
                      )}
                    </li>
                  );
                })}
              </ul>
            )}
          </div>
        </div>

        {/* 右 2：本地代理信息 + 开关 + 端口跳设置 */}
        <div className="flex flex-col justify-between gap-3 md:col-span-2">
          <div className="flex flex-col gap-1.5">
            <span className="text-sm font-medium">本地代理</span>
            <button
              type="button"
              onClick={() => setActiveView("settings")}
              className="self-start text-xs text-ink-secondary underline-offset-2 transition-colors hover:text-foreground hover:underline"
              title="前往设置修改端口"
            >
              端口 <TabularText>{status?.port ?? "—"}</TabularText>
            </button>
          </div>
          <div className="flex items-center justify-between gap-2">
            <span className="text-xs text-ink-secondary">
              {running ? "运行中" : "已停止"}
            </span>
            <Switch
              checked={running}
              onCheckedChange={toggle}
              aria-label="代理开关"
            />
          </div>
        </div>
      </CardContent>
    </Card>
  );
}
