import type { RequestLog, RequestTraceStage } from "@/services/modules/stats";

const STAGES: Array<{ key: keyof NonNullable<RequestLog["trace"]>; title: string }> = [
  { key: "receivedRequest", title: "接收请求" },
  { key: "forwardRequest", title: "转发请求" },
  { key: "receivedForwardedRequest", title: "接收转发的请求" },
  { key: "responseRequest", title: "响应请求" },
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

function StageCard({ title, stage }: { title: string; stage: RequestTraceStage }) {
  return (
    <section className="rounded-lg border border-edge bg-surface-raised/40 p-4">
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
  if (!log?.trace) {
    return (
      <section className="rounded-lg border border-dashed border-edge bg-surface-raised/20 p-5">
        <h2 className="text-sm font-medium text-foreground">请求四阶段详情</h2>
        <p className="mt-2 text-sm text-ink-mute">
          选择一条请求后，可以在这里查看四段详细链路。
        </p>
      </section>
    );
  }

  const trace = log.trace;

  return (
    <div className="flex flex-col gap-4">
      <div className="flex flex-col gap-1">
        <h2 className="text-sm font-medium text-foreground">请求四阶段详情</h2>
        <p className="text-xs text-ink-mute">
          {log.endpointName}
          {log.model ? ` · ${log.model}` : ""}
          {log.actualModel ? ` -> ${log.actualModel}` : ""}
        </p>
      </div>

      <div className="grid gap-4 xl:grid-cols-2">
        {STAGES.map((entry) => (
          <StageCard key={entry.key} title={entry.title} stage={trace[entry.key]} />
        ))}
      </div>
    </div>
  );
}
