import { cn } from "@/lib/utils";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { useLayoutStore } from "@/stores";
import type { NavItemDef } from "./navConfig";

export function NavItem({
  item,
  variant,
  collapsed = false,
}: {
  item: NavItemDef;
  variant: "horizontal" | "vertical";
  collapsed?: boolean;
}) {
  const activeView = useLayoutStore((s) => s.activeView);
  const setActiveView = useLayoutStore((s) => s.setActiveView);
  const lang = useLayoutStore((s) => s.lang);

  const isActive = activeView === item.id;
  const label = lang === "zh" ? item.label : item.labelEn;
  const Icon = item.icon;

  if (variant === "horizontal") {
    return (
      <button
        type="button"
        onClick={() => setActiveView(item.id)}
        className={cn(
          "inline-flex h-9 shrink-0 cursor-pointer items-center gap-2 rounded-full px-3 text-sm font-medium whitespace-nowrap transition-colors",
          isActive
            ? "bg-primary text-primary-foreground"
            : "text-ink-secondary hover:bg-surface-hover hover:text-ink-primary"
        )}
      >
        <Icon className="size-4" />
        {label}
      </button>
    );
  }

  const row = (
    <button
      type="button"
      onClick={() => setActiveView(item.id)}
      aria-label={label}
      className={cn(
        "flex h-10 w-full cursor-pointer items-center gap-2.5 rounded-sm text-sm transition-colors",
        collapsed ? "justify-center px-0" : "px-3",
        isActive
          ? "bg-primary/12 text-primary-soft"
          : "text-ink-secondary hover:bg-surface-hover hover:text-ink-primary"
      )}
    >
      <Icon className={cn("size-4 shrink-0", isActive && "text-primary")} />
      {!collapsed && <span className="whitespace-nowrap">{label}</span>}
    </button>
  );

  if (collapsed) {
    return (
      <Tooltip>
        <TooltipTrigger asChild>{row}</TooltipTrigger>
        <TooltipContent side="right">{label}</TooltipContent>
      </Tooltip>
    );
  }

  return row;
}
