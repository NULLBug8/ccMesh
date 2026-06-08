import { WindowControls } from "./WindowControls";

/** 无边框窗口自定义标题栏：左侧可拖拽区，右侧窗口控制按钮。 */
export function TitleBar() {
  return (
    <div
      data-tauri-drag-region
      className="flex h-8 shrink-0 select-none items-center justify-between border-b border-edge-subtle bg-surface pl-3"
    >
      <span
        data-tauri-drag-region
        className="text-xs font-medium tracking-tight text-ink-mute"
      >
        ccNexus
      </span>
      <WindowControls />
    </div>
  );
}
