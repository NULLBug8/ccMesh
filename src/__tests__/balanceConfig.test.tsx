import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { Balances } from "@/pages/Balances";

const balanceMocks = vi.hoisted(() => ({
  endpoints: [
    {
      id: 1,
      name: "daily relay",
      apiUrl: "https://relay.example.com",
      apiKey: "sk-test",
      authMode: "api_key",
      enabled: true,
      useProxy: false,
      transformer: "openai",
      model: "",
      models: ["gpt-5.5"],
      activeModels: [],
      modelMappings: [],
      balanceQuery: {
        enabled: true,
        templateId: "openai-credit-grants",
        method: "GET",
        path: "/dashboard/billing/credit_grants",
        headers: [],
        body: "",
        extraction: {
          balancePath: "$.total_available",
          currencyPath: "$.currency",
          usedPath: "$.total_used",
          expiresAtPath: "$.expires_at",
        },
      },
      remark: "",
      sortOrder: 0,
      testStatus: "unknown",
      createdAt: "",
      updatedAt: "",
    },
  ],
}));

vi.mock("@tanstack/react-query", () => ({
  useQuery: () => ({
    data: balanceMocks.endpoints,
    isLoading: false,
  }),
  useMutation: () => ({
    mutate: vi.fn(),
    isPending: false,
  }),
  useQueryClient: () => ({
    invalidateQueries: vi.fn(),
  }),
}));

vi.mock("@/hooks/useEndpoints", () => ({
  useEndpoints: () => ({
    data: balanceMocks.endpoints,
    isLoading: false,
  }),
}));

describe("Balances page", () => {
  it("shows centralized relay balance query configuration", () => {
    render(<Balances />);

    expect(screen.getByText("余额查询")).toBeInTheDocument();
    expect(screen.getByText("daily relay")).toBeInTheDocument();
    expect(screen.getByText("openai-credit-grants")).toBeInTheDocument();
    expect(screen.getByText("/dashboard/billing/credit_grants")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "查询全部余额" })).toBeInTheDocument();
  });
});
