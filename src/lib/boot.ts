import { configApi } from "@/services/modules/config";
import { request } from "@/services/request";
import { isWebRuntime } from "@/services/runtime";

let revealed = false;

export async function revealMainWindow(): Promise<void> {
  if (revealed || isWebRuntime()) return;
  revealed = true;

  try {
    const silent = await configApi
      .getConfig()
      .then((config) => config.silentStart)
      .catch(() => false);
    if (silent) return;

    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    const win = getCurrentWindow();
    await win.show();
    await win.setFocus();
    await request("notify_window_shown").catch(() => undefined);
  } catch {
    // Ignore non-Tauri hosts or unavailable windows.
  }
}
