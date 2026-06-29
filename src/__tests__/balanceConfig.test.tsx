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
      testModel: "",
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
          limits: [],
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
    {
      templateId: "sub2api",
      path: "/api/user/quota",
      success: false,
      urlReachable: true,
      statusCode: 200,
      latencyMs: 22,
      message: "need json path",
      sample: "{\"data\":{\"three_hour\":{\"remain\":10},\"daily\":{\"remain\":90}}}",
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
    {
      templateId: "sub2api",
      path: "/api/user/quota",
      success: false,
      urlReachable: true,
      statusCode: 200,
      latencyMs: 22,
      message: "need json path",
      sample: "{\"data\":{\"three_hour\":{\"remain\":10},\"daily\":{\"remain\":90}}}",
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
    expect(ids).toContain("newapi-user-key");
    expect(ids).toContain("crazyrouter");
    expect(ids).toContain("cafecode");
    expect(ids).toContain("tokenfor-me");
    expect(ids).toContain("laozhang");
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

  it("does not offer AI generation when URLs only return login or auth error samples", async () => {
    vi.spyOn(endpointApi, "probeBalanceTemplates").mockResolvedValueOnce({
      status: "allFailed",
      results: [
        {
          templateId: "openai",
          path: "/dashboard/billing/credit_grants",
          success: false,
          urlReachable: true,
          statusCode: 200,
          latencyMs: 12,
          message: "返回的是 HTML 页面，不是余额 JSON",
          sample: "<!doctype html><html></html>",
          config: null,
          balance: null,
        },
        {
          templateId: "newapi",
          path: "/api/user/self",
          success: false,
          urlReachable: true,
          statusCode: 200,
          latencyMs: 18,
          message: "鉴权失败，不能作为 AI 样本",
          sample: "{\"success\":false,\"message\":\"Unauthorized, invalid access token\"}",
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

    expect(await screen.findByText("模板 URL 有返回，但没有可用于 AI 的余额样本")).toBeInTheDocument();
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
            limits: [],
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
        limits: [
          {
            label: "3小时额度",
            balancePath: "$.data.three_hour.remain",
            usedPath: "$.data.three_hour.used",
            expiresAtPath: "$.data.three_hour.reset_at",
          },
          {
            label: "一天额度",
            balancePath: "$.data.daily.remain",
            usedPath: "$.data.daily.used",
            expiresAtPath: "$.data.daily.reset_at",
          },
        ],
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
      expect(screen.getByDisplayValue("3小时额度")).toBeInTheDocument();
      expect(screen.getByDisplayValue("$.data.three_hour.remain")).toBeInTheDocument();
    });
    expect(generate).toHaveBeenCalledWith(
      1,
      "gpt-5.5",
      expect.arrayContaining([
        expect.objectContaining({ path: "/api/user/self" }),
        expect.objectContaining({ path: "/api/user/quota" }),
      ]),
    );
  });

  it("tests the current balance template before saving", async () => {
    const testBalance = vi.spyOn(endpointApi, "testBalanceTemplate").mockResolvedValueOnce({
      success: true,
      status: 200,
      latencyMs: 33,
      balance: "88",
      currency: "USD",
      used: "12",
      expiresAt: null,
      limits: [
        {
          label: "1周额度",
          balance: "700",
          used: "300",
          expiresAt: "2026-07-04T00:00:00Z",
        },
      ],
      message: "余额查询成功",
      raw: "{}",
    });

    renderEndpointForm();
    await openBalanceTab();
    fireEvent.change(screen.getByDisplayValue("$.data.quota"), {
      target: { value: "$.data.balance" },
    });
    fireEvent.click(screen.getByRole("button", { name: "测试当前模板" }));

    await waitFor(() => {
      expect(testBalance).toHaveBeenCalledWith(
        1,
        expect.objectContaining({
          extraction: expect.objectContaining({ balancePath: "$.data.balance" }),
        }),
      );
      expect(screen.getByText("余额 88 USD")).toBeInTheDocument();
      expect(screen.getByText("1周额度：剩余 700，已用 300，到期 2026-07-04T00:00:00Z")).toBeInTheDocument();
    });
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
