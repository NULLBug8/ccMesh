import { render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { Settings } from "@/pages/Settings";
import { usePageLayoutStore } from "@/stores";

vi.mock("@tanstack/react-query", () => ({
  useQuery: ({ queryKey }: { queryKey: string[] }) => {
    if (queryKey[0] === "config") {
      return {
        data: {
          port: 3000,
          theme: "system",
          themeAuto: false,
          autoLightStart: "07:00",
          autoDarkStart: "19:00",
          language: "zh",
          logLevel: "info",
          proxyEnabled: false,
          proxyUrl: "",
          openaiUa: "",
          claudeCliUa: "",
          globalTestModel: "",
        },
      };
    }
    return { data: false, isLoading: false };
  },
  useQueryClient: () => ({ invalidateQueries: vi.fn() }),
}));

vi.mock("next-themes", () => ({ useTheme: () => ({ setTheme: vi.fn() }) }));
vi.mock("@/pages/Settings/_components/TokenCounter", () => ({
  TokenCounter: () => <div>token-counter</div>,
}));

describe("Settings layout", () => {
  beforeEach(() => {
    usePageLayoutStore.setState({
      editModeByView: { settings: true },
      layoutByView: {
        settings: {
          mode: "stack",
          sections: [
            { id: "header", visible: true },
            { id: "general", visible: true },
            { id: "proxy", visible: false },
            { id: "advanced", visible: false },
            { id: "tokens", visible: false },
          ],
        },
      },
    });
  });

  it("uses the shared layout editor and hides disabled sections", () => {
    render(<Settings />);
    expect(screen.getByText("布局编辑")).toBeInTheDocument();
    expect(screen.queryByText("token-counter")).not.toBeInTheDocument();
  });
});