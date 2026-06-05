import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { TestBadge } from "@/pages/Endpoints/_components/TestBadge";

describe("TestBadge", () => {
  it("available 显示「可用」", () => {
    render(<TestBadge status="available" />);
    expect(screen.getByText("可用")).toBeInTheDocument();
  });

  it("unavailable 显示「不可用」", () => {
    render(<TestBadge status="unavailable" />);
    expect(screen.getByText("不可用")).toBeInTheDocument();
  });

  it("未知状态回落「未测试」", () => {
    render(<TestBadge status="weird" />);
    expect(screen.getByText("未测试")).toBeInTheDocument();
  });
});
