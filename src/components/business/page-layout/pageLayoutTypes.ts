import type { ReactNode } from "react";

export type PageLayoutMode = "stack" | "two-column" | "split";

export interface PageSectionState {
  id: string;
  visible: boolean;
}

export interface PageLayoutConfig {
  mode: PageLayoutMode;
  sections: PageSectionState[];
}

export interface PageLayoutDefinition {
  defaultMode: PageLayoutMode;
  sections: Array<{
    id: string;
    title: string;
    defaultVisible?: boolean;
  }>;
}

export interface PageSectionEntry {
  title: string;
  render: () => ReactNode;
  className?: string;
  modeClassName?: Partial<Record<PageLayoutMode, string>>;
}

export type PageSectionRegistry = Record<string, PageSectionEntry>;
