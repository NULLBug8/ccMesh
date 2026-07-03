import type { ReactNode } from "react";

import { Badge } from "@/components/ui/badge";
import { StatusDot, TabularText } from "@/components/ui";
import type { LogLine } from "@/services/modules/logs";
import { cn } from "@/lib/utils";
import { LEVEL_DOT, LEVEL_VARIANT } from "./logLevels";

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

function formatTooltip(line: LogLine): string {
  const fields = line.fields.map((f) => `${f.key}=${f.value}`).join(" ");
  return [line.time, line.level, line.target, line.message, fields]
    .filter(Boolean)
    .join(" ");
}

const CARD_BORDER: Record<string, string> = {
  ERROR: "border-destructive/40",
  WARN: "border-warning/40",
};

const MESSAGE_TONE: Record<string, string> = {
  ERROR: "text-destructive",
  WARN: "text-warning",
};

/** 两行内 inline 流式展示：meta · target · message · fields（超出 line-clamp-2，title 补全）。 */
export function LogRow({ line, keyword }: { line: LogLine; keyword: string }) {
  const variant = LEVEL_VARIANT[line.level] ?? "muted";
  const dot = LEVEL_DOT[line.level] ?? "idle";
  const tone = MESSAGE_TONE[line.level];
  const body = highlight(line.message, keyword);

  return (
    <article
      title={formatTooltip(line)}
      className={cn(
        "rounded border bg-surface-card px-2 py-1",
        CARD_BORDER[line.level] ?? "border-edge-subtle",
      )}
    >
      <div className="line-clamp-2 text-[11px] leading-snug break-all">
        <span className="mr-1 inline-flex items-center gap-1 align-baseline whitespace-nowrap">
          <StatusDot status={dot} className="size-1.5 shrink-0" />
          <Badge
            variant={variant}
            className="h-3.5 shrink-0 px-1 py-0 text-[9px] leading-none uppercase"
          >
            {line.level}
          </Badge>
          <TabularText className="text-ink-mute">{line.time}</TabularText>
        </span>

        {line.target ? (
          <>
            <span className="text-ink-disabled">· </span>
            <span className="font-mono text-ink-secondary">{line.target}</span>
            <span className="text-ink-disabled"> · </span>
          </>
        ) : null}

        <span className={cn("text-ink-primary", tone)}>{body}</span>

        {line.fields.map((f) => (
          <span
            key={f.key}
            className="font-mono text-ink-mute before:content-['\00a0']"
          >
            {f.key}=<span className="text-ink-secondary">{f.value}</span>
          </span>
        ))}
      </div>
    </article>
  );
}
