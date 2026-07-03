import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it } from "vitest";

import { useEndpointBalanceResults } from "@/hooks/useEndpointBalanceResults";
import type { EndpointBalanceResult } from "@/services/modules/endpoint";

const cachedBalance: EndpointBalanceResult = {
  success: true,
  status: 200,
  latencyMs: 321,
  balance: "8.7",
  currency: "USD",
  used: "51.2",
  expiresAt: null,
  limits: [],
  message: "余额查询成功",
  raw: "{}",
};

function Probe() {
  const { results } = useEndpointBalanceResults();
  return <div>{results[7]?.balance ?? "missing"}</div>;
}

describe("endpoint balance result persistence", () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it("hydrates balance results from localStorage after page reload", async () => {
    localStorage.setItem(
      "endpoint-balance-results",
      JSON.stringify({ 7: cachedBalance }),
    );
    const client = new QueryClient({
      defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
    });

    render(
      <QueryClientProvider client={client}>
        <Probe />
      </QueryClientProvider>,
    );

    await waitFor(() => expect(screen.getByText("8.7")).toBeInTheDocument());
  });
});
