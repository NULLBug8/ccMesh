import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { Rules } from "@/pages/Rules";
import { usePageLayoutStore } from "@/stores";

const rulesMocks = vi.hoisted(() => {
  const baseRules = {
    routing: {
      strategy: "balanced",
      modelAffinity: true,
      headerAffinity: true,
      modelMappingStrategy: "site-first",
      maxRetries: 0,
      requestTimeoutSeconds: 0,
    },
    circuitBreaker: {
      failureThreshold: 4,
      successThreshold: 2,
      timeoutSeconds: 60,
      errorRateThreshold: 0.6,
      minRequests: 10,
      failureStatusCodes: [429, 500, 502, 503, 504],
    },
    degradation: {
      enabled: true,
      reasoningEffortFallback: true,
      requestThinkingSignature: true,
      retryWithoutStream: false,
      fallbackTemperature: 0,
    },
  };

  return {
    baseRules,
    setConfig: vi.fn().mockResolvedValue(baseRules),
    resetConfig: vi.fn().mockResolvedValue(baseRules),
  };
});

vi.mock("@tanstack/react-query", () => ({
  useQuery: ({ queryKey }: { queryKey: string[] }) => {
    if (queryKey[0] === "rules-config") {
      return {
        data: rulesMocks.baseRules,
        isLoading: false,
      };
    }

    return {
      data: undefined,
      isLoading: false,
    };
  },
  useQueryClient: () => ({
    invalidateQueries: vi.fn(),
  }),
}));

vi.mock("sonner", () => ({
  toast: {
    success: vi.fn(),
    error: vi.fn(),
  },
}));

vi.mock("@/services/modules/rules", () => ({
  rulesApi: {
    getConfig: vi.fn(),
    setConfig: rulesMocks.setConfig,
    resetConfig: rulesMocks.resetConfig,
  },
}));

describe("Rules page", () => {
  beforeEach(() => {
    rulesMocks.setConfig.mockClear();
    rulesMocks.resetConfig.mockClear();
    usePageLayoutStore.setState({
      editModeByView: {
        rules: true,
      },
      layoutByView: {
        rules: {
          mode: "stack",
          sections: [
            { id: "header", visible: true },
            { id: "routing", visible: true },
            { id: "circuitBreaker", visible: true },
            { id: "degradation", visible: true },
          ],
        },
      },
    });
  });

  it("loads rules and saves edited breaker thresholds", async () => {
    render(<Rules />);

    expect(screen.getByText("布局编辑")).toBeInTheDocument();

    fireEvent.change(screen.getByLabelText("failure-threshold"), {
      target: { value: "9" },
    });
    fireEvent.click(screen.getByRole("button", { name: "保存规则" }));

    await waitFor(() => {
      expect(rulesMocks.setConfig).toHaveBeenCalledWith(
        expect.objectContaining({
          circuitBreaker: expect.objectContaining({
            failureThreshold: 9,
          }),
        }),
      );
    });
  });

  it("shows model mapping strategy and concrete examples for configurable fields", async () => {
    render(<Rules />);

    expect(screen.getByText("模型映射策略")).toBeInTheDocument();
    expect(screen.getByText(/示例：站点 A 同时配置原生 GPT-5.5/)).toBeInTheDocument();
    expect(screen.getByText(/示例：429,500,502,503,504/)).toBeInTheDocument();
    expect(screen.getByText(/示例：30 表示单次上游请求最多等待 30 秒/)).toBeInTheDocument();

    fireEvent.click(screen.getByText("全局原生优先"));
    fireEvent.click(screen.getByRole("button", { name: "保存规则" }));

    await waitFor(() => {
      expect(rulesMocks.setConfig).toHaveBeenCalledWith(
        expect.objectContaining({
          routing: expect.objectContaining({
            modelMappingStrategy: "global-native-first",
          }),
        }),
      );
    });
  });
});
