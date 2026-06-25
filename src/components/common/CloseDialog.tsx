import { useEffect, useState } from "react";

import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { configApi } from "@/services/modules/config";
import { windowApi } from "@/services/modules/window";
import { isWebRuntime } from "@/services/runtime";

export function CloseDialog() {
  const [open, setOpen] = useState(false);
  const [remember, setRemember] = useState(false);

  useEffect(() => {
    if (isWebRuntime()) return;

    let unlisten: (() => void) | undefined;

    void import("@tauri-apps/api/event").then(({ listen }) => {
      listen("close-requested", () => setOpen(true)).then((dispose) => {
        unlisten = dispose;
      });
    });

    return () => unlisten?.();
  }, []);

  if (isWebRuntime()) return null;

  const choose = async (action: "minimize" | "quit") => {
    if (remember) {
      await configApi.setConfig({ closeWindowBehavior: action }).catch(() => undefined);
    }
    setOpen(false);
    await windowApi.applyCloseAction(action).catch(() => undefined);
  };

  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>关闭窗口</DialogTitle>
          <DialogDescription>选择关闭时的行为</DialogDescription>
        </DialogHeader>
        <div className="flex items-center gap-2">
          <Switch id="remember-close" checked={remember} onCheckedChange={setRemember} />
          <Label htmlFor="remember-close">记住我的选择</Label>
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={() => choose("minimize")}>
            最小化到托盘
          </Button>
          <Button variant="destructive" onClick={() => choose("quit")}>
            退出
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
