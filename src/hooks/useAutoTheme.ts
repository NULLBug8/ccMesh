import { useEffect } from "react";
import { useTheme } from "next-themes";
import { useQuery } from "@tanstack/react-query";

import { configApi } from "@/services/modules/config";

function parseHM(s: string): number {
  const [h, m] = s.split(":").map((x) => Number(x));
  return (h || 0) * 60 + (m || 0);
}

/** themeAuto 开启时，按 autoLightStart / autoDarkStart 时间区间定时切换明暗。 */
export function useAutoTheme() {
  const { setTheme } = useTheme();
  const { data: cfg } = useQuery({ queryKey: ["config"], queryFn: configApi.getConfig });

  const themeAuto = cfg?.themeAuto ?? false;
  const lightStart = cfg?.autoLightStart ?? "07:00";
  const darkStart = cfg?.autoDarkStart ?? "19:00";

  useEffect(() => {
    if (!themeAuto) return;
    const light = parseHM(lightStart);
    const dark = parseHM(darkStart);
    const apply = () => {
      const now = new Date();
      const mins = now.getHours() * 60 + now.getMinutes();
      const isLight =
        light <= dark ? mins >= light && mins < dark : mins >= light || mins < dark;
      setTheme(isLight ? "light" : "dark");
    };
    apply();
    const id = setInterval(apply, 60_000);
    return () => clearInterval(id);
  }, [themeAuto, lightStart, darkStart, setTheme]);
}
