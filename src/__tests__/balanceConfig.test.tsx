import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { describe, expect, it, vi } from "vitest";

import { Balances } from "@/pages/Balances";
import { EndpointForm } from "@/pages/Endpoints/_components/EndpointForm";
import { TooltipProvider } from "@/components/ui/tooltip";
import { endpointApi } from "@/services/modules/endpoint";

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
    {
      id: 2,
      name: "ai config endpoint",
      apiUrl: "https://ai.example.com",
      apiKey: "sk-ai",
      authMode: "api_key",
      enabled: true,
      useProxy: false,
      transformer: "openai",
      model: "gpt-5",
      models: ["gpt-5"],
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
      sortOrder: 1,
      testStatus: "unknown",
      createdAt: "",
      updatedAt: "",
    },
  ],
}));

vi.mock("@/hooks/useEndpoints", () => ({
  useEndpoints: () => ({
    data: balanceMocks.endpoints,
    isLoading: false,
  }),
}));

describe("Balances page", () => {
  it("shows centralized relay balance query configuration", () => {
    const client = new QueryClient({
      defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
    });
    render(
      <QueryClientProvider client={client}>
        <Balances />
      </QueryClientProvider>,
    );

    expect(screen.getByText("余额查询")).toBeInTheDocument();
    expect(screen.getByText("daily relay")).toBeInTheDocument();
    expect(screen.getAllByText("openai-credit-grants").length).toBeGreaterThan(0);
    expect(screen.getAllByText("/dashboard/billing/credit_grants").length).toBeGreaterThan(0);
    expect(screen.getByRole("button", { name: "查询全部余额" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "查询 daily relay 余额" })).toBeEnabled();
    expect(screen.getByText("站点")).toBeInTheDocument();
    expect(screen.getByText("模板 / 路径")).toBeInTheDocument();
  });
});

function renderEndpointForm() {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  const endpoint = balanceMocks.endpoints[0];

  return render(
    <QueryClientProvider client={client}>
      <TooltipProvider>
        <EndpointForm open onOpenChange={vi.fn()} editing={endpoint} />
      </TooltipProvider>
    </QueryClientProvider>,
  );
}

describe("Endpoint balance template assistant", () => {
  it("blocks AI generation when every built-in template URL fails", async () => {
    vi.spyOn(endpointApi, "probeBalanceTemplates").mockResolvedValueOnce({
      status: "allFailed",
      results: [
        {
          templateId: "openai-credit-grants",
          path: "/dashboard/billing/credit_grants",
          success: false,
          urlReachable: false,
          statusCode: null,
          latencyMs: 12,
          message: "HTTP 404",
          sample: null,
          config: null,
          balance: null,
        },
      ],
      matched: null,
      usableSamples: [],
    });

    renderEndpointForm();

    const balanceTab = screen.getByRole("tab", { name: "余额" });
    fireEvent.pointerDown(balanceTab);
    fireEvent.mouseDown(balanceTab);
    fireEvent.click(balanceTab);
    fireEvent.click(await screen.findByRole("button", { name: "智能识别余额模板" }));

    expect(await screen.findByText("全部模板 URL 都没有请求成功")).toBeInTheDocument();
    expect(screen.getByLabelText("自定义余额接口路径")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "让 AI 生成模板" })).not.toBeInTheDocument();
  });

  it("applies the matched template when a probe extracts balance", async () => {
    vi.spyOn(endpointApi, "probeBalanceTemplates").mockResolvedValueOnce({
      status: "matched",
      results: [],
      matched: {
        templateId: "newapi-user-self",
        path: "/api/user/self",
        success: true,
        urlReachable: true,
        statusCode: 200,
        latencyMs: 18,
        message: "余额字段已识别",
        sample: null,
        balance: "123",
        config: {
          enabled: true,
          templateId: "newapi-user-self",
          method: "GET",
          path: "/api/user/self",
          headers: [{ name: "Authorization", value: "Bearer {{apiKey}}" }],
          body: "",
          extraction: {
            balancePath: "$.data.quota",
            currencyPath: "$.data.currency",
            usedPath: "$.data.used_quota",
            expiresAtPath: "",
          },
        },
      },
      usableSamples: [],
    });

    renderEndpointForm();

    const balanceTab = screen.getByRole("tab", { name: "余额" });
    fireEvent.pointerDown(balanceTab);
    fireEvent.mouseDown(balanceTab);
    fireEvent.click(balanceTab);
    fireEvent.click(await screen.findByRole("button", { name: "智能识别余额模板" }));

    await waitFor(() => {
      expect(screen.getByDisplayValue("/api/user/self")).toBeInTheDocument();
      expect(screen.getByDisplayValue("$.data.quota")).toBeInTheDocument();
    });
  });

  it("uses a selected AI endpoint to generate a template from a reachable sample", async () => {
    vi.spyOn(endpointApi, "probeBalanceTemplates").mockResolvedValueOnce({
      status: "sampleAvailable",
      results: [
        {
          templateId: "newapi-user-self",
          path: "/api/user/self",
          success: false,
          urlReachable: true,
          statusCode: 200,
          latencyMs: 18,
          message: "余额响应中未找到余额字段",
          sample: "{\"data\":{\"balance\":88}}",
          balance: null,
          config: null,
        },
      ],
      matched: null,
      usableSamples: [
        {
          templateId: "newapi-user-self",
          path: "/api/user/self",
          success: false,
          urlReachable: true,
          statusCode: 200,
          latencyMs: 18,
          message: "余额响应中未找到余额字段",
          sample: "{\"data\":{\"balance\":88}}",
          balance: null,
          config: null,
        },
      ],
    });
    vi.spyOn(endpointApi, "generateBalanceTemplate").mockResolvedValueOnce({
      enabled: true,
      templateId: "ai-generated",
      method: "GET",
      path: "/api/user/self",
      headers: [{ name: "Authorization", value: "Bearer {{apiKey}}" }],
      body: "",
      extraction: {
        balancePath: "$.data.balance",
        currencyPath: "",
        usedPath: "",
        expiresAtPath: "",
      },
    });

    renderEndpointForm();

    const balanceTab = screen.getByRole("tab", { name: "余额" });
    fireEvent.pointerDown(balanceTab);
    fireEvent.mouseDown(balanceTab);
    fireEvent.click(balanceTab);
    fireEvent.click(await screen.findByRole("button", { name: "智能识别余额模板" }));

    const aiSelect = await screen.findByLabelText("AI 配置端点");
    fireEvent.change(aiSelect, { target: { value: "2" } });
    fireEvent.click(screen.getByRole("button", { name: "让 AI 生成模板" }));

    await waitFor(() => {
      expect(screen.getByDisplayValue("$.data.balance")).toBeInTheDocument();
    });
  });
});
