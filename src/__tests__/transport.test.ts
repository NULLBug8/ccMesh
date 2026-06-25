import { afterEach, describe, expect, it } from "vitest";

import { createTransport, isWebRuntime } from "@/services/runtime";

describe("runtime transport", () => {
  afterEach(() => {
    delete window.__CCMESH_WEB__;
    delete window.__TAURI_INTERNALS__;
  });

  it("creates a web transport when window.__CCMESH_WEB__ is true", () => {
    window.__CCMESH_WEB__ = true;
    expect(createTransport().kind).toBe("web");
  });

  it("creates a desktop transport when Tauri internals are present", () => {
    window.__TAURI_INTERNALS__ = {};
    expect(createTransport().kind).toBe("desktop");
  });

  it("prefers explicit host markers over inferred runtime", () => {
    expect(isWebRuntime()).toBe(true);
    window.__TAURI_INTERNALS__ = {};
    expect(isWebRuntime()).toBe(false);
    window.__CCMESH_WEB__ = true;
    expect(isWebRuntime()).toBe(true);
  });
});
