import { useEffect, useMemo, useRef, useState } from "react";
import { ArrowDownIcon } from "lucide-react";
import { toast } from "sonner";

import { RequestMonitor } from "@/components/business/RequestMonitor";
import { PageLayoutEditor } from "@/components/business/page-layout/PageLayoutEditor";
import { PageSectionHost } from "@/components/business/page-layout/PageSectionHost";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { configApi } from "@/services/modules/config";
import { logsApi, type LogLine } from "@/services/modules/logs";
import type { RequestLog } from "@/services/modules/stats";
import { resolveViewLayout, usePageLayoutStore } from "@/stores";
import { LogRow } from "./_components/LogRow";
import { LogToolbar } from "./_components/LogToolbar";
import { RequestTracePanel } from "./_components/RequestTracePanel";
import { logsLayoutDefinition } from "./layout";

const BOTTOM_THRESHOLD = 24;

export function Logs() {
  const [lines, setLines] = useState<LogLine[]>([]);
  const [selectedLevels, setSelectedLevels] = useState<Set<string>>(new Set());
  const [keyword, setKeyword] = useState("");
  const [captureLevel, setCaptureLevel] = useState("info");
  const [atBottom, setAtBottom] = useState(true);
  const [detailRequestLog, setDetailRequestLog] = useState<RequestLog | null>(null);
  const scrollRef = useRef<HTMLDivElement>(null);
  const savedLayout = usePageLayoutStore((state) => state.getLayout("logs"));
  const layout = resolveViewLayout(logsLayoutDefinition, savedLayout);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    logsApi.recent().then(setLines).catch(() => undefined);
    logsApi
      .onLine((line) => setLines((prev) => [...prev.slice(-499), line]))
      .then((cleanup) => {
        unlisten = cleanup;
      });
    configApi
      .getConfig()
      .then((config) => setCaptureLevel(config.logLevel || "info"))
      .catch(() => undefined);
    return () => unlisten?.();
  }, []);

  useEffect(() => {
    if (atBottom && scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [lines, atBottom]);

  const counts = useMemo(() => {
    const next: Record<string, number> = {};
    for (const line of lines) {
      next[line.level] = (next[line.level] ?? 0) + 1;
    }
    return next;
  }, [lines]);

  const filtered = useMemo(() => {
    const lowerKeyword = keyword.trim().toLowerCase();
    return lines.filter((line) => {
      if (selectedLevels.size > 0 && !selectedLevels.has(line.level)) return false;
      if (!lowerKeyword) return true;
      const fields = line.fields.map((field) => `${field.key}=${field.value}`).join(" ");
      const haystack = `${line.message} ${line.target} ${fields}`.toLowerCase();
      return haystack.includes(lowerKeyword);
    });
  }, [keyword, lines, selectedLevels]);

  const onScroll = () => {
    const element = scrollRef.current;
    if (!element) return;
    setAtBottom(
      element.scrollHeight - element.scrollTop - element.clientHeight < BOTTOM_THRESHOLD,
    );
  };

  const toggleLevel = (level: string) =>
    setSelectedLevels((prev) => {
      const next = new Set(prev);
      if (next.has(level)) {
        next.delete(level);
      } else {
        next.add(level);
      }
      return next;
    });

  const changeCapture = (level: string) => {
    setCaptureLevel(level);
    logsApi.setLevel(level).catch(() => toast.error("设置捕获等级失败"));
  };

  const copyAll = async () => {
    const text = filtered
      .map((line) => {
        const fields = line.fields.length
          ? ` ${line.fields.map((field) => `${field.key}=${field.value}`).join(" ")}`
          : "";
        return `${line.time} ${line.level} ${line.target} ${line.message}${fields}`;
      })
      .join("\n");

    try {
      if (navigator.clipboard?.writeText) {
        await navigator.clipboard.writeText(text);
      } else {
        const textarea = document.createElement("textarea");
        textarea.value = text;
        textarea.style.position = "fixed";
        textarea.style.opacity = "0";
        document.body.appendChild(textarea);
        textarea.select();
        document.execCommand("copy");
        document.body.removeChild(textarea);
      }
      toast.success(`已复制 ${filtered.length} 行`);
    } catch {
      toast.error("复制失败");
    }
  };

  const jumpToBottom = () => {
    const element = scrollRef.current;
    if (element) {
      element.scrollTop = element.scrollHeight;
    }
    setAtBottom(true);
  };

  const handleOpenRequestDetail = (nextLog: RequestLog | null) => {
    if (nextLog) setDetailRequestLog(nextLog);
  };

  return (
    <div className="flex h-full w-full min-w-0 flex-col gap-5">
      <PageLayoutEditor view="logs" definition={logsLayoutDefinition} />
      <PageSectionHost
        layout={layout}
        registry={{
          header: {
            title: "标题",
            className: "xl:col-span-12",
            modeClassName: {
              "two-column": "xl:col-span-2",
            },
            render: () => (
              <div className="flex flex-wrap items-center justify-between gap-4 rounded-2xl border border-edge bg-gradient-to-r from-surface-raised via-surface to-surface-raised/40 p-4 shadow-sm">
                <div className="flex flex-col gap-1">
                  <span className="text-[11px] font-medium tracking-[0.16em] text-ink-mute uppercase">
                    Request Observatory
                  </span>
                  <h1 className="text-xl font-light tracking-tight">日志观测台</h1>
                </div>
                <div className="flex min-w-0 flex-wrap items-center gap-2 text-xs">
                  <span className="rounded-full border border-edge-subtle bg-background/60 px-3 py-1 font-mono text-ink-secondary">
                    {detailRequestLog ? `#${detailRequestLog.id}` : "点击详情查看链路"}
                  </span>
                  {detailRequestLog ? (
                    <>
                      <span className="rounded-full border border-edge-subtle bg-background/60 px-3 py-1 text-ink-secondary">
                        {detailRequestLog.endpointName}
                      </span>
                      <span className="max-w-[34rem] truncate rounded-full border border-edge-subtle bg-background/60 px-3 py-1 font-mono text-ink-mute">
                        {detailRequestLog.inboundPath || "--"} →{" "}
                        {detailRequestLog.upstreamPath || "--"}
                      </span>
                    </>
                  ) : null}
                </div>
              </div>
            ),
          },
          toolbar: {
            title: "日志工具栏",
            className: "xl:col-span-12",
            modeClassName: {
              "two-column": "xl:col-span-2",
            },
            render: () => (
              <div className="rounded-2xl border border-edge bg-surface p-4">
                <LogToolbar
                  selected={selectedLevels}
                  onToggleLevel={toggleLevel}
                  onShowAll={() => setSelectedLevels(new Set())}
                  counts={counts}
                  total={lines.length}
                  keyword={keyword}
                  onKeyword={setKeyword}
                  captureLevel={captureLevel}
                  onCaptureLevel={changeCapture}
                  onCopy={copyAll}
                  onClear={() => setLines([])}
                />
              </div>
            ),
          },
          requests: {
            title: "最近请求",
            className: "min-h-[34rem] rounded-2xl border border-edge bg-surface p-4 shadow-sm",
            modeClassName: {
              split: "xl:col-span-12",
            },
            render: () => (
              <RequestMonitor
                mode="live"
                pageSize={12}
                title="最近请求"
                variant="table"
                selectionMode="button"
                selectedLogId={detailRequestLog?.id ?? null}
                autoSelectFirst={false}
                onSelectLog={handleOpenRequestDetail}
              />
            ),
          },
          stream: {
            title: "日志流",
            className: "min-h-0 flex-1",
            modeClassName: {
              "two-column": "xl:col-span-2",
              split: "xl:col-span-12",
            },
            render: () => (
              <div className="relative flex min-h-[20rem] flex-1 flex-col overflow-hidden rounded-2xl border border-edge bg-surface p-4">
                <div className="mb-3 flex items-center justify-between gap-3">
                  <div>
                    <h2 className="text-sm font-medium text-foreground">运行日志流</h2>
                    <p className="text-xs text-ink-mute">
                      辅助排查系统事件；请求链路优先看上方详情。
                    </p>
                  </div>
                  <span className="rounded-full border border-edge-subtle px-2 py-1 font-mono text-xs text-ink-secondary">
                    {filtered.length}/{lines.length}
                  </span>
                </div>
                <div
                  ref={scrollRef}
                  onScroll={onScroll}
                  className="min-h-0 flex-1 overflow-y-auto rounded-xl border border-edge bg-surface-raised p-3 font-mono text-xs"
                >
                  {filtered.length === 0 ? (
                    <p className="text-ink-mute">
                      {lines.length === 0 ? "暂无日志" : "没有匹配的日志"}
                    </p>
                  ) : (
                    filtered.map((line, index) => (
                      <LogRow key={`${line.time}-${index}`} line={line} keyword={keyword} />
                    ))
                  )}
                </div>
                {!atBottom && (
                  <Button
                    variant="secondary"
                    size="sm"
                    onClick={jumpToBottom}
                    className="absolute right-3 bottom-3 shadow-level-2"
                  >
                    <ArrowDownIcon className="size-4" />
                    回到底部
                  </Button>
                )}
              </div>
            ),
          },
        }}
      />
      <Dialog
        open={Boolean(detailRequestLog)}
        onOpenChange={(open) => {
          if (!open) setDetailRequestLog(null);
        }}
      >
        <DialogContent className="grid h-[calc(100dvh-2rem)] max-w-[calc(100vw-2rem)] grid-rows-[auto_minmax(0,1fr)] overflow-hidden p-0 sm:max-w-[calc(100vw-2rem)]">
          <DialogHeader className="shrink-0 border-b border-edge bg-surface/95 px-6 py-4 pr-14">
            <DialogTitle>请求四阶段详情</DialogTitle>
            {detailRequestLog ? (
              <p className="font-mono text-xs text-ink-mute">
                #{detailRequestLog.id} · {detailRequestLog.endpointName} ·{" "}
                {detailRequestLog.inboundPath || "--"} →{" "}
                {detailRequestLog.upstreamPath || "--"}
              </p>
            ) : null}
          </DialogHeader>
          <div className="min-h-0 overflow-auto p-5">
            <RequestTracePanel log={detailRequestLog} showSummary={false} />
          </div>
        </DialogContent>
      </Dialog>
    </div>
  );
}
