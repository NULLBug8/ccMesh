import "@testing-library/jest-dom/vitest";
import { vi } from "vitest";

// mock Tauri IPC，使前端逻辑可在 jsdom 下测试
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockResolvedValue(undefined),
}));
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));
