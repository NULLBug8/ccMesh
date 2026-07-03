import { useEffect, useRef } from "react";
import { useTheme } from "next-themes";
import { useQuery } from "@tanstack/react-query";

import { configApi } from "@/services/modules/config";

export function useThemeSync() {
  const { theme, setTheme } = useTheme();
  const initialized = useRef(false);
  const { data: cfg } = useQuery({
    queryKey: ["config"],
    queryFn: configApi.getConfig,
  });

  useEffect(() => {
    if (initialized.current || !cfg) return;
    initialized.current = true;
    if (cfg.theme && cfg.theme !== theme) setTheme(cfg.theme);
  }, [cfg, theme, setTheme]);

  useEffect(() => {
    if (!initialized.current || !theme) return;
    configApi.setConfig({ theme }).catch(() => undefined);
  }, [theme]);
}
