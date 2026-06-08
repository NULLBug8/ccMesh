/** 视图级日志等级（与后端 tracing Level 文本一致，大写）。 */
export const LOG_LEVELS = ["ERROR", "WARN", "INFO", "DEBUG", "TRACE"] as const;

/** 捕获等级（后端 set_log_level 接受的小写值）。 */
export const CAPTURE_LEVELS = ["trace", "debug", "info", "warn", "error"] as const;

/** 等级徽章配色（背景弱化 + 文字主色）。 */
export const LEVEL_BADGE: Record<string, string> = {
  ERROR: "bg-destructive/15 text-destructive",
  WARN: "bg-warning/15 text-warning",
  INFO: "bg-info/15 text-info",
  DEBUG: "bg-ink-mute/15 text-ink-mute",
  TRACE: "bg-ink-mute/10 text-ink-mute",
};
