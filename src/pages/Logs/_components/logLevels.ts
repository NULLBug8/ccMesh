/** 视图级日志等级（与后端 tracing Level 文本一致，大写）。 */
export const LOG_LEVELS = ["ERROR", "WARN", "INFO", "DEBUG", "TRACE"] as const;

/** 捕获等级（后端 set_log_level 接受的小写值）。 */
export const CAPTURE_LEVELS = ["trace", "debug", "info", "warn", "error"] as const;

/** Badge variant（shadcn Badge）。 */
export type LevelBadgeVariant = "danger" | "warning" | "info" | "muted";

export const LEVEL_VARIANT: Record<string, LevelBadgeVariant> = {
  ERROR: "danger",
  WARN: "warning",
  INFO: "info",
  DEBUG: "muted",
  TRACE: "muted",
};

/** StatusDot 语义色。 */
export const LEVEL_DOT: Record<
  string,
  "danger" | "warning" | "info" | "idle"
> = {
  ERROR: "danger",
  WARN: "warning",
  INFO: "info",
  DEBUG: "idle",
  TRACE: "idle",
};

/** @deprecated 卡片改用 Badge variant；toolbar chips 仍可用 LEVEL_BADGE */
export const LEVEL_BADGE: Record<string, string> = {
  ERROR: "bg-destructive/15 text-destructive",
  WARN: "bg-warning/15 text-warning",
  INFO: "bg-info/15 text-info",
  DEBUG: "bg-ink-mute/15 text-ink-mute",
  TRACE: "bg-ink-mute/10 text-ink-mute",
};
