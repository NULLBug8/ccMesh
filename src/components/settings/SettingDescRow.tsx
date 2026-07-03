import type { ReactNode } from "react";

export function SettingDescRow({
  title,
  desc,
  children,
}: {
  title: string;
  desc: string;
  children: ReactNode;
}) {
  return (
    <div className="flex items-center justify-between gap-3">
      <div className="flex min-w-0 flex-col gap-0.5">
        <span className="text-sm text-ink-primary">{title}</span>
        <span className="text-xs text-ink-mute">{desc}</span>
      </div>
      {children}
    </div>
  );
}
