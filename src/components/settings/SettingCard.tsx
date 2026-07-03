import type { LucideIcon } from "lucide-react";
import type { ReactNode } from "react";

import { cn } from "@/lib/utils";

export function SettingCard({
  icon: Icon,
  title,
  children,
  className,
}: {
  icon: LucideIcon;
  title: string;
  children: ReactNode;
  className?: string;
}) {
  return (
    <div
      className={cn(
        "rounded-lg border border-edge-subtle bg-surface-card p-6",
        className,
      )}
    >
      <h2 className="mb-5 flex items-center gap-2 text-base font-medium text-ink-primary">
        <Icon className="h-5 w-5 shrink-0" aria-hidden />
        {title}
      </h2>
      <div className="flex flex-col gap-4">{children}</div>
    </div>
  );
}
