import { afterEach, describe, expect, it } from "vitest";

import { createTransport, isWebRuntime } from "@/services/runtime";

describe("runtime transport", () => {
  afterEach(() => {
    delete window.__CCMESH_WEB__;
  });

  it("always creates the web transport", () => {
    expect(createTransport().kind).toBe("web");
  });

  it("treats this build as web runtime", () => {
    expect(isWebRuntime()).toBe(true);
    window.__CCMESH_WEB__ = true;
    expect(isWebRuntime()).toBe(true);
  });
});