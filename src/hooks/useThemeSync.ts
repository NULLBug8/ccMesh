import { useEffect, useRef } from "react";
import { useTheme } from "next-themes";
import { useQuery } from "@tanstack/react-query";

import { configApi } from "@/services/modules/config";
import { revealMainWindow } from "@/lib/boot";

/** 启动时从后端配置恢复主题；之后主题变更回写后端（供跨设备同步）。 */
export function useThemeSync() {
  const { theme, setTheme } = useTheme();
  const initialized = useRef(false);
  // 复用 ["config"] 查询缓存，与 useAutoTheme 共享，避免首屏重复 IPC
  const { data: cfg, isError } = useQuery({
    queryKey: ["config"],
    queryFn: configApi.getConfig,
  });

  // 首次拿到后端配置后恢复主题，并在主题就绪、首帧绘制后显示窗口
  useEffect(() => {
    if (initialized.current || !cfg) return;
    initialized.current = true;
    if (cfg.theme && cfg.theme !== theme) setTheme(cfg.theme);
    requestAnimationFrame(() =>
      requestAnimationFrame(() => void revealMainWindow()),
    );
  }, [cfg, theme, setTheme]);

  // 配置请求失败时也兜底显示窗口（避免窗口永不出现）
  useEffect(() => {
    if (isError) void revealMainWindow();
  }, [isError]);

  // 主题变更后回写后端
  useEffect(() => {
    if (!initialized.current || !theme) return;
    configApi.setConfig({ theme }).catch(() => undefined);
  }, [theme]);
}
