import { create } from "zustand";
import { persist } from "zustand/middleware";

export type NavMode = "horizontal" | "vertical";
export type SidebarState = "expanded" | "collapsed";
export type ViewId =
  | "dashboard"
  | "endpoints"
  | "statistics"
  | "sync"
  | "logs"
  | "settings";
export type Lang = "zh" | "en";

interface LayoutState {
  navMode: NavMode;
  sidebarState: SidebarState;
  activeView: ViewId;
  lang: Lang;
  setNavMode: (mode: NavMode) => void;
  toggleNavMode: () => void;
  setSidebarState: (state: SidebarState) => void;
  toggleSidebar: () => void;
  setActiveView: (view: ViewId) => void;
  toggleLang: () => void;
}

export const useLayoutStore = create<LayoutState>()(
  persist(
    (set) => ({
      navMode: "vertical",
      sidebarState: "expanded",
      activeView: "dashboard",
      lang: "zh",
      setNavMode: (navMode) => set({ navMode }),
      toggleNavMode: () =>
        set((s) => ({
          navMode: s.navMode === "horizontal" ? "vertical" : "horizontal",
        })),
      setSidebarState: (sidebarState) => set({ sidebarState }),
      toggleSidebar: () =>
        set((s) => ({
          sidebarState:
            s.sidebarState === "expanded" ? "collapsed" : "expanded",
        })),
      setActiveView: (activeView) => set({ activeView }),
      toggleLang: () => set((s) => ({ lang: s.lang === "zh" ? "en" : "zh" })),
    }),
    {
      name: "layout-prefs",
      partialize: (s) => ({
        navMode: s.navMode,
        sidebarState: s.sidebarState,
        lang: s.lang,
      }),
    }
  )
);
