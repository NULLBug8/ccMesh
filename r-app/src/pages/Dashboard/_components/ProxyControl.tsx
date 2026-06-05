import { useEffect } from "react";
import { toast } from "sonner";

import { Badge } from "@/components/ui/badge";
import { Card, CardContent } from "@/components/ui/card";
import { Switch } from "@/components/ui/switch";
import { StatusDot, TabularText } from "@/components/ui";
import { proxyApi } from "@/services/modules/proxy";
import { useProxyStore } from "@/stores/modules/proxy";

export function ProxyControl() {
  const status = useProxyStore((s) => s.status);
  const setStatus = useProxyStore((s) => s.setStatus);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    proxyApi.status().then(setStatus).catch(() => undefined);
    proxyApi.onStatusChanged(setStatus).then((un) => {
      unlisten = un;
    });
    return () => unlisten?.();
  }, [setStatus]);

  const running = status?.running ?? false;

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
    <Card>
      <CardContent className="flex items-center justify-between px-5 py-4">
        <div className="flex flex-col gap-1.5">
          <div className="flex items-center gap-2">
            <StatusDot status={running ? "success" : "idle"} pulse={running} />
            <span className="text-sm font-medium">
              本地代理 {running ? "运行中" : "已停止"}
            </span>
          </div>
          <div className="flex items-center gap-2 text-xs text-ink-secondary">
            <span>
              端口 <TabularText>{status?.port ?? "—"}</TabularText>
            </span>
            <span>·</span>
            <span>
              启用端点 <TabularText>{status?.enabledEndpointCount ?? 0}</TabularText>
            </span>
            {status?.currentEndpoint ? (
              <>
                <span>·</span>
                <Badge variant="info">{status.currentEndpoint}</Badge>
              </>
            ) : null}
          </div>
        </div>
        <Switch
          checked={running}
          onCheckedChange={toggle}
          aria-label="代理开关"
        />
      </CardContent>
    </Card>
  );
}
