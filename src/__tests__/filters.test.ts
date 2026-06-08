import { beforeEach, describe, expect, it } from "vitest";

import { useFilterStore } from "@/stores/modules/filters";

describe("filters store", () => {
  beforeEach(() => useFilterStore.getState().reset());

  it("isActive 默认 false", () => {
    expect(useFilterStore.getState().isActive()).toBe(false);
  });

  it("搜索时 isActive 为 true", () => {
    useFilterStore.getState().setSearch("foo");
    expect(useFilterStore.getState().isActive()).toBe(true);
  });

  it("筛选类型时 isActive 为 true", () => {
    useFilterStore.getState().setTransformer("openai");
    expect(useFilterStore.getState().isActive()).toBe(true);
  });

  it("仅启用时 isActive 为 true", () => {
    useFilterStore.getState().setEnabledOnly(true);
    expect(useFilterStore.getState().isActive()).toBe(true);
  });
});
