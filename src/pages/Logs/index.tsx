import { useEffect, useMemo, useRef, useState } from "react";
import { ArrowDownIcon } from "lucide-react";
import { toast } from "sonner";

import { RequestMonitor } from "@/components/business/RequestMonitor";
import { PageLayoutEditor } from "@/components/business/page-layout/PageLayoutEditor";
import { PageSectionHost } from "@/components/business/page-layout/PageSectionHost";
import { Button } from "@/components/ui/button";
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
  const [selectedRequestLog, setSelectedRequestLog] = useState<RequestLog | null>(null);
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

  const handleSelectRequestLog = (nextLog: RequestLog | null) => {
    setSelectedRequestLog((previous) => {
      if (nextLog == null) {
        return previous == null ? previous : null;
      }
      return previous === nextLog ? previous : nextLog;
    });
  };

  return (
    <div className="flex h-full w-full min-w-0 flex-col gap-4">
      <PageLayoutEditor view="logs" definition={logsLayoutDefinition} />
      <PageSectionHost
        layout={layout}
        registry={{
          header: {
            title: "标题",
            render: () => <h1 className="text-2xl font-light tracking-tight">日志</h1>,
          },
          toolbar: {
            title: "日志工具栏",
            render: () => (
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
            ),
          },
          requests: {
            title: "最近请求",
            className: "rounded-lg border border-edge bg-surface p-4",
            modeClassName: {
              split: "xl:col-span-5",
            },
            render: () => (
              <RequestMonitor
                mode="live"
                pageSize={12}
                title="最近请求"
                selectedLogId={selectedRequestLog?.id ?? null}
                autoSelectFirst
                onSelectLog={handleSelectRequestLog}
              />
            ),
          },
          trace: {
            title: "请求四阶段详情",
            className: "rounded-lg border border-edge bg-surface p-4",
            modeClassName: {
              split: "xl:col-span-7",
            },
            render: () => <RequestTracePanel log={selectedRequestLog} />,
          },
          stream: {
            title: "日志流",
            className: "min-h-0 flex-1",
            modeClassName: {
              split: "xl:col-span-12",
            },
            render: () => (
              <div className="relative flex-1 overflow-hidden">
                <div
                  ref={scrollRef}
                  onScroll={onScroll}
                  className="h-full overflow-y-auto rounded-lg border border-edge bg-surface-raised p-3 font-mono text-xs"
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
    </div>
  );
}
