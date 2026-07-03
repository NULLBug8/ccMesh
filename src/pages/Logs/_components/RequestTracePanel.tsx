import type { RequestLog, RequestTraceStage } from "@/services/modules/stats";

const STAGES: Array<{
  key: keyof NonNullable<RequestLog["trace"]>;
  title: string;
  step: string;
  tone: string;
  headerLabel: string;
  emptyHeaderText: string;
  bodyLabel: string;
  emptyBodyText: string;
}> = [
  {
    key: "receivedRequest",
    title: "接收请求",
    step: "01",
    tone: "border-sky-500/35 bg-sky-500/5 text-sky-500",
    headerLabel: "请求头",
    emptyHeaderText: "无请求头",
    bodyLabel: "请求体",
    emptyBodyText: "无请求体",
  },
  {
    key: "forwardRequest",
    title: "转发请求",
    step: "02",
    tone: "border-violet-500/35 bg-violet-500/5 text-violet-500",
    headerLabel: "请求头",
    emptyHeaderText: "无请求头",
    bodyLabel: "请求体",
    emptyBodyText: "无请求体",
  },
  {
    key: "receivedForwardedRequest",
    title: "接收上游响应",
    step: "03",
    tone: "border-emerald-500/35 bg-emerald-500/5 text-emerald-500",
    headerLabel: "响应头",
    emptyHeaderText: "无响应头",
    bodyLabel: "响应体",
    emptyBodyText: "无响应体",
  },
  {
    key: "responseRequest",
    title: "响应客户端",
    step: "04",
    tone: "border-cyan-500/35 bg-cyan-500/5 text-cyan-500",
    headerLabel: "响应头",
    emptyHeaderText: "无响应头",
    bodyLabel: "响应体",
    emptyBodyText: "无响应体",
  },
];

function ValueBlock({
  label,
  value,
  mono = false,
}: {
  label: string;
  value: string | number | null | undefined;
  mono?: boolean;
}) {
  if (value == null || value === "") return null;

  return (
    <div className="flex flex-col gap-1">
      <span className="text-[11px] uppercase tracking-[0.06em] text-ink-mute">{label}</span>
      <span className={mono ? "break-all font-mono text-xs text-ink-secondary" : "text-sm"}>
        {String(value)}
      </span>
    </div>
  );
}

function MissingStageCard({ title }: { title: string }) {
  return (
    <section className="rounded-xl border border-dashed border-edge bg-surface-raised/20 p-4">
      <div className="mb-3 flex items-center justify-between gap-3">
        <h3 className="text-sm font-medium text-foreground">{title}</h3>
        <span className="rounded-full border border-edge-subtle px-2 py-0.5 text-[11px] text-ink-mute">
          未记录
        </span>
      </div>
      <div className="grid gap-3 md:grid-cols-2">
        <ValueBlock label="方法" value="--" mono />
        <ValueBlock label="URL" value="--" mono />
      </div>
      <p className="mt-3 text-xs leading-5 text-ink-mute">
        这条请求没有保存该阶段的 headers/body。后续新请求会在代理记录到 trace
        时显示完整内容。
      </p>
    </section>
  );
}

