import { render } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { Endpoints } from "@/pages/Endpoints";
import { useFilterStore, useLayoutStore, usePageLayoutStore } from "@/stores";

vi.mock("@/hooks/useEndpoints", () => ({
  useEndpoints: () => ({
    data: [
      {
        id: 1,
        name: "Daily endpoint",
        apiUrl: "https://example.com",
        apiKey: "test",
        transformer: "codex",
        model: "gpt-5",
        models: [],
        activeModels: [],
        modelMappings: {},
        enabled: true,
        testStatus: "unknown",
      },
    ],
    isLoading: false,
  }),
}));

vi.mock("@/hooks/useEndpointHealth", () => ({
  useEndpointHealthEvents: () => undefined,
}));

vi.mock("@/components/business/page-layout/PageLayoutEditor", () => ({
  PageLayoutEditor: () => <div data-testid="layout-editor" />,
}));

vi.mock("@/pages/Endpoints/_components/DnDList", () => ({
  DnDList: () => <div data-testid="endpoint-list" />,
}));

vi.mock("@/pages/Endpoints/_components/EndpointForm", () => ({
  EndpointForm: () => null,
}));

vi.mock("@/pages/Endpoints/_components/FilterBar", () => ({
  FilterBar: () => <div data-testid="filter-bar" />,
}));

vi.mock("@/pages/Endpoints/_components/ModelList", () => ({
  ModelList: () => <div data-testid="model-list" />,
}));

describe("Endpoints layout", () => {
  it("uses the available workspace width instead of a narrow centered column", () => {
    useFilterStore.setState({
      search: "",
      enabledOnly: false,
      transformer: "all",
    });
    useLayoutStore.setState({ endpointView: "list" });
    usePageLayoutStore.setState({
      editModeByView: {},
      layoutByView: {},
    });

    const { container } = render(<Endpoints />);
    const page = container.firstElementChild;

    expect(page).toHaveClass("w-full");
    expect(page?.className).not.toContain("max-w-4xl");
  });

  it("keeps endpoint filters on their own full-width row in split layout", () => {
    useFilterStore.setState({
      search: "",
      enabledOnly: false,
      transformer: "all",
    });
    useLayoutStore.setState({ endpointView: "list" });
    usePageLayoutStore.setState({
      editModeByView: {},
      layoutByView: {},
    });

    render(<Endpoints />);
    const headerSection = document.querySelector("[data-testid='filter-bar']")
      ?.parentElement?.parentElement;

    expect(headerSection).toHaveClass("xl:col-span-12");
  });
});
