import { readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";

const PAGE_FILES = [
  "src/pages/Dashboard/index.tsx",
  "src/pages/Endpoints/index.tsx",
  "src/pages/Rules/index.tsx",
  "src/pages/Settings/index.tsx",
  "src/pages/Logs/index.tsx",
  "src/pages/Statistics/index.tsx",
  "src/pages/Sync/index.tsx",
];

describe("page root width", () => {
  it("does not reintroduce narrow centered workspace containers", () => {
    const offenders = PAGE_FILES.filter((file) => {
      const source = readFileSync(join(process.cwd(), file), "utf8");
      return /className="mx-auto[^"]*max-w-/.test(source);
    });

    expect(offenders).toEqual([]);
  });
});
