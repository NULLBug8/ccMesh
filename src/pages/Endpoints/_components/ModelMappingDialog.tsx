import { useEffect, useState } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { ArrowRightIcon, PlusIcon, XIcon } from "lucide-react";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  endpointApi,
  litOutboundModels,
  type Endpoint,
  type ModelMapping,
} from "@/services/modules/endpoint";

const errMsg = (e: unknown) => (e instanceof Error ? e.message : String(e));

interface Props {
  open: boolean;
  onOpenChange: (o: boolean) => void;
  endpoint: Endpoint;
}

/** 端点模型映射弹窗：左=入站(手输) 中=→ 右=出站(仅该端点可用模型)，支持多条。 */
export function ModelMappingDialog({ open, onOpenChange, endpoint }: Props) {
  const qc = useQueryClient();
  // 出站候选按点亮模型过滤（未点亮任何项时回退全部），与对外公布口径一致。
  const outbound = litOutboundModels(endpoint);
  const [rows, setRows] = useState<ModelMapping[]>([]);

  useEffect(() => {
    if (open) setRows(endpoint.modelMappings ?? []);
  }, [open, endpoint]);

  const addRow = () => setRows((r) => [...r, { from: "", to: outbound[0] ?? "" }]);
  const removeRow = (i: number) => setRows((r) => r.filter((_, idx) => idx !== i));
  const setFrom = (i: number, v: string) =>
    setRows((r) => r.map((row, idx) => (idx === i ? { ...row, from: v } : row)));
  const setTo = (i: number, v: string) =>
    setRows((r) => r.map((row, idx) => (idx === i ? { ...row, to: v } : row)));

  const save = useMutation({
    mutationFn: () => {
      const cleaned = rows
        .map((r) => ({ from: r.from.trim(), to: r.to.trim() }))
        .filter((r) => r.from && r.to);
      return endpointApi.update(endpoint.id, { modelMappings: cleaned });
    },
    onSuccess: () => {
      toast.success("已保存模型映射");
      qc.invalidateQueries({ queryKey: ["endpoints"] });
      onOpenChange(false);
    },
    onError: (e) => toast.error(errMsg(e)),
  });

  const noModels = outbound.length === 0;

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-lg">
        <DialogHeader>
          <DialogTitle>模型映射 · {endpoint.name}</DialogTitle>
        </DialogHeader>

        <p className="text-xs text-ink-mute">
          客户端用「入站模型」请求，网关转发上游时改写为「出站模型」。出站只能选该端点点亮的模型（未点亮任何项时为全部）。
        </p>

        {noModels ? (
          <p className="rounded-md border border-edge bg-surface-raised px-3 py-2 text-sm text-ink-secondary">
            该端点暂无可用的点亮模型，请先在端点中配置模型清单（并点亮）或锁定模型，再添加映射。
          </p>
        ) : (
          <div className="flex flex-col gap-2">
            <div className="flex items-center gap-2 px-1 text-xs text-ink-mute">
              <span className="flex-1">入站模型（手动输入）</span>
              <span className="w-5" />
              <span className="flex-1">出站模型（可用模型）</span>
              <span className="w-8" />
            </div>

            {rows.length === 0 ? (
              <p className="px-1 text-sm text-ink-mute">暂无映射，点击下方「添加映射」。</p>
            ) : (
              rows.map((row, i) => (
                <div key={i} className="flex items-center gap-2">
                  <Input
                    className="flex-1"
                    placeholder="如 gpt-5"
                    value={row.from}
                    onChange={(e) => setFrom(i, e.target.value)}
                  />
                  <ArrowRightIcon className="size-4 shrink-0 text-ink-mute" />
                  <div className="flex-1">
                    <Select value={row.to} onValueChange={(v) => setTo(i, v)}>
                      <SelectTrigger className="w-full">
                        <SelectValue placeholder="选择出站模型" />
                      </SelectTrigger>
                      <SelectContent>
                        {outbound.map((m) => (
                          <SelectItem key={m} value={m} className="font-mono text-xs">
                            {m}
                          </SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                  </div>
                  <Button
                    type="button"
                    size="icon"
                    variant="ghost"
                    aria-label="移除该映射"
                    onClick={() => removeRow(i)}
                  >
                    <XIcon className="size-4" />
                  </Button>
                </div>
              ))
            )}

            <Button
              type="button"
              variant="outline"
              size="sm"
              className="self-start"
              onClick={addRow}
            >
              <PlusIcon className="size-4" />
              添加映射
            </Button>
          </div>
        )}

        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            取消
          </Button>
          <Button onClick={() => save.mutate()} disabled={noModels || save.isPending}>
            保存
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
