import { getCurrentWindow } from "@tauri-apps/api/window";

let revealed = false;

/**
 * 首屏（含主题）就绪后显示主窗口，幂等。
 * 配合 tauri.conf.json 的 visible:false 消除启动白屏与主题闪烁；
 * 非 Tauri 环境（浏览器预览）静默忽略。
 */
export async function revealMainWindow(): Promise<void> {
  if (revealed) return;
  revealed = true;
  try {
    const win = getCurrentWindow();
    await win.show();
    await win.setFocus();
  } catch {
    // 浏览器预览或窗口不可用时忽略
  }
}
