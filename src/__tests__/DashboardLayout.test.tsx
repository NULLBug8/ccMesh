import { render } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { Dashboard } from "@/pages/Dashboard";
import { usePageLayoutStore } from "@/stores";

vi.mock("@/hooks/useStats", () => ({
  useStats: () => ({
    data: {
      today: {
        requests: 7,
        errors: 0,
        inputTokens: 100,
        outputTokens: 200,
        cacheCreationTokens: 0,
        cacheReadTokens: 0,
        endpoints: [],
      },
    },
  }),
}));

vi.mock("@/components/business/page-layout/PageLayoutEditor", () => ({
  PageLayoutEditor: () => <div data-testid="layout-editor" />,
}));

vi.mock("@/components/business/RequestMonitor", () => ({
  RequestMonitor: () => <div data-testid="request-monitor" />,
}));

vi.mock("@/pages/Dashboard/_components/ServiceCard", () => ({
  ServiceCard: () => <div data-testid="service-card" />,
}));

describe("Dashboard layout", () => {
  it("uses the available workspace width instead of a narrow centered column", () => {
    usePageLayoutStore.setState({
      editModeByView: {},
      layoutByView: {},
    });

    const { container } = render(<Dashboard />);
    const page = container.firstElementChild;

    expect(page).toHaveClass("w-full");
    expect(page?.className).not.toContain("max-w-5xl");
  });

  it("keeps dashboard sections readable when the saved layout is split", () => {
    usePageLayoutStore.setState({
      editModeByView: {},
      layoutByView: {
        dashboard: {
          mode: "split",
          sections: [
            { id: "hero", visible: true },
            { id: "service", visible: true },
            { id: "stats", visible: true },
            { id: "requests", visible: true },
          ],
        },
      },
    });

    render(<Dashboard />);

    expect(document.querySelector("header")?.parentElement).toHaveClass(
      "xl:col-span-12",
    );
    expect(document.querySelector("[data-testid='service-card']")?.parentElement)
      .toHaveClass("xl:col-span-7");
    expect(document.querySelector("[data-testid='request-monitor']")?.parentElement)
      .toHaveClass("xl:col-span-12");
  });
});
