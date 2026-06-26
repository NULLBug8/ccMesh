import type { RequestLog, RequestTraceStage } from "@/services/modules/stats";

const STAGES: Array<{ key: keyof NonNullable<RequestLog["trace"]>; title: string }> = [
  { key: "receivedRequest", title: "接收请求" },
  { key: "forwardRequest", title: "转发请求" },
  { key: "receivedForwardedRequest", title: "接收上游响应" },
  { key: "responseRequest", title: "响应客户端" },
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

function StageCard({ title, stage }: { title: string; stage: RequestTraceStage }) {
  return (
    <section className="rounded-xl border border-edge bg-surface-raised/40 p-4 shadow-sm">
      <div className="mb-3 flex items-start justify-between gap-3">
        <h3 className="text-sm font-medium text-foreground">{title}</h3>
        {stage.statusCode != null && (
          <span className="rounded-full border border-edge-subtle px-2 py-0.5 font-mono text-[11px] text-ink-secondary">
            {stage.statusCode}
          </span>
        )}
      </div>

      <div className="flex flex-col gap-3">
        <div className="grid gap-3 md:grid-cols-2">
          <ValueBlock label="方法" value={stage.method} mono />
          <ValueBlock label="URL" value={stage.url} mono />
        </div>

        <div className="flex flex-col gap-2">
          <span className="text-[11px] uppercase tracking-[0.06em] text-ink-mute">请求头</span>
          {stage.headers.length === 0 ? (
            <p className="text-xs text-ink-mute">无请求头</p>
          ) : (
            <div className="rounded-md border border-edge-subtle bg-background/60">
              {stage.headers.map((header) => (
                <div
                  key={`${header.key}:${header.value}`}
                  className="grid gap-2 border-b border-edge-subtle px-3 py-2 last:border-0 md:grid-cols-[160px_1fr]"
                >
                  <span className="font-mono text-xs text-ink-secondary">{header.key}</span>
                  <span className="break-all font-mono text-xs">{header.value}</span>
                </div>
              ))}
            </div>
          )}
        </div>

        <div className="flex flex-col gap-2">
          <span className="text-[11px] uppercase tracking-[0.06em] text-ink-mute">请求体</span>
          {stage.body ? (
            <pre className="max-h-52 overflow-auto rounded-md border border-edge-subtle bg-background/60 p-3 font-mono text-xs text-ink-secondary">
              {stage.body}
            </pre>
          ) : (
            <p className="text-xs text-ink-mute">无请求体</p>
          )}
        </div>
      </div>
    </section>
  );
}

export function RequestTracePanel({ log }: { log: RequestLog | null }) {
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

      <div className="grid min-w-0 gap-4 xl:grid-cols-2">
        {STAGES.map((entry) =>
          trace ? (
            <StageCard key={entry.key} title={entry.title} stage={trace[entry.key]} />
          ) : (
            <MissingStageCard key={entry.key} title={entry.title} />
          ),
        )}
      </div>
    </div>
  );
}
