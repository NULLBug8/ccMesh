import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { Pagination } from "@/components/ui/Pagination";

describe("Pagination", () => {
  it("disables previous on first page and advances to next page", () => {
    const onChange = vi.fn();
    render(<Pagination page={1} pageSize={10} total={35} onPageChange={onChange} />);

    expect(screen.getByText("35")).toBeInTheDocument();
    expect(screen.getByLabelText("上一页")).toBeDisabled();
    fireEvent.click(screen.getByLabelText("下一页"));
    expect(onChange).toHaveBeenCalledWith(2);
  });

  it("disables next on last page", () => {
    render(<Pagination page={4} pageSize={10} total={35} onPageChange={() => {}} />);

    expect(screen.getByLabelText("下一页")).toBeDisabled();
    expect(screen.getByLabelText("上一页")).not.toBeDisabled();
  });
});