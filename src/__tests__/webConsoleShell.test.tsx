import type { ReactNode } from "react";
import { fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { AppLayout } from "@/layouts/AppLayout";
import { TooltipProvider } from "@/components/ui/tooltip";
import { useLayoutStore, usePageLayoutStore } from "@/stores";

vi.mock("@/components/common", () => ({
  Logo: ({ iconOnly, extra }: { iconOnly?: boolean; extra?: ReactNode }) => (
    <div data-testid={iconOnly ? "logo-icon" : "logo-full"}>
      <span>Logo</span>
      {extra}
    </div>
  ),
  ThemeToggle: () => <button type="button">Theme</button>,
  LangToggle: () => <button type="button">Lang</button>,
}));

vi.mock("@/components/business", () => ({
  UpdateBadge: () => <span data-testid="update-badge" />,
  VersionPopover: () => <span data-testid="version-popover" />,
}));

vi.mock("@/pages/Dashboard", () => ({
  Dashboard: () => <div>Dashboard page</div>,
}));

vi.mock("@/pages/Endpoints", () => ({
  Endpoints: () => <div>Endpoints page</div>,
}));

vi.mock("@/pages/ConfigProfiles", () => ({
  ConfigProfiles: () => <div>Config Profiles page</div>,
}));

vi.mock("@/pages/Rules", () => ({
  Rules: () => <div>Rules page</div>,
}));

vi.mock("@/pages/Balances", () => ({
  Balances: () => <div>Balances page</div>,
}));

vi.mock("@/pages/Statistics", () => ({
  Statistics: () => <div>Statistics page</div>,
}));

vi.mock("@/pages/Sync", () => ({
  Sync: () => <div>Sync page</div>,
}));

vi.mock("@/pages/Logs", () => ({
  Logs: () => <div>Logs page</div>,
}));

vi.mock("@/pages/Settings", () => ({
  Settings: () => <div>Settings page</div>,
}));

describe("web console shell", () => {
  window.matchMedia =
    window.matchMedia ??
    ((query: string) =>
      ({
        matches: false,
        media: query,
        onchange: null,
        addEventListener: vi.fn(),
        removeEventListener: vi.fn(),
        addListener: vi.fn(),
        removeListener: vi.fn(),
        dispatchEvent: vi.fn(),
      }) as MediaQueryList);

  beforeEach(() => {
    useLayoutStore.setState({
      navMode: "vertical",
      sidebarState: "expanded",
      activeView: "dashboard",
      lang: "en",
      endpointView: "list",
    });
    usePageLayoutStore.setState({
      editModeByView: {},
      layoutByView: {},
    });
    localStorage.clear();
    window.__CCMESH_WEB__ = true;
  });

  afterEach(() => {
    delete window.__CCMESH_WEB__;
  });

  function renderShell() {
    return render(
      <TooltipProvider>
        <AppLayout />
      </TooltipProvider>,
    );
  }

  it("hides desktop-only window controls in web mode", async () => {
    renderShell();
    expect(screen.getByText("ccMesh Web")).toBeInTheDocument();
    expect(screen.queryByLabelText("最小化")).not.toBeInTheDocument();
    expect(await screen.findByText("Dashboard page")).toBeInTheDocument();
  });

  it("switches between menu pages in web mode", async () => {
    renderShell();

    expect(await screen.findByText("Dashboard page")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Rules" }));
    expect(await screen.findByText("Rules page")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Balances" }));
    expect(await screen.findByText("Balances page")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Logs" }));
    expect(await screen.findByText("Logs page")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Settings" }));
    expect(await screen.findByText("Settings page")).toBeInTheDocument();
  });
});
