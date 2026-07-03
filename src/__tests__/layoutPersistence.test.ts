import { beforeEach, describe, expect, it } from "vitest";

import { useLayoutStore } from "@/stores/modules/layout";

describe("layout persistence", () => {
  beforeEach(() => {
    localStorage.clear();
    useLayoutStore.setState({ activeView: "dashboard" });
  });

  it("persists active view so refresh stays on current page", () => {
    useLayoutStore.getState().setActiveView("logs");

    const saved = JSON.parse(localStorage.getItem("layout-prefs") ?? "{}");

    expect(saved.state.activeView).toBe("logs");
  });
});
