import { useEffect } from "react";
import { toast } from "sonner";

import { proxyApi } from "@/services/modules/proxy";
import { isWebRuntime } from "@/services/runtime";

export function useTrayActions() {
  useEffect(() => {
    if (isWebRuntime()) return;

    let unlisten: (() => void) | undefined;

    void import("@tauri-apps/api/event").then(({ listen }) => {
      listen<string>("tray-action", async (event) => {
        try {
          if (event.payload === "start") {
            await proxyApi.start();
            toast.success("代理已启动");
          } else if (event.payload === "stop") {
            await proxyApi.stop();
            toast.success("代理已停止");
          }
        } catch (error) {
          toast.error(error instanceof Error ? error.message : String(error));
        }
      }).then((dispose) => {
        unlisten = dispose;
      });
    });

    return () => unlisten?.();
  }, []);
}
