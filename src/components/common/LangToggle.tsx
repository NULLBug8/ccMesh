import { Button } from "@/components/ui/button";
import { useLayoutStore } from "@/stores";

export function LangToggle() {
  const lang = useLayoutStore((s) => s.lang);
  const toggleLang = useLayoutStore((s) => s.toggleLang);

  return (
    <Button variant="outline" size="icon" aria-label="切换语言" onClick={toggleLang}>
      <span className="text-xs font-medium">{lang === "zh" ? "中" : "EN"}</span>
    </Button>
  );
}