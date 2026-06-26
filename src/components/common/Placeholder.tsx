import type { ReactNode } from "react";

import { Card, CardContent } from "@/components/ui/card";

export function Placeholder({
  title,
  description,
  children,
}: {
  title: string;
  description?: string;
  children?: ReactNode;
}) {
  return (
    <div className="flex w-full min-w-0 flex-col gap-6">
      <div className="flex flex-col gap-1">
        <h1 className="text-2xl font-light tracking-tight">{title}</h1>
        {description && (
          <p className="text-sm text-ink-secondary">{description}</p>
        )}
      </div>
      {children ?? (
        <Card>
          <CardContent className="flex h-64 items-center justify-center pt-6 text-sm text-ink-mute">
            建设中
          </CardContent>
        </Card>
      )}
    </div>
  );
}
