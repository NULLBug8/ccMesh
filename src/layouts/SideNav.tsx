import {
  ChevronLeftIcon,
  ChevronRightIcon,
  LogOutIcon,
  PanelTopIcon,
  PanelsTopLeftIcon,
} from "lucide-react";

import { Button } from "@/components/ui/button";
import { ThemeToggle, Logo, LangToggle } from "@/components/common";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { cn } from "@/lib/utils";
import { useLayoutStore, usePageLayoutStore } from "@/stores";
import { NavItem } from "./NavItem";
import { NAV_ITEMS, SETTINGS_ITEM } from "./navConfig";

async function logout() {
  window.location.href = "/__auth/logout";
}

export function SideNav() {
  const sidebarState = useLayoutStore((s) => s.sidebarState);
  const toggleSidebar = useLayoutStore((s) => s.toggleSidebar);
  const setNavMode = useLayoutStore((s) => s.setNavMode);
  const activeView = useLayoutStore((s) => s.activeView);
  const toggleEditMode = usePageLayoutStore((s) => s.toggleEditMode);
  const isEditing = usePageLayoutStore((s) => s.isEditing(activeView));
  const collapsed = sidebarState === "collapsed";

  return (
    <nav
      className={cn(
        "flex shrink-0 flex-col border-r border-edge bg-surface transition-[width] duration-200 ease-in-out",
        collapsed ? "w-14" : "w-[220px]",
      )}
    >
      <div className="flex h-14 shrink-0 items-center border-b border-edge-subtle px-4">
        <Logo iconOnly={collapsed} />
      </div>

      <div className="flex-1 overflow-y-auto px-2 py-2">
        <div className="flex flex-col gap-1">
          {NAV_ITEMS.map((item) => (
            <NavItem key={item.id} item={item} variant="vertical" collapsed={collapsed} />
          ))}
        </div>
      </div>

      <div className="flex flex-col gap-1 border-t border-edge px-2 py-2">
        <NavItem item={SETTINGS_ITEM} variant="vertical" collapsed={collapsed} />
        <div
          className={cn(
            "flex gap-1 pt-1",
            collapsed ? "flex-col items-center" : "items-center justify-between",
          )}
        >
          <div className={cn("flex gap-1", collapsed && "flex-col")}>
            <ThemeToggle />
            <LangToggle />
          </div>
          <div className={cn("flex gap-1", collapsed && "flex-col")}>
            <Tooltip>
              <TooltipTrigger asChild>
                <Button
                  variant={isEditing ? "secondary" : "ghost"}
                  size="icon"
                  aria-label="切换布局编辑"
                  onClick={() => toggleEditMode(activeView)}
                >
                  <PanelsTopLeftIcon className="size-4" />
                </Button>
              </TooltipTrigger>
              <TooltipContent side="right">
                {isEditing ? "退出布局编辑" : "进入布局编辑"}
              </TooltipContent>
            </Tooltip>
            <Tooltip>
              <TooltipTrigger asChild>
                <Button
                  variant="ghost"
                  size="icon"
                  aria-label="切换为顶部导航"
                  onClick={() => setNavMode("horizontal")}
                >
                  <PanelTopIcon className="size-4" />
                </Button>
              </TooltipTrigger>
              <TooltipContent side="right">切换为顶部导航</TooltipContent>
            </Tooltip>
            <Tooltip>
              <TooltipTrigger asChild>
                <Button
                  variant="ghost"
                  size="icon"
                  aria-label={collapsed ? "展开侧边栏" : "折叠侧边栏"}
                  onClick={toggleSidebar}
                >
                  {collapsed ? (
                    <ChevronRightIcon className="size-4" />
                  ) : (
                    <ChevronLeftIcon className="size-4" />
                  )}
                </Button>
              </TooltipTrigger>
              <TooltipContent side="right">
                {collapsed ? "展开侧边栏" : "折叠侧边栏"}
              </TooltipContent>
            </Tooltip>
            <Tooltip>
              <TooltipTrigger asChild>
                <Button
                  variant="outline"
                  size={collapsed ? "icon" : "sm"}
                  aria-label="退出登录"
                  onClick={logout}
                  className={cn(
                    "border-danger/40 text-danger hover:bg-danger/10 hover:text-danger",
                    !collapsed && "px-2.5",
                  )}
                >
                  <LogOutIcon className="size-4" />
                  {!collapsed ? <span className="ml-1.5">退出</span> : null}
                </Button>
              </TooltipTrigger>
              <TooltipContent side="right">退出登录</TooltipContent>
            </Tooltip>
          </div>
        </div>
      </div>
    </nav>
  );
}
