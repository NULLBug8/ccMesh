import type { ReactNode } from "react";

import { cn } from "@/lib/utils";

export function SettingsGrid({
  children,
  className,
}: {
  children: ReactNode;
  className?: string;
}) {
  return (
    <div className={cn("grid grid-cols-1 gap-4 lg:grid-cols-2", className)}>
      {children}
    </div>
  );
}
