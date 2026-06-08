import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { fmtDateTime, fmtTime, RequestLogTable } from "@/components/business/RequestMonitor";
import type { RequestLog } from "@/services/modules/stats";

const log: RequestLog = {
  id: 1,
  ts: Date.now(),
  endpointName: "ep-a",
  inboundFormat: "claude",
  upstreamUrl: "https://up.example",
  inboundPath: "/v1/messages",
  upstreamPath: "/v1/chat/completions",
  statusCode: 200,
  isError: false,
  inputTokens: 10,
  outputTokens: 5,
  cacheCreationTokens: 2,
  cacheReadTokens: 3,
  model: "claude-3",
  durationMs: 120,
};

describe("RequestLogTable", () => {
  it("渲染请求行、状态码与 Token 合计", () => {
    render(<RequestLogTable items={[log]} />);
    expect(screen.getByText("ep-a")).toBeInTheDocument();
    expect(screen.getByText("200")).toBeInTheDocument();
    // Token 合计 = 10 + 5 + 2 + 3
    expect(screen.getByText("20")).toBeInTheDocument();
  });

  it("入站/出站展示真实路由路径", () => {
    render(<RequestLogTable items={[log]} />);
    expect(screen.getByText("/v1/messages")).toBeInTheDocument();
    expect(screen.getByText("/v1/chat/completions")).toBeInTheDocument();
  });

  it("旧行无路径时按入站协议推断兜底", () => {
    const legacy: RequestLog = {
      ...log,
      id: 2,
      inboundFormat: "openai",
      inboundPath: "",
      upstreamPath: "",
    };
    render(<RequestLogTable items={[legacy]} />);
    // 入站与出站都兜底为 openai 路由
    expect(screen.getAllByText("/v1/chat/completions")).toHaveLength(2);
  });

  it("空数据显示占位", () => {
    render(<RequestLogTable items={[]} />);
    expect(screen.getByText("暂无请求记录")).toBeInTheDocument();
  });
});

describe("fmtTime", () => {
  it("按 24 小时制 时:分:秒 零填充展示", () => {
    // 用本地时间分量构造，断言与时区无关
    const ts = new Date(2026, 5, 7, 9, 5, 3).getTime();
    expect(fmtTime(ts)).toBe("09:05:03");
  });

  it("午夜为 00:00:00（非 24:00:00）", () => {
    const ts = new Date(2026, 5, 7, 0, 0, 0).getTime();
    expect(fmtTime(ts)).toBe("00:00:00");
  });

  it("下午为 24 小时制（无上午/下午前缀）", () => {
    const ts = new Date(2026, 5, 7, 23, 59, 59).getTime();
    expect(fmtTime(ts)).toBe("23:59:59");
  });
});

describe("fmtDateTime", () => {
  it("展示 年-月-日 时:分:秒（零填充，24 小时制）", () => {
    const ts = new Date(2026, 5, 7, 9, 5, 3).getTime();
    expect(fmtDateTime(ts)).toBe("2026-06-07 09:05:03");
  });
});
