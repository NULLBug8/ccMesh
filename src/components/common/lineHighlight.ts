import { RangeSetBuilder } from "@codemirror/state";
import {
  Decoration,
  type DecorationSet,
  EditorView,
  ViewPlugin,
  type ViewUpdate,
} from "@codemirror/view";

const lineDeco = Decoration.line({ class: "cm-toggle-line" });

function buildDeco(view: EditorView, patterns: string[]): DecorationSet {
  const builder = new RangeSetBuilder<Decoration>();
  for (const { from, to } of view.visibleRanges) {
    let pos = from;
    while (pos <= to) {
      const line = view.state.doc.lineAt(pos);
      if (patterns.some((p) => p && line.text.includes(p))) {
        builder.add(line.from, line.from, lineDeco);
      }
      pos = line.to + 1;
    }
  }
  return builder.finish();
}

const toggleLineTheme = EditorView.baseTheme({
  ".cm-toggle-line": {
    backgroundColor: "rgba(34, 197, 94, 0.16)",
    transition: "background-color 200ms ease",
  },
});

/**
 * 高亮文本中包含任一 pattern 的整行（用品牌绿底）。
 * 供配置文件页：开关开启时联动高亮右侧整合编辑器中对应的配置行。
 */
export function lineHighlight(patterns: string[]) {
  const plugin = ViewPlugin.fromClass(
    class {
      decorations: DecorationSet;
      constructor(view: EditorView) {
        this.decorations = buildDeco(view, patterns);
      }
      update(u: ViewUpdate) {
        if (u.docChanged || u.viewportChanged) {
          this.decorations = buildDeco(u.view, patterns);
        }
      }
    },
    { decorations: (v) => v.decorations },
  );
  return [plugin, toggleLineTheme];
}
