import { create } from "zustand";

interface FilterState {
  search: string;
  enabledOnly: boolean;
  transformer: string; // "all" | "claude" | "openai"
  setSearch: (v: string) => void;
  setEnabledOnly: (v: boolean) => void;
  setTransformer: (v: string) => void;
  reset: () => void;
  /** 是否处于筛选状态（筛选时禁用拖拽排序）。 */
  isActive: () => boolean;
}

export const useFilterStore = create<FilterState>((set, get) => ({
  search: "",
  enabledOnly: false,
  transformer: "all",
  setSearch: (search) => set({ search }),
  setEnabledOnly: (enabledOnly) => set({ enabledOnly }),
  setTransformer: (transformer) => set({ transformer }),
  reset: () => set({ search: "", enabledOnly: false, transformer: "all" }),
  isActive: () => {
    const s = get();
    return s.search !== "" || s.enabledOnly || s.transformer !== "all";
  },
}));
