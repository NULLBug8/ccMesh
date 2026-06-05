import { StatusDot } from "@/components/ui";

const MAP: Record<string, { dot: "success" | "danger" | "idle"; label: string }> = {
  available: { dot: "success", label: "可用" },
  unavailable: { dot: "danger", label: "不可用" },
  unknown: { dot: "idle", label: "未测试" },
};

export function TestBadge({ status }: { status: string }) {
  const m = MAP[status] ?? MAP.unknown;
  return (
    <span className="flex items-center gap-1.5 text-xs text-ink-secondary">
      <StatusDot status={m.dot} />
      {m.label}
    </span>
  );
}
