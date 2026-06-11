import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { describe, expect, it, vi, beforeEach } from "vitest";

const mockGetAppVersion = vi.fn().mockResolvedValue("0.1.2");
const mockOpenReleases = vi.fn().mockResolvedValue(undefined);
const mockCheck = vi.fn();
const mockDownloadAndInstall = vi.fn().mockResolvedValue(undefined);
const mockOnProgress = vi.fn().mockResolvedValue(() => {});

vi.mock("@/services/modules/update", () => ({
  getAppVersion: (...args: unknown[]) => mockGetAppVersion(...args),
  openReleases: (...args: unknown[]) => mockOpenReleases(...args),
  GITHUB_RELEASES_URL: "https://github.com/VkRainB/ccMesh/releases",
  updateApi: {
    check: (...args: unknown[]) => mockCheck(...args),
    downloadAndInstall: (...args: unknown[]) => mockDownloadAndInstall(...args),
    onProgress: (...args: unknown[]) => mockOnProgress(...args),
  },
}));

let storeState = { available: false, version: "", set: vi.fn() };
vi.mock("@/stores/modules/update", () => ({
  useUpdateStore: (sel: (s: typeof storeState) => unknown) => sel(storeState),
}));

import { VersionPopover } from "@/components/business/VersionPopover";

describe("VersionPopover", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    storeState = { available: false, version: "", set: vi.fn() };
    mockGetAppVersion.mockResolvedValue("0.1.2");
  });

  it("渲染版本号文本", async () => {
    render(<VersionPopover />);
    await waitFor(() => {
      expect(screen.getByText("v0.1.2")).toBeInTheDocument();
    });
  });

  it("available=false 时不渲染更新图标", async () => {
    render(<VersionPopover />);
    await waitFor(() => {
      expect(screen.getByText("v0.1.2")).toBeInTheDocument();
    });
    expect(screen.queryByLabelText("下载更新")).not.toBeInTheDocument();
  });

  it("available=true 时渲染更新图标，点击调用 downloadAndInstall", async () => {
    storeState = { available: true, version: "0.2.0", set: vi.fn() };
    render(<VersionPopover />);
    await waitFor(() => {
      expect(screen.getByText("v0.1.2")).toBeInTheDocument();
    });
    const icon = screen.getByLabelText("下载更新");
    expect(icon).toBeInTheDocument();
    fireEvent.click(icon);
    expect(mockDownloadAndInstall).toHaveBeenCalled();
  });

  it("打开 Popover 后点「查看发布」调用 openReleases", async () => {
    render(<VersionPopover />);
    await waitFor(() => {
      expect(screen.getByText("v0.1.2")).toBeInTheDocument();
    });
    fireEvent.click(screen.getByText("v0.1.2"));
    await waitFor(() => {
      expect(screen.getByText("查看发布")).toBeInTheDocument();
    });
    fireEvent.click(screen.getByText("查看发布"));
    expect(mockOpenReleases).toHaveBeenCalled();
  });

  it("打开 Popover 后点 Star 图标调用 openReleases", async () => {
    render(<VersionPopover />);
    await waitFor(() => {
      expect(screen.getByText("v0.1.2")).toBeInTheDocument();
    });
    fireEvent.click(screen.getByText("v0.1.2"));
    await waitFor(() => {
      expect(screen.getByLabelText("Star")).toBeInTheDocument();
    });
    fireEvent.click(screen.getByLabelText("Star"));
    expect(mockOpenReleases).toHaveBeenCalled();
  });

  it("手动检查返回 available=false 时显示已是最新", async () => {
    mockCheck.mockResolvedValue({
      available: false,
      version: "0.1.2",
      currentVersion: "0.1.2",
      notes: "",
    });
    render(<VersionPopover />);
    await waitFor(() => {
      expect(screen.getByText("v0.1.2")).toBeInTheDocument();
    });
    fireEvent.click(screen.getByText("v0.1.2"));
    await waitFor(() => {
      expect(screen.getByLabelText("手动检查更新")).toBeInTheDocument();
    });
    fireEvent.click(screen.getByLabelText("手动检查更新"));
    await waitFor(() => {
      expect(screen.getByText("已是最新版本")).toBeInTheDocument();
    });
  });
});
