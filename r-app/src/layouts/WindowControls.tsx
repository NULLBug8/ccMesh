import { useEffect, useState } from "react";
import { MinusIcon, SquareIcon, CopyIcon, XIcon } from "lucide-react";
import { getCurrentWindow } from "@tauri-apps/api/window";

import { cn } from "@/lib/utils";

const appWindow = getCurrentWindow();

export function WindowControls() {
  const [maximized, setMaximized] = useState(false);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    appWindow.isMaximized().then(setMaximized).catch(() => {});
    appWindow
      .onResized(() => {
        appWindow.isMaximized().then(setMaximized).catch(() => {});
      })
      .then((un) => {
        unlisten = un;
      });
    return () => unlisten?.();
  }, []);

  const btn =
    "inline-flex h-8 w-11 items-center justify-center text-ink-secondary transition-colors hover:bg-surface-hover hover:text-ink-primary cursor-pointer";

  return (
    <div className="flex items-center">
      <button
        type="button"
        aria-label="最小化"
        className={btn}
        onClick={() => appWindow.minimize()}
      >
        <MinusIcon className="size-3.5" />
      </button>
      <button
        type="button"
        aria-label={maximized ? "还原" : "最大化"}
        className={btn}
        onClick={() => appWindow.toggleMaximize()}
      >
        {maximized ? (
          <CopyIcon className="size-3.5" />
        ) : (
          <SquareIcon className="size-3" />
        )}
      </button>
      <button
        type="button"
        aria-label="关闭"
        className={cn(btn, "hover:bg-destructive hover:text-white")}
        onClick={() => appWindow.close()}
      >
        <XIcon className="size-4" />
      </button>
    </div>
  );
}
