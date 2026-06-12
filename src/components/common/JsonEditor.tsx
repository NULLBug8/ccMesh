import { useMemo } from "react";
import { json } from "@codemirror/lang-json";
import type { Extension } from "@codemirror/state";
import CodeMirror from "@uiw/react-codemirror";

import { lineHighlight } from "./lineHighlight";

interface Props {
  value: string;
  theme: "dark" | "light";
  onChange?: (val: string) => void;
  /** 只读时禁用编辑（CodeMirror editable=false）。 */
  readOnly?: boolean;
  /** 固定高度（fill=false 时生效）。 */
  height?: string;
  /** 填满父容器高度（父需有确定高度，如 flex-1 + min-h-0），内部滚动而非撑开。 */
  fill?: boolean;
  /** "json" 启用 JSON 语法高亮；"text" 为纯文本（如 TOML，无内置语言包）。 */
  lang?: "json" | "text";
  /** 高亮包含任一字符串的行（如开关开启时联动高亮对应配置行）。 */
  highlightPatterns?: string[];
}

/**
 * 通用 CodeMirror 编辑器（懒加载）。从 Endpoints 私有版本提升并增强：
 * 支持 readOnly 切换、固定高度或 fill 填满父容器、JSON/纯文本模式。
 * 供配置文件页的操作字段编辑器与整合编辑器复用。
 */
export default function JsonEditor({
  value,
  theme,
  onChange,
  readOnly = false,
  height = "240px",
  fill = false,
  lang = "json",
  highlightPatterns,
}: Props) {
  const patternKey = (highlightPatterns ?? []).join("\u0001");
  const extensions = useMemo(() => {
    const ext: Extension[] = lang === "json" ? [json()] : [];
    const patterns = patternKey ? patternKey.split("\u0001") : [];
    if (patterns.length > 0) ext.push(...lineHighlight(patterns));
    return ext;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [lang, patternKey]);

  return (
    <div
      className={
        "overflow-hidden rounded-md border border-edge" + (fill ? " h-full" : "")
      }
    >
      <CodeMirror
        value={value}
        height={fill ? "100%" : height}
        width="100%"
        theme={theme}
        editable={!readOnly}
        extensions={extensions}
        onChange={(val) => onChange?.(val)}
        className={"text-sm" + (fill ? " h-full" : "")}
        basicSetup={{ lineNumbers: true, foldGutter: false }}
      />
    </div>
  );
}
