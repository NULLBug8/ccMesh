import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import {
  ErrorDetail,
  fmtDateTime,
  fmtTime,
  RequestLogTable,
  RequestLogCards,
  TokenDetail,
} from "@/components/business/RequestMonitor";
import type { RequestLog } from "@/services/modules/stats";

const log: RequestLog = {
  id: 1,
  ts: Date.now(),
  endpointName: "ep-a",
  inboundFormat: "claude",
  transformer: "claude",
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
  trace: null,
};

describe("RequestLogTable", () => {
  it("renders request rows, status code, and token total", () => {
    render(<RequestLogTable items={[log]} />);
    expect(screen.getByText("ep-a")).toBeInTheDocument();
    expect(screen.getByText("200")).toBeInTheDocument();
    expect(screen.getByText("20")).toBeInTheDocument();
  });

  it("renders the actual inbound and outbound paths", () => {
    render(<RequestLogTable items={[log]} />);
    expect(screen.getByText("/v1/messages")).toBeInTheDocument();
    expect(screen.getByText("/v1/chat/completions")).toBeInTheDocument();
  });

  it("falls back to inferred paths when legacy rows have no path", () => {
    const legacy: RequestLog = {
      ...log,
      id: 2,
      inboundFormat: "openai",
      inboundPath: "",
      upstreamPath: "",
    };
    render(<RequestLogTable items={[legacy]} />);
    expect(screen.getAllByText("/v1/chat/completions")).toHaveLength(2);
  });

  it("hides duration and first-byte timing for failed requests", () => {
    const failed: RequestLog = {
      ...log,
      id: 3,
      statusCode: 500,
      isError: true,
    };
    render(<RequestLogTable items={[failed]} />);
    expect(screen.queryByText("0.12s")).not.toBeInTheDocument();
    expect(screen.queryByText("0.08s")).not.toBeInTheDocument();
    expect(screen.getAllByText("--")).toHaveLength(2);
  });

  it("shows an error detail trigger when the row contains an error body", () => {
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

  it("supports row selection", () => {
    const onSelectLog = vi.fn();
    render(
      <RequestLogTable items={[log]} selectedLogId={log.id} onSelectLog={onSelectLog} />,
    );

    const row = screen.getByTestId("request-log-row-1");
    expect(row).toHaveAttribute("aria-selected", "true");

    fireEvent.click(row);
    expect(onSelectLog).toHaveBeenCalledWith(log);
  });

  it("renders an empty state when there are no items", () => {
    render(<RequestLogTable items={[]} />);
    expect(screen.getByText("暂无请求记录")).toBeInTheDocument();
  });
});

describe("RequestLogCards", () => {
  it("renders a compact selectable card list without the wide table", () => {
    const onSelectLog = vi.fn();
    const { container } = render(
      <RequestLogCards items={[log]} selectedLogId={log.id} onSelectLog={onSelectLog} />,
    );

    expect(container.querySelector("table")).not.toBeInTheDocument();
    const card = screen.getByTestId("request-log-card-1");
    expect(card).toHaveAttribute("aria-selected", "true");
    expect(screen.getByText("ep-a")).toBeInTheDocument();
    expect(screen.getByText("/v1/messages")).toBeInTheDocument();

    fireEvent.click(card);
    expect(onSelectLog).toHaveBeenCalledWith(log);
  });

  it("can open details from an explicit table action button", () => {
    const onSelectLog = vi.fn();
    render(
      <RequestLogTable
        items={[log]}
        selectedLogId={null}
        onSelectLog={onSelectLog}
        selectionMode="button"
      />,
    );

    const row = screen.getByTestId("request-log-row-1");
    fireEvent.click(row);
    expect(onSelectLog).not.toHaveBeenCalled();

    fireEvent.click(screen.getByRole("button", { name: "查看请求详情" }));
    expect(onSelectLog).toHaveBeenCalledWith(log);
  });
});

describe("ErrorDetail", () => {
  it("formats JSON error bodies", () => {
    render(<ErrorDetail errorBody='{"error":{"code":"channel:client_restricted"}}' />);
    expect(screen.getByText(/"code": "channel:client_restricted"/)).toBeInTheDocument();
  });

  it("renders plain-text error bodies unchanged", () => {
    render(<ErrorDetail errorBody="upstream forbidden" />);
    expect(screen.getByText("upstream forbidden")).toBeInTheDocument();
  });
});

describe("fmtTime", () => {
  it("uses zero-padded 24-hour time", () => {
    const ts = new Date(2026, 5, 7, 9, 5, 3).getTime();
    expect(fmtTime(ts)).toBe("09:05:03");
  });

  it("renders midnight as 00:00:00", () => {
    const ts = new Date(2026, 5, 7, 0, 0, 0).getTime();
    expect(fmtTime(ts)).toBe("00:00:00");
  });

  it("renders late evening without AM/PM suffixes", () => {
    const ts = new Date(2026, 5, 7, 23, 59, 59).getTime();
    expect(fmtTime(ts)).toBe("23:59:59");
  });
});

describe("fmtDateTime", () => {
  it("renders a full date-time string", () => {
    const ts = new Date(2026, 5, 7, 9, 5, 3).getTime();
    expect(fmtDateTime(ts)).toBe("2026-06-07 09:05:03");
  });
});

describe("TokenDetail", () => {
  it("shows the actual mapped model when available", () => {
    const mapped: RequestLog = {
      ...log,
      model: "claude-opus-4-8",
      actualModel: "gpt-5.5",
    };
    render(<TokenDetail log={mapped} total={20} />);
    expect(screen.getByText("模型：claude-opus-4-8")).toBeInTheDocument();
    expect(screen.getByText(/实际模型/)).toBeInTheDocument();
    expect(screen.getByText("gpt-5.5").className).toContain("text-info");
  });

  it("omits the actual model row when passthrough is used", () => {
    render(<TokenDetail log={{ ...log, actualModel: null }} total={20} />);
    expect(screen.queryByText(/实际模型/)).not.toBeInTheDocument();
  });
});
