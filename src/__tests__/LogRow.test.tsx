import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { LogRow } from "@/pages/Logs/_components/LogRow";
import type { LogLine } from "@/services/modules/logs";

const line: LogLine = {
  time: "10:00:00.123",
  level: "INFO",
  target: "ccmesh_lib::modules::storage::migration",
  message: "已应用数据库迁移",
  fields: [{ key: "version", value: "4" }],
};

describe("LogRow", () => {
  it("渲染等级/时间/完整来源/message/字段", () => {
    render(<LogRow line={line} keyword="" />);
    expect(screen.getByText("INFO")).toBeInTheDocument();
    expect(screen.getByText("10:00:00.123")).toBeInTheDocument();
    expect(screen.getByText("已应用数据库迁移")).toBeInTheDocument();
    expect(screen.getByText("4")).toBeInTheDocument();
    expect(screen.getByText(/storage::migration/)).toBeInTheDocument();
  });

  it("关键字命中高亮为 mark", () => {
    const { container } = render(<LogRow line={line} keyword="迁移" />);
    const marks = container.querySelectorAll("mark");
    expect(marks.length).toBe(1);
    expect(marks[0].textContent).toBe("迁移");
  });

  it("ERROR 使用警示卡片边框", () => {
    const err: LogLine = { ...line, level: "ERROR", message: "连接失败" };
    const { container } = render(<LogRow line={err} keyword="" />);
    const card = container.querySelector("article");
    expect(card?.className).toMatch(/border-destructive/);
    expect(screen.getByText("连接失败")).toHaveClass("text-destructive");
  });
});
