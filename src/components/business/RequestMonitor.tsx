import { useEffect, useMemo, useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { InfoIcon, TriangleAlertIcon } from "lucide-react";

import { StatusDot, TabularText } from "@/components/ui";
import { Button } from "@/components/ui/button";
import { Pagination } from "@/components/ui/Pagination";
import {
  HoverCard,
  HoverCardContent,
  HoverCardTrigger,
} from "@/components/ui/hover-card";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { formatDuration, formatTokenK } from "@/lib/format";
import { RANGE_OPTIONS, rangeMs, startOfTodayMs, type RangeKey } from "@/lib/range";
import { cn } from "@/lib/utils";
import { statsApi, type RequestLog } from "@/services/modules/stats";

type Mode = "live" | "ranged";

interface Props {
  mode: Mode;
  endpointFilter?: string;
  pageSize?: number;
  title?: string;
  variant?: "table" | "cards";
  selectionMode?: "row" | "button";
  selectedLogId?: number | null;
  autoSelectFirst?: boolean;
  onSelectLog?: (log: RequestLog | null) => void;
}

interface RequestLogTableProps {
  items: RequestLog[];
  selectedLogId?: number | null;
  onSelectLog?: (log: RequestLog) => void;
  selectionMode?: "row" | "button";
}

export function RequestMonitor({
  mode,
  endpointFilter,
  pageSize = 20,
  title,
  variant = "table",
  selectionMode = "row",
  selectedLogId,
  autoSelectFirst = false,
  onSelectLog,
}: Props) {
  const qc = useQueryClient();
  const [page, setPage] = useState(1);
  const [rangeKey, setRangeKey] = useState<RangeKey>("today");
  const todayStart = startOfTodayMs();
  const range = useMemo(
    () => (mode === "ranged" ? rangeMs(rangeKey, todayStart) : {}),
    [mode, rangeKey, todayStart],
  );

  const { data, isLoading } = useQuery({
    queryKey: [
      "request-logs",
      mode,
      range.startMs ?? null,
      range.endMs ?? null,
      endpointFilter ?? null,
      page,
      pageSize,
    ],
    queryFn: () =>
      statsApi.getRequestLogs({
        startMs: range.startMs,
        endMs: range.endMs,
        endpoint: endpointFilter,
        page,
        pageSize,
      }),
  });

  useEffect(() => {
    if (mode !== "live") return;
    let unlisten: (() => void) | undefined;
    statsApi
      .onRequestLogged(() => {
        if (page === 1) {
          qc.invalidateQueries({ queryKey: ["request-logs", "live"] });
        }
      })
      .then((cleanup) => {
        unlisten = cleanup;
      });
    return () => unlisten?.();
  }, [mode, page, qc]);

  const items = data?.items ?? [];
  const total = data?.total ?? 0;

  useEffect(() => {
    if (!onSelectLog) return;
    if (items.length === 0) {
      if (selectedLogId != null) onSelectLog(null);
      return;
    }
    if (!autoSelectFirst) return;

    if (selectedLogId == null) {
      onSelectLog(items[0]);
      return;
    }

    const matched = items.find((item) => item.id === selectedLogId);
    onSelectLog(matched ?? items[0]);
  }, [autoSelectFirst, items, onSelectLog, selectedLogId]);

  return (
    <section className="flex h-full min-w-0 flex-col gap-3">
      <div className="flex items-center justify-between gap-3">
        <h2 className="text-sm font-medium text-ink-secondary">
          {title ?? (mode === "live" ? "实时请求监控" : "端点请求记录")}
        </h2>
        {mode === "ranged" && (
          <Select
            value={rangeKey}
            onValueChange={(value) => {
              setRangeKey(value as RangeKey);
              setPage(1);
            }}
          >
            <SelectTrigger className="w-32">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {RANGE_OPTIONS.map((option) => (
                <SelectItem key={option.key} value={option.key}>
                  {option.label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        )}
      </div>

      {isLoading ? (
        <p className="text-sm text-ink-mute">加载中...</p>
      ) : (
        variant === "cards" ? (
          <RequestLogCards
            items={items}
            selectedLogId={selectedLogId}
            onSelectLog={onSelectLog ? (log) => onSelectLog(log) : undefined}
          />
        ) : (
          <RequestLogTable
            items={items}
            selectedLogId={selectedLogId}
            onSelectLog={onSelectLog ? (log) => onSelectLog(log) : undefined}
            selectionMode={selectionMode}
          />
        )
      )}

      {total > pageSize && (
        <Pagination
          page={page}
          pageSize={pageSize}
          total={total}
          onPageChange={setPage}
        />
      )}
    </section>
  );
}

export function RequestLogCards({
  items,
  selectedLogId,
  onSelectLog,
}: RequestLogTableProps) {
  if (items.length === 0) {
    return <p className="text-sm text-ink-mute">暂无请求记录</p>;
  }

  return (
    <div className="flex min-h-0 flex-1 flex-col gap-2 overflow-y-auto pr-1">
      {items.map((log) => (
        <RequestLogCard
          key={log.id || log.ts}
          log={log}
          selected={selectedLogId != null && log.id === selectedLogId}
          onSelect={onSelectLog}
        />
      ))}
    </div>
  );
}

function RequestLogCard({
  log,
  selected,
  onSelect,
}: {
  log: RequestLog;
  selected: boolean;
  onSelect?: (log: RequestLog) => void;
}) {
  const total =
    log.inputTokens +
    log.outputTokens +
    log.cacheCreationTokens +
    log.cacheReadTokens;
  const selectable = Boolean(onSelect);
  const selectCard = () => onSelect?.(log);
  const status = log.statusCode ?? "ERR";

  return (
    <article
      aria-selected={selectable ? selected : undefined}
      data-testid={`request-log-card-${log.id || log.ts}`}
      tabIndex={selectable ? 0 : undefined}
      className={cn(
        "group rounded-xl border bg-background/45 p-3 transition-colors",
        selectable && "cursor-pointer hover:border-primary/40 hover:bg-surface-hover/60",
        selected
          ? "border-primary/60 bg-primary/10 shadow-sm ring-1 ring-primary/30"
          : "border-edge-subtle",
      )}
      onClick={selectable ? selectCard : undefined}
      onKeyDown={
        selectable
          ? (event) => {
              if (event.key === "Enter" || event.key === " ") {
                event.preventDefault();
                selectCard();
              }
            }
          : undefined
      }
    >
      <div className="flex items-start justify-between gap-3">
        <div className="min-w-0">
          <div className="flex items-center gap-2">
            <StatusDot status={statusDot(log.statusCode)} />
            <span className="truncate text-sm font-medium text-foreground">
              {log.endpointName}
            </span>
          </div>
          <div className="mt-1 font-mono text-[11px] text-ink-mute">
            <TabularText>{fmtDateTime(log.ts)}</TabularText>
          </div>
        </div>
        <span
          className={cn(
            "rounded-full border px-2 py-0.5 font-mono text-[11px]",
            log.isError || log.statusCode == null || log.statusCode >= 400
              ? "border-destructive/40 bg-destructive/10 text-destructive"
              : "border-success/40 bg-success/10 text-success",
          )}
        >
          {status}
        </span>
      </div>

      <div className="mt-3 space-y-1.5">
        <div className="min-w-0 rounded-lg border border-edge-subtle bg-surface/50 px-2 py-1.5">
          <div className="text-[10px] uppercase tracking-[0.08em] text-ink-mute">入站</div>
          <div className="truncate font-mono text-xs text-ink-secondary" title={log.inboundPath}>
            {log.inboundPath || inferPath(log.inboundFormat)}
          </div>
        </div>
        <div className="min-w-0 rounded-lg border border-edge-subtle bg-surface/50 px-2 py-1.5">
          <div className="text-[10px] uppercase tracking-[0.08em] text-ink-mute">出站</div>
          <div
            className="truncate font-mono text-xs text-ink-secondary"
            title={log.upstreamUrl ? `${log.upstreamUrl}${log.upstreamPath}` : undefined}
          >
            {log.upstreamPath || inferPath(log.inboundFormat)}
          </div>
        </div>
      </div>

      <div className="mt-3 grid grid-cols-3 gap-2 text-xs">
        <div>
          <div className="text-[10px] text-ink-mute">用时</div>
          <TabularText className="text-ink-secondary">
            {!log.isError && log.durationMs != null ? formatDuration(log.durationMs) : "--"}
          </TabularText>
        </div>
        <div>
          <div className="text-[10px] text-ink-mute">首字</div>
          <TabularText className="text-ink-secondary">
            {!log.isError && log.firstByteMs != null ? formatDuration(log.firstByteMs) : "--"}
          </TabularText>
        </div>
        <div>
          <div className="text-[10px] text-ink-mute">Token</div>
          <TabularText className="text-ink-secondary">{formatTokenK(total)}</TabularText>
        </div>
      </div>

      {log.isError && log.errorBody ? (
        <HoverCard openDelay={100} closeDelay={50}>
          <HoverCardTrigger asChild>
            <button
              type="button"
              className="mt-3 inline-flex items-center gap-1 text-xs text-warning hover:text-warning/80"
              onClick={(event) => event.stopPropagation()}
            >
              <TriangleAlertIcon className="size-3" />
              查看错误详情
            </button>
          </HoverCardTrigger>
          <HoverCardContent align="start" className="max-h-72 w-96 overflow-auto">
            <ErrorDetail errorBody={log.errorBody} />
          </HoverCardContent>
        </HoverCard>
      ) : null}
    </article>
  );
}

export function RequestLogTable({
  items,
  selectedLogId,
  onSelectLog,
  selectionMode = "row",
}: RequestLogTableProps) {
  if (items.length === 0) {
    return <p className="text-sm text-ink-mute">暂无请求记录</p>;
  }

  return (
    <div className="min-w-0 overflow-auto rounded-xl border border-edge bg-background/35">
      <table className="w-full min-w-[760px] text-sm">
        <thead>
          <tr className="border-b border-edge text-xs text-ink-secondary">
            <th className="px-3 py-2 text-left font-medium">时间</th>
            <th className="px-3 py-2 text-left font-medium">端点</th>
            <th className="px-3 py-2 text-left font-medium">入站</th>
            <th className="px-3 py-2 text-left font-medium">出站</th>
            <th className="w-[5.5rem] px-3 py-2 text-left font-medium">状态</th>
            <th className="px-3 py-2 text-right font-medium">用时</th>
            <th className="px-3 py-2 text-right font-medium">首字</th>
            <th className="px-3 py-2 text-right font-medium">Token</th>
            {selectionMode === "button" ? (
              <th className="w-[5rem] px-3 py-2 text-right font-medium">详情</th>
            ) : null}
          </tr>
        </thead>
        <tbody>
          {items.map((log) => (
            <RequestRow
              key={log.id || log.ts}
              log={log}
              selected={selectedLogId != null && log.id === selectedLogId}
              onSelect={onSelectLog}
              selectionMode={selectionMode}
            />
          ))}
        </tbody>
      </table>
    </div>
  );
}

export function fmtTime(ts: number): string {
  const date = new Date(ts);
  const pad = (value: number) => String(value).padStart(2, "0");
  return `${pad(date.getHours())}:${pad(date.getMinutes())}:${pad(date.getSeconds())}`;
}

export function fmtDate(ts: number): string {
  const date = new Date(ts);
  const pad = (value: number) => String(value).padStart(2, "0");
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}`;
}

export function fmtDateTime(ts: number): string {
  return `${fmtDate(ts)} ${fmtTime(ts)}`;
}

function statusDot(code: number | null): "success" | "warning" | "danger" {
  if (code == null) return "danger";
  if (code < 300) return "success";
  if (code < 400) return "warning";
  return "danger";
}

export function formatErrorBody(errorBody: string): string {
  try {
    return JSON.stringify(JSON.parse(errorBody), null, 2);
  } catch {
    return errorBody;
  }
}

export function ErrorDetail({ errorBody }: { errorBody: string }) {
  return (
    <div className="flex flex-col gap-2">
      <div className="text-sm font-medium">错误详情</div>
      <pre className="whitespace-pre-wrap break-words font-mono text-xs text-ink-secondary">
        {formatErrorBody(errorBody)}
      </pre>
    </div>
  );
}

function inferPath(format: string): string {
  if (format === "openai") return "/v1/chat/completions";
  if (format === "responses") return "/v1/responses";
  if (format === "claude") return "/v1/messages";
  return "--";
}

function RequestRow({
  log,
  selected,
  onSelect,
  selectionMode = "row",
}: {
  log: RequestLog;
  selected: boolean;
  onSelect?: (log: RequestLog) => void;
  selectionMode?: "row" | "button";
}) {
  const total =
    log.inputTokens +
    log.outputTokens +
    log.cacheCreationTokens +
    log.cacheReadTokens;
  const rowSelectable = Boolean(onSelect) && selectionMode !== "button";

  const selectRow = () => onSelect?.(log);

  return (
    <tr
      aria-selected={rowSelectable ? selected : undefined}
      data-testid={`request-log-row-${log.id || log.ts}`}
      tabIndex={rowSelectable ? 0 : undefined}
      className={cn(
        "border-b border-edge-subtle last:border-0",
        rowSelectable && "cursor-pointer transition-colors hover:bg-surface-hover/50",
        rowSelectable && selected && "bg-info/20 ring-1 ring-inset ring-info/40",
      )}
      onClick={rowSelectable ? selectRow : undefined}
      onKeyDown={
        rowSelectable
          ? (event) => {
              if (event.key === "Enter" || event.key === " ") {
                event.preventDefault();
                selectRow();
              }
            }
          : undefined
      }
    >
      <td className="px-3 py-2 whitespace-nowrap" title={new Date(log.ts).toLocaleString()}>
        <TabularText>{fmtDateTime(log.ts)}</TabularText>
      </td>
      <td className="px-3 py-2">{log.endpointName}</td>
      <td
        className="px-3 py-2 font-mono text-xs text-ink-secondary"
        title={`入站协议：${log.inboundFormat}`}
      >
        {log.inboundPath || inferPath(log.inboundFormat)}
      </td>
      <td
        className="max-w-[200px] truncate px-3 py-2 font-mono text-xs text-ink-secondary"
        title={log.upstreamUrl ? `${log.upstreamUrl}${log.upstreamPath}` : undefined}
      >
        {log.upstreamPath || inferPath(log.inboundFormat)}
      </td>
      <td className="w-[5.5rem] px-3 py-2">
        <div className="flex items-center justify-between">
          <span className="inline-flex items-center gap-1.5">
            <StatusDot status={statusDot(log.statusCode)} />
            <TabularText className="w-8 text-left text-xs text-ink-secondary">
              {log.statusCode ?? "ERR"}
            </TabularText>
          </span>
          {log.isError && log.errorBody ? (
            <HoverCard openDelay={100} closeDelay={50}>
              <HoverCardTrigger asChild>
                <button
                  type="button"
                  aria-label="查看错误详情"
                  title="查看错误详情"
                  className="inline-flex shrink-0 items-center text-warning/60 transition-colors hover:text-warning"
                  onClick={(event) => event.stopPropagation()}
                >
                  <TriangleAlertIcon className="size-3" />
                </button>
              </HoverCardTrigger>
              <HoverCardContent align="center" className="max-h-72 w-96 overflow-auto">
                <ErrorDetail errorBody={log.errorBody} />
              </HoverCardContent>
            </HoverCard>
          ) : (
            <span className="inline-block size-3 shrink-0" aria-hidden="true" />
          )}
        </div>
      </td>
      <td className="px-3 py-2 text-right text-xs text-ink-secondary">
        <TabularText>
          {!log.isError && log.durationMs != null ? formatDuration(log.durationMs) : "--"}
        </TabularText>
      </td>
      <td className="px-3 py-2 text-right text-xs text-ink-secondary">
        <TabularText>
          {!log.isError && log.firstByteMs != null ? formatDuration(log.firstByteMs) : "--"}
        </TabularText>
      </td>
      <td className="px-3 py-2 text-right">
        <HoverCard openDelay={100} closeDelay={50}>
          <HoverCardTrigger asChild>
            <button
              type="button"
              className="inline-flex items-center gap-1 text-ink-secondary transition-colors hover:text-foreground"
              onClick={(event) => event.stopPropagation()}
            >
              <TabularText>{total}</TabularText>
              <InfoIcon className="size-3.5" />
            </button>
          </HoverCardTrigger>
          <HoverCardContent align="end" className="w-56">
            <TokenDetail log={log} total={total} />
          </HoverCardContent>
        </HoverCard>
      </td>
      {selectionMode === "button" ? (
        <td className="px-3 py-2 text-right">
          <Button
            type="button"
            variant="outline"
            size="sm"
            aria-label="查看请求详情"
            onClick={(event) => {
              event.stopPropagation();
              onSelect?.(log);
            }}
          >
            详情
          </Button>
        </td>
      ) : null}
    </tr>
  );
}

export function TokenDetail({ log, total }: { log: RequestLog; total: number }) {
  const rows: [string, number][] = [
    ["输入", log.inputTokens],
    ["输出", log.outputTokens],
    ["缓存创建", log.cacheCreationTokens],
    ["缓存读取", log.cacheReadTokens],
  ];

  return (
    <div className="flex flex-col gap-1.5 text-xs">
      {log.model && (
        <div className="truncate text-ink-secondary" title={log.model}>
          模型：{log.model}
        </div>
      )}
      {log.actualModel && (
        <div title={log.actualModel} className="text-ink-secondary">
          实际模型：<span className="truncate text-info">{log.actualModel}</span>
        </div>
      )}
      {rows.map(([label, value]) => (
        <div key={label} className="flex items-center justify-between gap-4">
          <span className="text-ink-secondary">{label}</span>
          <span title={value.toLocaleString()}>
            <TabularText>{formatTokenK(value)}</TabularText>
          </span>
        </div>
      ))}
      <div className="mt-1 flex items-center justify-between gap-4 border-t border-edge-subtle pt-1.5 font-medium">
        <span>合计</span>
        <span title={total.toLocaleString()}>
          <TabularText>{formatTokenK(total)}</TabularText>
        </span>
      </div>
      {!log.isError && log.firstByteMs != null && (
        <div className="flex items-center justify-between gap-4 text-ink-secondary">
          <span>首字节</span>
          <TabularText>{formatDuration(log.firstByteMs)}</TabularText>
        </div>
      )}
      {!log.isError && log.durationMs != null && (
        <div className="flex items-center justify-between gap-4 text-ink-secondary">
          <span>耗时</span>
          <TabularText>{formatDuration(log.durationMs)}</TabularText>
        </div>
      )}
    </div>
  );
}
