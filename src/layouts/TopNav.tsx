import { PanelLeftIcon, PanelsTopLeftIcon } from "lucide-react";

import { Button } from "@/components/ui/button";
import { ThemeToggle, Logo, LangToggle } from "@/components/common";
import { useLayoutStore, usePageLayoutStore } from "@/stores";
import { NavItem } from "./NavItem";
import { NAV_ITEMS, SETTINGS_ITEM } from "./navConfig";

export function TopNav() {
  const setNavMode = useLayoutStore((s) => s.setNavMode);
  const activeView = useLayoutStore((s) => s.activeView);
  const toggleEditMode = usePageLayoutStore((s) => s.toggleEditMode);
  const isEditing = usePageLayoutStore((s) => s.isEditing(activeView));

  return (
    <header className="flex h-14 shrink-0 items-center gap-4 border-b border-edge bg-surface px-6">
      <div className="w-[160px] shrink-0">
        <Logo />
      </div>

      <nav className="flex flex-1 items-center gap-1 overflow-x-auto">
        {NAV_ITEMS.map((item) => (
          <NavItem key={item.id} item={item} variant="horizontal" />
        ))}
        <NavItem item={SETTINGS_ITEM} variant="horizontal" />
      </nav>

      <div className="flex shrink-0 items-center gap-2">
        <Button
          variant={isEditing ? "secondary" : "ghost"}
          size="icon"
          aria-label="切换布局编辑"
          onClick={() => toggleEditMode(activeView)}
        >
          <PanelsTopLeftIcon className="size-4" />
        </Button>
        <Button
          variant="outline"
          size="icon"
          aria-label="切换为侧边导航"
          onClick={() => setNavMode("vertical")}
        >
          <PanelLeftIcon className="size-4" />
        </Button>
        <ThemeToggle />
        <LangToggle />
      </div>
    </header>
  );
}