import { beforeEach, describe, expect, it } from "vitest";

import { usePageLayoutStore } from "@/stores/modules/pageLayout";

describe("page layout store", () => {
  beforeEach(() => usePageLayoutStore.getState().resetAll());

  it("stores per-view edit mode and layout preferences independently", () => {
    const store = usePageLayoutStore.getState();

    store.setEditMode("dashboard", true);
    store.setLayout("dashboard", {
      mode: "two-column",
      sections: [
        { id: "service", visible: true },
        { id: "stats", visible: true },
      ],
    });
    store.setLayout("logs", {
      mode: "split",
      sections: [
        { id: "log-stream", visible: true },
        { id: "request-trace", visible: true },
      ],
    });

    expect(usePageLayoutStore.getState().isEditing("dashboard")).toBe(true);
    expect(usePageLayoutStore.getState().isEditing("logs")).toBe(false);
    expect(usePageLayoutStore.getState().getLayout("dashboard")?.mode).toBe(
      "two-column",
    );
    expect(usePageLayoutStore.getState().getLayout("logs")?.mode).toBe("split");
  });

  it("toggles edit mode only for the active view", () => {
    const store = usePageLayoutStore.getState();

    store.toggleEditMode("settings");
    expect(store.isEditing("settings")).toBe(true);
    expect(store.isEditing("dashboard")).toBe(false);

    store.toggleEditMode("settings");
    expect(store.isEditing("settings")).toBe(false);
  });
});
