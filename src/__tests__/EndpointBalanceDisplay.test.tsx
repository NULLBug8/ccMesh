import { render, screen } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { describe, expect, it, vi } from "vitest";

import { TooltipProvider } from "@/components/ui/tooltip";
import { EndpointCard } from "@/pages/Endpoints/_components/EndpointCard";
import type { Endpoint, EndpointBalanceResult } from "@/services/modules/endpoint";

vi.mock("@/hooks/useEndpointHealth", () => ({
  useEndpointHealth: () => ({ data: [] }),
}));

const endpoint: Endpoint = {
  id: 4,
  name: "Café Code",
  apiUrl: "https://www.cafecode.work",
  apiKey: "sk-test",
  authMode: "api_key",
  enabled: true,
  useProxy: false,
  transformer: "codex",
  model: "",
  models: ["gpt-5.5"],
  activeModels: [],
  modelMappings: [],
  balanceQuery: {
    enabled: true,
    templateId: "cafecode",
    method: "GET",
    path: "/v1/usage",
    headers: [],
    body: "",
    extraction: {
      balancePath: "$.remaining",
      currencyPath: "$.unit",
      usedPath: "$.usage.today.actual_cost",
      expiresAtPath: "",
      limits: [],
    },
  },
  remark: "",
  sortOrder: 0,
  testStatus: "available",
  createdAt: "",
  updatedAt: "",
};

const balance: EndpointBalanceResult = {
  success: true,
  status: 200,
  latencyMs: 1200,
  balance: "18.32",
  currency: "USD",
  used: "41.67",
  expiresAt: null,
  limits: [],
  message: "余额查询成功",
  raw: "{}",
};

describe("EndpointCard balance display", () => {
  it("shows cached balance result on the endpoint card", () => {
    const client = new QueryClient({
      defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
    });
    client.setQueryData(["endpoint-balances"], { [endpoint.id]: balance });

    render(
      <QueryClientProvider client={client}>
        <TooltipProvider>
          <EndpointCard endpoint={endpoint} draggable={false} onEdit={vi.fn()} />
        </TooltipProvider>
      </QueryClientProvider>,
    );

    expect(screen.getByText("余额 18 USD")).toBeInTheDocument();
    expect(screen.getByText("已用 42")).toBeInTheDocument();
  });

  it("shows usage-only balance result without a fake empty balance", () => {
    const client = new QueryClient({
      defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
    });
    client.setQueryData(["endpoint-balances"], {
      [endpoint.id]: {
        ...balance,
        balance: null,
        currency: null,
        used: "48.299",
        limits: [],
        message: "用量查询成功：站点未返回余额字段，仅返回已用量",
      },
    });

    render(
      <QueryClientProvider client={client}>
        <TooltipProvider>
          <EndpointCard endpoint={endpoint} draggable={false} onEdit={vi.fn()} />
        </TooltipProvider>
      </QueryClientProvider>,
    );

    expect(screen.queryByText("余额 -")).not.toBeInTheDocument();
    expect(screen.getByText("已用 48")).toBeInTheDocument();
  });
});
