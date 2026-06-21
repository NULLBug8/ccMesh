import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { ErrorDetail, fmtDateTime, fmtTime, RequestLogTable, TokenDetail } from "@/components/business/RequestMonitor";
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
  firstByteMs: 80,
  actualModel: null,
  errorBody: null,
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

  it("成功行展示用时/首字", () => {
    render(<RequestLogTable items={[log]} />);
    expect(screen.getByText("0.12s")).toBeInTheDocument(); // 用时 120ms
    expect(screen.getByText("0.08s")).toBeInTheDocument(); // 首字 80ms
  });

  it("失败行隐藏用时/首字（显示 —）", () => {
    const failed: RequestLog = {
      ...log,
      id: 3,
      statusCode: 500,
      isError: true,
    };
    render(<RequestLogTable items={[failed]} />);
    // 计时单元格应为占位符，且不出现秒数值
    expect(screen.queryByText("0.12s")).not.toBeInTheDocument();
    expect(screen.queryByText("0.08s")).not.toBeInTheDocument();
    expect(screen.getAllByText("—").length).toBeGreaterThanOrEqual(2);
  });

  it("错误行有错误体时展示详情入口", () => {
    const failed: RequestLog = {
      ...log,
      id: 4,
      statusCode: 403,
      isError: true,
      errorBody: '{"error":{"code":"channel:client_restricted"}}',
    };
    render(<RequestLogTable items={[failed]} />);
    expect(screen.getByRole("button", { name: "查看错误详情" })).toBeInTheDocument();
  });

  it("空数据显示占位", () => {
    render(<RequestLogTable items={[]} />);
    expect(screen.getByText("暂无请求记录")).toBeInTheDocument();
  });
});

describe("ErrorDetail", () => {
  it("格式化 JSON 错误体", () => {
    render(<ErrorDetail errorBody='{"error":{"code":"channel:client_restricted"}}' />);
    expect(screen.getByText(/"code": "channel:client_restricted"/)).toBeInTheDocument();
  });

  it("非 JSON 错误体显示原文", () => {
    render(<ErrorDetail errorBody="upstream forbidden" />);
    expect(screen.getByText("upstream forbidden")).toBeInTheDocument();
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

describe("TokenDetail 实际模型", () => {
  it("映射生效时展示实际模型（值为蓝色）", () => {
    const mapped: RequestLog = { ...log, model: "claude-opus-4-8", actualModel: "gpt-5.5" };
    render(<TokenDetail log={mapped} total={20} />);
    expect(screen.getByText("模型：claude-opus-4-8")).toBeInTheDocument();
    expect(screen.getByText(/实际模型/)).toBeInTheDocument();
    const val = screen.getByText("gpt-5.5");
    expect(val.className).toContain("text-info");
  });

  it("无映射(透传)时不展示实际模型", () => {
    render(<TokenDetail log={{ ...log, actualModel: null }} total={20} />);
    expect(screen.queryByText(/实际模型/)).not.toBeInTheDocument();
  });
});
