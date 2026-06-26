import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { RequestTracePanel } from "@/pages/Logs/_components/RequestTracePanel";
import type { RequestLog } from "@/services/modules/stats";

const tracedLog: RequestLog = {
  id: 11,
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
  cacheCreationTokens: 0,
  cacheReadTokens: 0,
  model: "claude-opus-4",
  durationMs: 200,
  firstByteMs: 120,
  actualModel: "gpt-5.5",
  errorBody: null,
  trace: {
    receivedRequest: {
      method: "POST",
      url: "/v1/messages",
      statusCode: null,
      headers: [
        { key: "content-type", value: "application/json" },
        { key: "x-request-id", value: "req_1" },
      ],
      body: '{"model":"claude-opus-4"}',
    },
    forwardRequest: {
      method: "POST",
      url: "https://up.example/v1/chat/completions",
      statusCode: null,
      headers: [{ key: "authorization", value: "[redacted]" }],
      body: '{"model":"gpt-5.5"}',
    },
    receivedForwardedRequest: {
      method: null,
      url: "https://up.example/v1/chat/completions",
      statusCode: 200,
      headers: [{ key: "content-type", value: "application/json" }],
      body: '{"id":"chatcmpl_123"}',
    },
    responseRequest: {
      method: null,
      url: "/v1/messages",
      statusCode: 200,
      headers: [{ key: "content-type", value: "application/json" }],
      body: '{"id":"msg_123"}',
    },
  },
};

describe("RequestTracePanel", () => {
  it("renders four trace stages with request details", () => {
    render(<RequestTracePanel log={tracedLog} />);

    expect(screen.getByText("接收请求")).toBeInTheDocument();
    expect(screen.getByText("转发请求")).toBeInTheDocument();
    expect(screen.getByText("接收上游响应")).toBeInTheDocument();
    expect(screen.getByText("响应客户端")).toBeInTheDocument();

    expect(screen.getAllByText("/v1/messages")).toHaveLength(2);
    expect(screen.getAllByText("https://up.example/v1/chat/completions")).toHaveLength(2);
    expect(screen.getByText((content) => content.includes('"id":"msg_123"'))).toBeInTheDocument();
    expect(screen.getAllByText("content-type")).toHaveLength(3);
  });

  it("keeps the four-stage layout visible when the selected request has no trace", () => {
    render(<RequestTracePanel log={{ ...tracedLog, id: 12, trace: null }} />);

    expect(screen.getByText("ep-a")).toBeInTheDocument();
    expect(screen.getByText("未记录详细链路")).toBeInTheDocument();
    expect(screen.getByText("接收请求")).toBeInTheDocument();
    expect(screen.getByText("转发请求")).toBeInTheDocument();
    expect(screen.getByText("接收上游响应")).toBeInTheDocument();
    expect(screen.getByText("响应客户端")).toBeInTheDocument();
  });

  it("asks the user to select a request before showing details", () => {
    render(<RequestTracePanel log={null} />);

    expect(screen.getByText("选择一条请求查看链路")).toBeInTheDocument();
  });
});
