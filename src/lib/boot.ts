import { getCurrentWindow } from "@tauri-apps/api/window";

import { configApi } from "@/services/modules/config";
import { request } from "@/services/request";

let revealed = false;

/**
 * 首屏（含主题）就绪后显示主窗口，幂等。
 * 配合 tauri.conf.json 的 visible:false 消除启动白屏与主题闪烁；
 * 非 Tauri 环境（浏览器预览）静默忽略。
 *
 * 静默启动（silentStart）开启时不展示窗口，常驻托盘后台运行；
 * 用户经托盘左键/「显示窗口」唤起。配置读取失败按非静默处理，避免窗口永不出现。
 */
export async function revealMainWindow(): Promise<void> {
  if (revealed) return;
  revealed = true;
  try {
    const silent = await configApi
      .getConfig()
      .then((c) => c.silentStart)
      .catch(() => false);
    if (silent) return;
    const win = getCurrentWindow();
    await win.show();
    await win.setFocus();
    // Linux：首屏 show() 后触发窗口交互重激活，修复 WebKitGTK 整窗点击无响应；
    // 其他平台为 no-op。失败不影响窗口已显示，吞掉异常。
    await request("notify_window_shown").catch(() => {});
  } catch {
    // 浏览器预览或窗口不可用时忽略
  }
}
