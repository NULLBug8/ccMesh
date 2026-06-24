import { cn } from "@/lib/utils";
import type { PageLayoutConfig, PageSectionRegistry } from "./pageLayoutTypes";

interface Props {
  layout: PageLayoutConfig;
  registry: PageSectionRegistry;
}

function containerClass(mode: PageLayoutConfig["mode"]): string {
  switch (mode) {
    case "two-column":
      return "grid gap-6 xl:grid-cols-2";
    case "split":
      return "grid gap-6 xl:grid-cols-12";
    case "stack":
    default:
      return "flex flex-col gap-6";
  }
}

export function PageSectionHost({ layout, registry }: Props) {
  return (
    <div className={containerClass(layout.mode)}>
      {layout.sections
        .filter((section) => section.visible && registry[section.id])
        .map((section) => {
          const entry = registry[section.id];
          return (
            <div
              key={section.id}
              className={cn(entry.className, entry.modeClassName?.[layout.mode])}
            >
              {entry.render()}
            </div>
          );
        })}
    </div>
  );
}
