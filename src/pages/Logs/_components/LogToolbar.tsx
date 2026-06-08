import { CopyIcon, SearchIcon, Trash2Icon } from "lucide-react";

import { TabularText } from "@/components/ui";
import { Button } from "@/components/ui/button";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { CAPTURE_LEVELS, LOG_LEVELS } from "./logLevels";

interface Props {
  selected: Set<string>;
  onToggleLevel: (level: string) => void;
  onShowAll: () => void;
  counts: Record<string, number>;
  total: number;
  keyword: string;
  onKeyword: (s: string) => void;
  captureLevel: string;
  onCaptureLevel: (level: string) => void;
  onCopy: () => void;
  onClear: () => void;
}

function chip(active: boolean): string {
  return `inline-flex items-center gap-1 rounded-full border px-2 py-0.5 text-xs transition-colors ${
    active
      ? "border-primary bg-primary/10 text-foreground"
      : "border-edge text-ink-secondary hover:bg-surface-hover"
  }`;
}

/** 日志工具栏：等级过滤 chips(含计数) + 搜索 + 捕获等级开关 + 复制/清空。 */
export function LogToolbar({
  selected,
  onToggleLevel,
  onShowAll,
  counts,
  total,
  keyword,
  onKeyword,
  captureLevel,
  onCaptureLevel,
  onCopy,
  onClear,
}: Props) {
  return (
    <div className="flex flex-col gap-2">
      <div className="flex flex-wrap items-center gap-2">
        <button type="button" onClick={onShowAll} className={chip(selected.size === 0)}>
          ALL <TabularText>{total}</TabularText>
        </button>
        {LOG_LEVELS.map((lvl) => (
          <button
            key={lvl}
            type="button"
            onClick={() => onToggleLevel(lvl)}
            className={chip(selected.has(lvl))}
          >
            {lvl} <TabularText>{counts[lvl] ?? 0}</TabularText>
          </button>
        ))}

        <div className="ml-auto flex items-center gap-2">
          <Select value={captureLevel} onValueChange={onCaptureLevel}>
            <SelectTrigger className="h-8 w-32" title="捕获等级（低于此级别的日志不记录）">
              <span className="text-xs text-ink-mute">捕获</span>
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {CAPTURE_LEVELS.map((l) => (
                <SelectItem key={l} value={l}>
                  {l}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
          <Button variant="outline" size="sm" onClick={onCopy}>
            <CopyIcon className="size-4" /> 复制
          </Button>
          <Button variant="outline" size="sm" onClick={onClear}>
            <Trash2Icon className="size-4" /> 清空
          </Button>
        </div>
      </div>

      <div className="relative">
        <SearchIcon className="absolute top-1/2 left-2 size-4 -translate-y-1/2 text-ink-mute" />
        <input
          value={keyword}
          onChange={(e) => onKeyword(e.target.value)}
          placeholder="搜索 message / 来源 / 字段…"
          className="h-8 w-full rounded-md border border-edge bg-transparent pr-2 pl-8 text-sm outline-none focus-visible:border-ring focus-visible:ring-[3px] focus-visible:ring-ring/50"
        />
      </div>
    </div>
  );
}
