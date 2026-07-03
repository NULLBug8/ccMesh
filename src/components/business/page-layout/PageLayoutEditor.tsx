import {
  ArrowDownIcon,
  ArrowUpIcon,
  EyeIcon,
  EyeOffIcon,
  RotateCcwIcon,
} from "lucide-react";

import { Button } from "@/components/ui/button";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { cn } from "@/lib/utils";
import { usePageLayoutStore, resolveViewLayout } from "@/stores";
import type { ViewId } from "@/stores";
import type { PageLayoutDefinition } from "./pageLayoutTypes";

interface Props {
  view: ViewId;
  definition: PageLayoutDefinition;
  className?: string;
}

const MODE_OPTIONS = [
  { value: "stack", label: "垂直堆叠" },
  { value: "two-column", label: "双列布局" },
  { value: "split", label: "分栏布局" },
] as const;

export function PageLayoutEditor({ view, definition, className }: Props) {
  const editing = usePageLayoutStore((state) => state.isEditing(view));
  const savedLayout = usePageLayoutStore((state) => state.getLayout(view));
  const setLayout = usePageLayoutStore((state) => state.setLayout);
  const resetView = usePageLayoutStore((state) => state.resetView);

  if (!editing) return null;

  const layout = resolveViewLayout(definition, savedLayout);

  const moveSection = (id: string, direction: -1 | 1) => {
    const index = layout.sections.findIndex((section) => section.id === id);
    const nextIndex = index + direction;
    if (index < 0 || nextIndex < 0 || nextIndex >= layout.sections.length) return;
    const sections = [...layout.sections];
    const [target] = sections.splice(index, 1);
    sections.splice(nextIndex, 0, target);
    setLayout(view, { ...layout, sections });
  };

  const toggleVisibility = (id: string) => {
    setLayout(view, {
      ...layout,
      sections: layout.sections.map((section) =>
        section.id === id ? { ...section, visible: !section.visible } : section,
      ),
    });
  };

  return (
    <section
      className={cn(
        "rounded-lg border border-dashed border-edge bg-surface-raised/70 p-4",
        className,
      )}
    >
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <h2 className="text-sm font-medium text-foreground">布局编辑</h2>
          <p className="text-xs text-ink-mute">调整区块顺序、显隐和页面布局模式。</p>
        </div>

        <div className="flex items-center gap-2">
          <Select
            value={layout.mode}
            onValueChange={(mode) =>
              setLayout(view, { ...layout, mode: mode as typeof layout.mode })
            }
          >
            <SelectTrigger className="w-36">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {MODE_OPTIONS.map((option) => (
                <SelectItem key={option.value} value={option.value}>
                  {option.label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>

          <Button variant="outline" size="sm" onClick={() => resetView(view)}>
            <RotateCcwIcon className="size-4" />
            恢复默认
          </Button>
        </div>
      </div>

      <div className="mt-4 flex flex-col gap-2">
        {layout.sections.map((section, index) => {
          const metadata = definition.sections.find((item) => item.id === section.id);
          if (!metadata) return null;
          return (
            <div
              key={section.id}
              className="flex items-center justify-between rounded-md border border-edge-subtle bg-background px-3 py-2"
            >
              <div className="flex flex-col">
                <span className="text-sm">{metadata.title}</span>
                <span className="text-xs text-ink-mute">{section.id}</span>
              </div>

              <div className="flex items-center gap-1">
                <Button
                  variant="ghost"
                  size="icon"
                  onClick={() => moveSection(section.id, -1)}
                  disabled={index === 0}
                  aria-label={`上移 ${metadata.title}`}
                >
                  <ArrowUpIcon className="size-4" />
                </Button>
                <Button
                  variant="ghost"
                  size="icon"
                  onClick={() => moveSection(section.id, 1)}
                  disabled={index === layout.sections.length - 1}
                  aria-label={`下移 ${metadata.title}`}
                >
                  <ArrowDownIcon className="size-4" />
                </Button>
                <Button
                  variant="ghost"
                  size="icon"
                  onClick={() => toggleVisibility(section.id)}
                  aria-label={`${section.visible ? "隐藏" : "显示"} ${metadata.title}`}
                >
                  {section.visible ? (
                    <EyeIcon className="size-4" />
                  ) : (
                    <EyeOffIcon className="size-4" />
                  )}
                </Button>
              </div>
            </div>
          );
        })}
      </div>
    </section>
  );
}
