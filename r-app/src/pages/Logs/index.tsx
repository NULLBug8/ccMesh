import { useEffect, useMemo, useRef, useState } from "react";
import { ArrowDownIcon } from "lucide-react";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import { configApi } from "@/services/modules/config";
import { logsApi, type LogLine } from "@/services/modules/logs";
import { LogRow } from "./_components/LogRow";
import { LogToolbar } from "./_components/LogToolbar";

const BOTTOM_THRESHOLD = 24;

export function Logs() {
  const [lines, setLines] = useState<LogLine[]>([]);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [keyword, setKeyword] = useState("");
  const [captureLevel, setCaptureLevel] = useState("info");
  const [atBottom, setAtBottom] = useState(true);
  const scrollRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    logsApi.recent().then(setLines).catch(() => undefined);
    logsApi
      .onLine((l) => setLines((prev) => [...prev.slice(-499), l]))
      .then((u) => {
        unlisten = u;
      });
    configApi
      .getConfig()
      .then((c) => setCaptureLevel(c.logLevel || "info"))
      .catch(() => undefined);
    return () => unlisten?.();
  }, []);

  // 贴底自动滚动：仅当用户停留在底部时跟随新日志
  useEffect(() => {
    if (atBottom && scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [lines, atBottom]);

  const onScroll = () => {
    const el = scrollRef.current;
    if (!el) return;
    setAtBottom(el.scrollHeight - el.scrollTop - el.clientHeight < BOTTOM_THRESHOLD);
  };

  const counts = useMemo(() => {
    const c: Record<string, number> = {};
    for (const l of lines) c[l.level] = (c[l.level] ?? 0) + 1;
    return c;
  }, [lines]);

  const filtered = useMemo(() => {
    const kw = keyword.trim().toLowerCase();
    return lines.filter((l) => {
      if (selected.size > 0 && !selected.has(l.level)) return false;
      if (kw) {
        const fields = l.fields.map((f) => `${f.key}=${f.value}`).join(" ");
        const hay = `${l.message} ${l.target} ${fields}`.toLowerCase();
        if (!hay.includes(kw)) return false;
      }
      return true;
    });
  }, [lines, selected, keyword]);

  const toggleLevel = (lvl: string) =>
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(lvl)) next.delete(lvl);
      else next.add(lvl);
      return next;
    });

  const changeCapture = (l: string) => {
    setCaptureLevel(l);
    logsApi.setLevel(l).catch(() => toast.error("设置捕获等级失败"));
  };

  const copyAll = async () => {
    const text = filtered
      .map((l) => {
        const fields = l.fields.length
          ? " " + l.fields.map((f) => `${f.key}=${f.value}`).join(" ")
          : "";
        return `${l.time} ${l.level} ${l.target} ${l.message}${fields}`;
      })
      .join("\n");
    try {
      await navigator.clipboard.writeText(text);
      toast.success(`已复制 ${filtered.length} 行`);
    } catch {
      toast.error("复制失败");
    }
  };

  const jumpToBottom = () => {
    const el = scrollRef.current;
    if (el) el.scrollTop = el.scrollHeight;
    setAtBottom(true);
  };

  return (
    <div className="mx-auto flex h-full max-w-4xl flex-col gap-4">
      <h1 className="text-2xl font-light tracking-tight">日志</h1>

      <LogToolbar
        selected={selected}
        onToggleLevel={toggleLevel}
        onShowAll={() => setSelected(new Set())}
        counts={counts}
        total={lines.length}
        keyword={keyword}
        onKeyword={setKeyword}
        captureLevel={captureLevel}
        onCaptureLevel={changeCapture}
        onCopy={copyAll}
        onClear={() => setLines([])}
      />

      <div className="relative flex-1 overflow-hidden">
        <div
          ref={scrollRef}
          onScroll={onScroll}
          className="h-full overflow-y-auto rounded-lg border border-edge bg-surface-raised p-3 font-mono text-xs"
        >
          {filtered.length === 0 ? (
            <p className="text-ink-mute">
              {lines.length === 0 ? "暂无日志" : "无匹配日志"}
            </p>
          ) : (
            filtered.map((l, i) => (
              <LogRow key={`${l.time}-${i}`} line={l} keyword={keyword} />
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
            <ArrowDownIcon className="size-4" /> 回到底部
          </Button>
        )}
      </div>
    </div>
  );
}
