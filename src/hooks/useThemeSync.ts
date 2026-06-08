import { useEffect, useRef } from "react";
import { useTheme } from "next-themes";

import { configApi } from "@/services/modules/config";

/** 启动时从后端配置恢复主题；之后主题变更回写后端（供跨设备同步）。 */
export function useThemeSync() {
  const { theme, setTheme } = useTheme();
  const initialized = useRef(false);

  useEffect(() => {
    configApi
      .getConfig()
      .then((cfg) => {
        if (cfg.theme && cfg.theme !== theme) setTheme(cfg.theme);
      })
      .catch(() => undefined)
      .finally(() => {
        initialized.current = true;
      });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    if (!initialized.current || !theme) return;
    configApi.setConfig({ theme }).catch(() => undefined);
  }, [theme]);
}
