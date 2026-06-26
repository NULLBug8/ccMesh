import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { describe, expect, it, vi } from "vitest";

import { TooltipProvider } from "@/components/ui/tooltip";
import { Balances } from "@/pages/Balances";
import { EndpointForm } from "@/pages/Endpoints/_components/EndpointForm";
import { BALANCE_QUERY_PRESETS, endpointApi } from "@/services/modules/endpoint";

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
        templateId: "newapi",
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
      remark: "",
      sortOrder: 0,
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

function queryClient() {
  return new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
}

function renderEndpointForm(endpoint = balanceMocks.endpoints[0]) {
  return render(
    <QueryClientProvider client={queryClient()}>
      <TooltipProvider>
        <EndpointForm open onOpenChange={vi.fn()} editing={endpoint} />
      </TooltipProvider>
    </QueryClientProvider>,
  );
}

async function openBalanceTab() {
  const balanceTab = screen.getByRole("tab", { name: "余额" });
  fireEvent.pointerDown(balanceTab);
  fireEvent.mouseDown(balanceTab);
  fireEvent.click(balanceTab);
}

const sampleProbeResult = {
  status: "sampleAvailable" as const,
  results: [
    {
      templateId: "newapi",
      path: "/api/user/self",
      success: false,
      urlReachable: true,
      statusCode: 200,
      latencyMs: 18,
      message: "need json path",
      sample: "{\"data\":{\"balance\":88}}",
      balance: null,
      config: null,
    },
  ],
  matched: null,
  usableSamples: [
    {
      templateId: "newapi",
      path: "/api/user/self",
      success: false,
      urlReachable: true,
      statusCode: 200,
      latencyMs: 18,
      message: "need json path",
      sample: "{\"data\":{\"balance\":88}}",
      balance: null,
      config: null,
    },
  ],
};

describe("Balances page", () => {
  it("shows centralized relay balance query configuration", () => {
    render(
      <QueryClientProvider client={queryClient()}>
        <Balances />
      </QueryClientProvider>,
    );

    expect(screen.getByText("余额查询")).toBeInTheDocument();
    expect(screen.getByText("daily relay")).toBeInTheDocument();
    expect(screen.getByText("newapi")).toBeInTheDocument();
    expect(screen.getByText("/api/user/self")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "查询全部余额" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "查询 daily relay 余额" })).toBeEnabled();
    expect(screen.getByText("站点")).toBeInTheDocument();
    expect(screen.getByText("模板 / 路径")).toBeInTheDocument();
  });
});

describe("Balance presets", () => {
  it("names common relay templates by relay type", () => {
    const ids = BALANCE_QUERY_PRESETS.map((item) => item.templateId);

    expect(ids).toContain("newapi");
    expect(ids).toContain("one-api");
    expect(ids).toContain("sub2api");
    expect(ids).toContain("openai");
  });
});

describe("Endpoint balance template assistant", () => {
  it("blocks AI generation when every built-in template URL fails", async () => {
    vi.spyOn(endpointApi, "probeBalanceTemplates").mockResolvedValueOnce({
      status: "allFailed",
      results: [
        {
          templateId: "newapi",
          path: "/api/user/self",
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
    await openBalanceTab();
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
        templateId: "newapi",
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
          templateId: "newapi",
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
    await openBalanceTab();
    fireEvent.click(await screen.findByRole("button", { name: "智能识别余额模板" }));

    await waitFor(() => {
      expect(screen.getByDisplayValue("/api/user/self")).toBeInTheDocument();
      expect(screen.getByDisplayValue("$.data.quota")).toBeInTheDocument();
    });
  });

  it("uses a selected model from the current endpoint to generate a template", async () => {
    vi.spyOn(endpointApi, "probeBalanceTemplates").mockResolvedValueOnce(sampleProbeResult);
    const generate = vi.spyOn(endpointApi, "generateBalanceTemplate").mockResolvedValueOnce({
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
    await openBalanceTab();
    fireEvent.click(await screen.findByRole("button", { name: "智能识别余额模板" }));

    const aiSelect = await screen.findByLabelText("AI 配置模型");
    fireEvent.change(aiSelect, { target: { value: "gpt-5.5" } });
    fireEvent.click(screen.getByRole("button", { name: "让 AI 生成模板" }));

    await waitFor(() => {
      expect(screen.getByDisplayValue("$.data.balance")).toBeInTheDocument();
    });
    expect(generate).toHaveBeenCalledWith(
      1,
      "gpt-5.5",
      expect.objectContaining({ path: "/api/user/self" }),
    );
  });

  it("requires current endpoint models before AI template generation is available", async () => {
    vi.spyOn(endpointApi, "probeBalanceTemplates").mockResolvedValueOnce(sampleProbeResult);
    const endpointWithoutModels = {
      ...balanceMocks.endpoints[0],
      model: "",
      models: [],
    };

    renderEndpointForm(endpointWithoutModels);
    await openBalanceTab();
    fireEvent.click(await screen.findByRole("button", { name: "智能识别余额模板" }));

    expect(await screen.findByText("请先在此站点下添加或拉取模型")).toBeInTheDocument();
    expect(screen.queryByLabelText("AI 配置模型")).not.toBeInTheDocument();
  });
});
