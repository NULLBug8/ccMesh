import { request } from "../request";

export const windowApi = {
  /** 设置语言：持久化后端配置并重建托盘文案。 */
  setLanguage: (lang: string) => request<void>("set_language", { lang }),
  /** ask 模式关闭选择："minimize" 隐藏到托盘，"quit" 退出。 */
  applyCloseAction: (action: string) =>
    request<void>("apply_close_action", { action }),
  hideToTray: () => request<void>("hide_to_tray"),
};
