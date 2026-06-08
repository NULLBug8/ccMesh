import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { toast } from "sonner";

import { proxyApi } from "@/services/modules/proxy";

/** 监听托盘 `tray-action` 事件（启停代理）。 */
export function useTrayActions() {
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    listen<string>("tray-action", async (e) => {
      try {
        if (e.payload === "start") {
          await proxyApi.start();
          toast.success("代理已启动");
        } else if (e.payload === "stop") {
          await proxyApi.stop();
          toast.success("代理已停止");
        }
      } catch (err) {
        toast.error(err instanceof Error ? err.message : String(err));
      }
    }).then((u) => {
      unlisten = u;
    });
    return () => unlisten?.();
  }, []);
}
