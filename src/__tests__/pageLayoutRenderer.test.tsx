import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { PageSectionHost } from "@/components/business/page-layout/PageSectionHost";

describe("PageSectionHost", () => {
  it("renders sections in configured order and hides disabled sections", () => {
    render(
      <PageSectionHost
        layout={{
          mode: "stack",
          sections: [
            { id: "b", visible: true },
            { id: "a", visible: false },
          ],
        }}
        registry={{
          a: { title: "A", render: () => <div>A</div> },
          b: { title: "B", render: () => <div>B</div> },
        }}
      />,
    );

    expect(screen.getByText("B")).toBeInTheDocument();
    expect(screen.queryByText("A")).not.toBeInTheDocument();
  });

  it("keeps split sections readable by defaulting them to full row width", () => {
    render(
      <PageSectionHost
        layout={{
          mode: "split",
          sections: [{ id: "a", visible: true }],
        }}
        registry={{
          a: { title: "A", render: () => <div>A</div> },
        }}
      />,
    );

    expect(screen.getByText("A").parentElement).toHaveClass("xl:col-span-12");
  });
});
