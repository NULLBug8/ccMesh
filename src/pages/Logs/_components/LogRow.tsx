import type { ReactNode } from "react";

import { TabularText } from "@/components/ui";
import type { LogLine } from "@/services/modules/logs";
import { LEVEL_BADGE } from "./logLevels";

/** 关键字命中高亮（大小写不敏感）。 */
function highlight(text: string, kw: string): ReactNode {
  if (!kw) return text;
  const lower = text.toLowerCase();
  const k = kw.toLowerCase();
  let idx = lower.indexOf(k);
  if (idx === -1) return text;
  const parts: ReactNode[] = [];
  let i = 0;
  let n = 0;
  while (idx !== -1) {
    if (idx > i) parts.push(text.slice(i, idx));
    parts.push(
      <mark key={n++} className="rounded-sm bg-warning/30 text-ink-primary">
        {text.slice(idx, idx + kw.length)}
      </mark>,
    );
    i = idx + kw.length;
    idx = lower.indexOf(k, i);
  }
  if (i < text.length) parts.push(text.slice(i));
  return parts;
}

/** 单条日志行：时间 + 等级徽章 + 来源 + message(高亮) + 结构化字段 chips。 */
export function LogRow({ line, keyword }: { line: LogLine; keyword: string }) {
  const badge = LEVEL_BADGE[line.level] ?? "bg-ink-mute/15 text-ink-mute";
  const shortTarget = line.target.split("::").slice(-2).join("::");
  return (
    <div className="flex gap-2 py-0.5 hover:bg-surface-hover/40">
      <TabularText className="shrink-0 text-ink-mute">{line.time}</TabularText>
      <span
        className={`inline-flex h-4 w-12 shrink-0 items-center justify-center rounded text-[10px] font-medium uppercase ${badge}`}
      >
        {line.level}
      </span>
      {line.target ? (
        <span
          className="max-w-[140px] shrink-0 truncate text-ink-mute"
          title={line.target}
        >
          {shortTarget}
        </span>
      ) : null}
      <span className="min-w-0 flex-1 break-all whitespace-pre-wrap text-ink-primary">
        {highlight(line.message, keyword)}
        {line.fields.map((f) => (
          <span
            key={f.key}
            className="ml-1.5 rounded bg-ink-mute/10 px-1 text-ink-secondary"
          >
            {f.key}={f.value}
          </span>
        ))}
      </span>
    </div>
  );
}
