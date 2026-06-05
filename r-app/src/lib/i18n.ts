import { useLayoutStore, type Lang } from "@/stores";
import { zh } from "@/locales/zh";
import { en } from "@/locales/en";

const resources: Record<Lang, Record<string, string>> = { zh, en };

/** 纯函数取值，命中则返回译文，否则回退 key。 */
export function translate(lang: Lang, key: string): string {
  return resources[lang]?.[key] ?? key;
}

/** Hook：基于当前语言（layout store）返回 t(key)。语言切换持久化见 P6-5。 */
export function useTranslation() {
  const lang = useLayoutStore((s) => s.lang);
  return { t: (key: string) => translate(lang, key), lang };
}
