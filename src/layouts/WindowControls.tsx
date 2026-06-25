import { useEffect, useState } from "react";
import { CopyIcon, MinusIcon, SquareIcon, XIcon } from "lucide-react";

import { IS_MAC } from "@/lib/platform";
import { cn } from "@/lib/utils";
import { isWebRuntime } from "@/services/runtime";

type TauriWindow = {
  minimize(): Promise<void>;
  toggleMaximize(): Promise<void>;
  close(): Promise<void>;
  isMaximized(): Promise<boolean>;
  onResized(cb: () => void): Promise<() => void>;
};

export function WindowControls() {
  const [appWindow, setAppWindow] = useState<TauriWindow | null>(null);
  const [maximized, setMaximized] = useState(false);

  useEffect(() => {
    if (isWebRuntime() || IS_MAC) return;

    let cancelled = false;
    let unlisten: (() => void) | undefined;

    void import("@tauri-apps/api/window").then(({ getCurrentWindow }) => {
      if (cancelled) return;

      const win = getCurrentWindow() as unknown as TauriWindow;
      setAppWindow(win);
      win.isMaximized().then(setMaximized).catch(() => undefined);
      win
        .onResized(() => {
          win.isMaximized().then(setMaximized).catch(() => undefined);
        })
        .then((dispose) => {
          unlisten = dispose;
        });
    });

    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, []);

  if (isWebRuntime() || IS_MAC || !appWindow) return null;

  const btn =
    "inline-flex h-8 w-11 cursor-pointer items-center justify-center text-ink-secondary transition-colors hover:bg-surface-hover hover:text-ink-primary";

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
        {maximized ? <CopyIcon className="size-3.5" /> : <SquareIcon className="size-3" />}
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
