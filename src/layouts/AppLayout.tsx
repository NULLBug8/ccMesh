import { useEffect, type ReactNode } from "react";

import { cn } from "@/lib/utils";
import { useLayoutStore, type ViewId } from "@/stores";
import { Dashboard } from "@/pages/Dashboard";
import { Endpoints } from "@/pages/Endpoints";
import { Statistics } from "@/pages/Statistics";
import { Sync } from "@/pages/Sync";
import { Logs } from "@/pages/Logs";
import { Settings } from "@/pages/Settings";
import { TopNav } from "./TopNav";
import { SideNav } from "./SideNav";
import { TitleBar } from "./TitleBar";

const VIEW_MAP: Record<ViewId, ReactNode> = {
  dashboard: <Dashboard />,
  endpoints: <Endpoints />,
  statistics: <Statistics />,
  sync: <Sync />,
  logs: <Logs />,
  settings: <Settings />,
};

export function AppLayout() {
  const navMode = useLayoutStore((s) => s.navMode);
  const activeView = useLayoutStore((s) => s.activeView);

  useEffect(() => {
    const mql = window.matchMedia("(max-width: 1024px)");
    const handler = (e: MediaQueryListEvent) => {
      const store = useLayoutStore.getState();
      if (store.navMode === "vertical" && e.matches) {
        store.setSidebarState("collapsed");
      }
    };
    mql.addEventListener("change", handler);
    return () => mql.removeEventListener("change", handler);
  }, []);

  return (
    <div className="flex h-screen w-screen flex-col overflow-hidden bg-background text-foreground">
      <TitleBar />
      <div
        className={cn(
          "flex flex-1 overflow-hidden",
          navMode === "vertical" ? "flex-row" : "flex-col"
        )}
      >
        {navMode === "horizontal" ? <TopNav /> : <SideNav />}
        <main className="flex-1 overflow-y-auto p-8">{VIEW_MAP[activeView]}</main>
      </div>
    </div>
  );
}
