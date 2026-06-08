import { Button } from "@/components/ui/button";
import { windowApi } from "@/services/modules/window";
import { useLayoutStore } from "@/stores";

export function LangToggle() {
  const lang = useLayoutStore((s) => s.lang);
  const toggleLang = useLayoutStore((s) => s.toggleLang);

  const onToggle = () => {
    const next = lang === "zh" ? "en" : "zh";
    toggleLang();
    // 持久化后端并重建托盘文案
    windowApi.setLanguage(next).catch(() => undefined);
  };

  return (
    <Button variant="outline" size="icon" aria-label="切换语言" onClick={onToggle}>
      <span className="text-xs font-medium">{lang === "zh" ? "中" : "EN"}</span>
    </Button>
  );
}
