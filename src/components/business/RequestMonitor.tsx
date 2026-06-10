import { useEffect, useMemo, useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { InfoIcon } from "lucide-react";

import { StatusDot, TabularText } from "@/components/ui";
import {
  HoverCard,
  HoverCardContent,
  HoverCardTrigger,
} from "@/components/ui/hover-card";
import { Pagination } from "@/components/ui/Pagination";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { RANGE_OPTIONS, rangeMs, startOfTodayMs, type RangeKey } from "@/lib/range";
import { formatDuration, formatTokenK } from "@/lib/format";
import { statsApi, type RequestLog } from "@/services/modules/stats";

type Mode = "live" | "ranged";

interface Props {
  /** live：事件驱动实时刷新；ranged：时间段 + 分页查询。 */
  mode: Mode;
  /** 可选端点过滤。 */
  endpointFilter?: string;
  pageSize?: number;
  /** 标题（默认按模式取）。 */
  title?: string;
}

/**
 * 端点请求实时监控（统计页 ranged / 仪表盘 live 复用）。
 * 数据统一走 `get_request_logs` 分页查询；live 模式在第 1 页时由 `request-logged` 事件触发刷新。
 */
export function RequestMonitor({ mode, endpointFilter, pageSize = 20, title }: Props) {
  const qc = useQueryClient();
  const [page, setPage] = useState(1);
  const [rangeKey, setRangeKey] = useState<RangeKey>("today");
  // 按天对齐的稳定锚点：同一天内多次渲染得到相同区间，避免 queryKey 逐帧漂移导致无限重取。
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

  // live：新请求事件 → 仅在第 1 页时刷新，避免打断翻页浏览
  useEffect(() => {
    if (mode !== "live") return;
    let un: (() => void) | undefined;
    statsApi
      .onRequestLogged(() => {
        if (page === 1) {
          qc.invalidateQueries({ queryKey: ["request-logs", "live"] });
        }
      })
      .then((u) => {
        un = u;
      });
    return () => un?.();
  }, [mode, page, qc]);

  const items = data?.items ?? [];
  const total = data?.total ?? 0;

  return (
    <section className="flex flex-col gap-3">
      <div className="flex items-center justify-between">
        <h2 className="text-sm font-medium text-ink-secondary">
          {title ?? (mode === "live" ? "实时请求监控" : "端点请求记录")}
        </h2>
        {mode === "ranged" && (
          <Select
            value={rangeKey}
            onValueChange={(v) => {
              setRangeKey(v as RangeKey);
              setPage(1);
            }}
          >
            <SelectTrigger className="w-32">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {RANGE_OPTIONS.map((r) => (
                <SelectItem key={r.key} value={r.key}>
                  {r.label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        )}
      </div>

      {isLoading ? (
        <p className="text-sm text-ink-mute">加载中…</p>
      ) : (
        <RequestLogTable items={items} />
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

/** 纯展示：请求明细表（空态自处理），便于复用与单测。 */
export function RequestLogTable({ items }: { items: RequestLog[] }) {
  if (items.length === 0) {
    return <p className="text-sm text-ink-mute">暂无请求记录</p>;
  }
  return (
    <div className="overflow-hidden rounded-lg border border-edge">
      <table className="w-full text-sm">
        <thead>
          <tr className="border-b border-edge text-xs text-ink-secondary">
            <th className="px-3 py-2 text-left font-medium">时间</th>
            <th className="px-3 py-2 text-left font-medium">端点</th>
            <th className="px-3 py-2 text-left font-medium">入站</th>
            <th className="px-3 py-2 text-left font-medium">出站</th>
            <th className="px-3 py-2 text-center font-medium">状态</th>
            <th className="px-3 py-2 text-right font-medium">用时</th>
            <th className="px-3 py-2 text-right font-medium">首字</th>
            <th className="px-3 py-2 text-right font-medium">Token</th>
          </tr>
        </thead>
        <tbody>
          {items.map((r) => (
            <RequestRow key={r.id || r.ts} log={r} />
          ))}
        </tbody>
      </table>
    </div>
  );
}

/** 请求时间按 24 小时制 时:分:秒（零填充）展示，避免地区设置带来的上午/下午前缀。 */
export function fmtTime(ts: number): string {
  const d = new Date(ts);
  const p = (n: number) => String(n).padStart(2, "0");
  return `${p(d.getHours())}:${p(d.getMinutes())}:${p(d.getSeconds())}`;
}

/** 请求日期 年-月-日（零填充，本地时区）。 */
export function fmtDate(ts: number): string {
  const d = new Date(ts);
  const p = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}`;
}

/** 请求时间完整展示：年-月-日 时:分:秒（24 小时制）。 */
export function fmtDateTime(ts: number): string {
  return `${fmtDate(ts)} ${fmtTime(ts)}`;
}

function statusDot(code: number | null): "success" | "warning" | "danger" {
  if (code == null) return "danger";
  if (code < 300) return "success";
  if (code < 400) return "warning";
  return "danger";
}

/** 旧行无真实路径时，按入站协议推断兜底路由。 */
function inferPath(format: string): string {
  if (format === "openai") return "/v1/chat/completions";
  if (format === "claude") return "/v1/messages";
  return "—";
}

function RequestRow({ log }: { log: RequestLog }) {
  const total =
    log.inputTokens +
    log.outputTokens +
    log.cacheCreationTokens +
    log.cacheReadTokens;
  return (
    <tr className="border-b border-edge-subtle last:border-0">
      <td
        className="px-3 py-2 whitespace-nowrap"
        title={new Date(log.ts).toLocaleString()}
      >
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
        title={
          log.upstreamUrl
            ? `${log.upstreamUrl}${log.upstreamPath}`
            : undefined
        }
      >
        {log.upstreamPath || inferPath(log.inboundFormat)}
      </td>
      <td className="px-3 py-2 text-center">
        <span className="inline-flex items-center gap-1.5">
          <StatusDot status={statusDot(log.statusCode)} />
          <TabularText className="text-xs">{log.statusCode ?? "ERR"}</TabularText>
        </span>
      </td>
      <td className="px-3 py-2 text-right text-xs text-ink-secondary">
        <TabularText>
          {!log.isError && log.durationMs != null ? formatDuration(log.durationMs) : "—"}
        </TabularText>
      </td>
      <td className="px-3 py-2 text-right text-xs text-ink-secondary">
        <TabularText>
          {!log.isError && log.firstByteMs != null ? formatDuration(log.firstByteMs) : "—"}
        </TabularText>
      </td>
      <td className="px-3 py-2 text-right">
        <HoverCard openDelay={100} closeDelay={50}>
          <HoverCardTrigger asChild>
            <button
              type="button"
              className="inline-flex items-center gap-1 text-ink-secondary transition-colors hover:text-foreground"
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
          实际模型：<span className=" truncate text-info">{log.actualModel}</span>
        </div>
      )}
      {rows.map(([k, v]) => (
        <div key={k} className="flex items-center justify-between gap-4">
          <span className="text-ink-secondary">{k}</span>
          <span title={v.toLocaleString()}>
            <TabularText>{formatTokenK(v)}</TabularText>
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
          <span>首字</span>
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