function StageCard({
  title,
  step,
  tone,
  stage,
  headerLabel,
  emptyHeaderText,
  bodyLabel,
  emptyBodyText,
}: {
  title: string;
  step: string;
  tone: string;
  stage: RequestTraceStage;
  headerLabel: string;
  emptyHeaderText: string;
  bodyLabel: string;
  emptyBodyText: string;
}) {
  return (
    <section
      data-testid="request-trace-stage"
      className="grid grid-cols-[minmax(0,1fr)] gap-4 rounded-2xl border border-edge bg-surface-raised/40 p-4 shadow-sm"
    >
      <div className="flex flex-wrap items-start justify-between gap-3 border-b border-edge-subtle pb-3">
        <div className="flex min-w-0 items-start gap-3">
          <span
            className={`inline-flex size-8 shrink-0 items-center justify-center rounded-full border font-mono text-[11px] ${tone}`}
          >
            {step}
          </span>
          <div className="min-w-0">
            <h3 className="text-base font-medium text-foreground">{title}</h3>
            <p className="mt-1 truncate font-mono text-xs text-ink-mute">
              {stage.url || "--"}
            </p>
          </div>
        </div>
        <div className="flex shrink-0 flex-wrap items-center gap-2">
          {stage.method ? (
            <span className="rounded-full border border-edge-subtle px-2 py-0.5 font-mono text-[11px] text-ink-secondary">
              {stage.method}
            </span>
          ) : null}
          {stage.statusCode != null && (
            <span className="rounded-full border border-edge-subtle px-2 py-0.5 font-mono text-[11px] text-ink-secondary">
              {stage.statusCode}
            </span>
          )}
        </div>
      </div>

      <div className="flex min-w-0 flex-col gap-4">
        <div className="flex flex-col gap-2">
          <span className="text-[11px] uppercase tracking-[0.06em] text-ink-mute">
            {headerLabel}
          </span>
          {stage.headers.length === 0 ? (
            <p className="text-xs text-ink-mute">{emptyHeaderText}</p>
          ) : (
            <div className="grid rounded-md border border-edge-subtle bg-background/60 md:grid-cols-2 xl:grid-cols-3">
              {stage.headers.map((header) => (
                <div
                  key={`${header.key}:${header.value}`}
                  className="grid gap-2 border-b border-edge-subtle px-3 py-2 md:grid-cols-[120px_1fr] md:border-r md:last:border-r-0 xl:[&:nth-child(3n)]:border-r-0"
                >
                  <span className="font-mono text-xs text-ink-secondary">{header.key}</span>
                  <span className="break-all font-mono text-xs">{header.value}</span>
                </div>
              ))}
            </div>
          )}
        </div>

        <div className="flex flex-col gap-2">
          <span className="text-[11px] uppercase tracking-[0.06em] text-ink-mute">
            {bodyLabel}
          </span>
          {stage.body ? (
            <pre className="max-h-[42rem] overflow-auto whitespace-pre-wrap break-words rounded-md border border-edge-subtle bg-background/60 p-4 font-mono text-xs leading-5 text-ink-secondary">
              {stage.body}
            </pre>
          ) : (
            <p className="text-xs text-ink-mute">{emptyBodyText}</p>
          )}
        </div>
      </div>
    </section>
  );
}

export function RequestTracePanel({
  log,
  showSummary = true,
}: {
  log: RequestLog | null;
  showSummary?: boolean;
}) {
  if (!log) {
    return (
      <section className="rounded-xl border border-dashed border-edge bg-surface-raised/20 p-6">
        <h2 className="text-sm font-medium text-foreground">选择一条请求查看链路</h2>
        <p className="mt-2 text-sm text-ink-mute">
          点击左侧最近请求后，这里会展示接收、转发、上游响应、客户端响应四段详情。
        </p>
      </section>
    );
  }

  const trace = log.trace;

  return (
    <div className="flex min-w-0 flex-col gap-4">
      {showSummary ? (
        <div className="rounded-xl border border-edge bg-surface-raised/60 p-4">
          <div className="flex flex-wrap items-start justify-between gap-3">
            <div className="flex min-w-0 flex-col gap-1">
              <h2 className="text-base font-medium text-foreground">请求四阶段详情</h2>
              <p className="truncate text-sm text-ink-secondary">{log.endpointName}</p>
              <p className="font-mono text-xs text-ink-mute">
                {log.inboundPath || "--"} → {log.upstreamPath || "--"}
              </p>
            </div>
            <div className="flex flex-wrap items-center gap-2 text-xs text-ink-secondary">
              <span className="rounded-full border border-edge-subtle px-2 py-1">
                HTTP {log.statusCode ?? "ERR"}
              </span>
              {log.model ? (
                <span className="rounded-full border border-edge-subtle px-2 py-1">
                  {log.model}
                </span>
              ) : null}
              {log.actualModel ? (
                <span className="rounded-full border border-info/40 bg-info/10 px-2 py-1 text-info">
                  → {log.actualModel}
                </span>
              ) : null}
            </div>
          </div>
          {!trace ? (
            <p className="mt-3 rounded-lg border border-warning/30 bg-warning/10 px-3 py-2 text-xs text-warning">
              未记录详细链路
            </p>
          ) : null}
        </div>
      ) : null}

      <div data-testid="request-trace-timeline" className="flex min-w-0 flex-col gap-4">
        {STAGES.map((entry) =>
          trace ? (
            <StageCard
              key={entry.key}
              title={entry.title}
              step={entry.step}
              tone={entry.tone}
              stage={trace[entry.key]}
              headerLabel={entry.headerLabel}
              emptyHeaderText={entry.emptyHeaderText}
              bodyLabel={entry.bodyLabel}
              emptyBodyText={entry.emptyBodyText}
            />
          ) : (
            <MissingStageCard key={entry.key} title={entry.title} />
          ),
        )}
      </div>
    </div>
  );
}
