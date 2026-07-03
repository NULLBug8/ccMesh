import { create } from "zustand";
import { persist } from "zustand/middleware";

import {
  type PageLayoutConfig,
  type PageLayoutDefinition,
} from "@/components/business/page-layout/pageLayoutTypes";
import type { ViewId } from "./layout";

interface PageLayoutState {
  editModeByView: Partial<Record<ViewId, boolean>>;
  layoutByView: Partial<Record<ViewId, PageLayoutConfig>>;
  setEditMode: (view: ViewId, editing: boolean) => void;
  toggleEditMode: (view: ViewId) => void;
  setLayout: (view: ViewId, layout: PageLayoutConfig) => void;
  resetView: (view: ViewId) => void;
  resetAll: () => void;
  isEditing: (view: ViewId) => boolean;
  getLayout: (view: ViewId) => PageLayoutConfig | undefined;
}

const initialState = {
  editModeByView: {},
  layoutByView: {},
} satisfies Pick<PageLayoutState, "editModeByView" | "layoutByView">;

export const usePageLayoutStore = create<PageLayoutState>()(
  persist(
    (set, get) => ({
      ...initialState,
      setEditMode: (view, editing) =>
        set((state) => ({
          editModeByView: {
            ...state.editModeByView,
            [view]: editing,
          },
        })),
      toggleEditMode: (view) =>
        set((state) => ({
          editModeByView: {
            ...state.editModeByView,
            [view]: !state.editModeByView[view],
          },
        })),
      setLayout: (view, layout) =>
        set((state) => ({
          layoutByView: {
            ...state.layoutByView,
            [view]: layout,
          },
        })),
      resetView: (view) =>
        set((state) => {
          const editModeByView = { ...state.editModeByView };
          const layoutByView = { ...state.layoutByView };
          delete editModeByView[view];
          delete layoutByView[view];
          return { editModeByView, layoutByView };
        }),
      resetAll: () => set(initialState),
      isEditing: (view) => Boolean(get().editModeByView[view]),
      getLayout: (view) => get().layoutByView[view],
    }),
    {
      name: "page-layout-prefs",
      partialize: (state) => ({
        layoutByView: state.layoutByView,
      }),
    },
  ),
);

export function resolveViewLayout(
  definition: PageLayoutDefinition,
  saved?: PageLayoutConfig,
): PageLayoutConfig {
  const definitionById = new Map(definition.sections.map((section) => [section.id, section]));
  const savedSections = saved?.sections ?? [];
  const orderedSaved = savedSections
    .filter((section) => definitionById.has(section.id))
    .map((section) => ({
      id: section.id,
      visible: section.visible,
    }));
  const missingSections = definition.sections
    .filter((section) => !orderedSaved.some((savedSection) => savedSection.id === section.id))
    .map((section) => ({
      id: section.id,
      visible: section.defaultVisible ?? true,
    }));

  return {
    mode: saved?.mode ?? definition.defaultMode,
    sections: [...orderedSaved, ...missingSections],
  };
}
