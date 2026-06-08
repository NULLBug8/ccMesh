import { useEffect, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { toast } from "sonner";

import { StatusDot, TabularText } from "@/components/ui";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent } from "@/components/ui/card";
import { Switch } from "@/components/ui/switch";
import { healthApi } from "@/services/modules/health";
import { proxyApi } from "@/services/modules/proxy";
import { statsApi } from "@/services/modules/stats";
import { useLayoutStore } from "@/stores";
import { useProxyStore } from "@/stores/modules/proxy";
import { SeaTide } from "./SeaTide";

/**
 * 仪表盘首卡（左 2/3 / 右 1/3 双卡片）：
 * 左卡=启用端点列表（当前工作端点蓝色高亮）；
 * 右卡=本地代理信息 + 开关 + 端口跳设置，运行时叠加海水涨潮动效。
 */
export function ServiceCard() {
  const status = useProxyStore((s) => s.status);
  const setStatus = useProxyStore((s) => s.setStatus);
  const setActiveView = useLayoutStore((s) => s.setActiveView);
  // 最近一条请求明细对应的端点（与实时监控同源，第一时间反映轮换/故障转移）。
  const [liveEndpoint, setLiveEndpoint] = useState<string | null>(null);
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
    <div className="grid grid-cols-1 gap-4 md:grid-cols-3">
      {/* 左 2/3：启用端点列表 */}
      <Card className="md:col-span-2">
        <CardContent className="flex flex-col gap-3 px-5 py-4">
          <div className="flex flex-col gap-1.5">
            <span className="text-xs text-ink-secondary">
              启用端点 <TabularText>{endpoints.length}</TabularText>
            </span>
            {endpoints.length === 0 ? (
              <span className="text-sm text-ink-mute">暂无启用端点</span>
            ) : (
              <ul className="flex flex-wrap gap-2">
                {endpoints.map((e) => {
                  const active = e.name === current;
                  return (
                    <li
                      key={e.name}
                      className="flex items-center gap-1.5 rounded-md px-2 py-1 text-sm"
                    >
                      <StatusDot
                        status={active && running ? "info" : "success"}
                        pulse={active && running}
                      />
                      {active ? (
                        <Badge variant="info">{e.name}</Badge>
                      ) : (
                        <span>{e.name}</span>
                      )}
                    </li>
                  );
                })}
              </ul>
            )}
          </div>
        </CardContent>
      </Card>

      {/* 右 1/3：本地代理信息 + 开关 + 端口跳设置 + 水纹波效 */}
      <Card className="relative overflow-hidden md:col-span-1">
        {running && <SeaTide />}
        <CardContent className="relative z-10 flex h-full flex-col justify-between gap-3 px-5 py-4">
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
        </CardContent>
      </Card>
    </div>
  );
}
