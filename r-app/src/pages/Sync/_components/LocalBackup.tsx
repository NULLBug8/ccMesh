import { useState } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { DownloadIcon, UploadIcon } from "lucide-react";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { backupApi, type ImportStrategy } from "@/services/modules/backup";

const errMsg = (e: unknown) => (e instanceof Error ? e.message : String(e));

/** 本地配置导出/导入（换机迁移）：端点(含多密钥)+应用设置 JSON，原生文件对话框。 */
export function LocalBackup() {
  const qc = useQueryClient();
  const [strategy, setStrategy] = useState<ImportStrategy>("skip");

  const exportM = useMutation({
    mutationFn: () => backupApi.exportConfig(),
    onSuccess: (path) => {
      if (path) toast.success(`已导出到 ${path}`);
    },
    onError: (e) => toast.error(`导出失败：${errMsg(e)}`),
  });

  const importM = useMutation({
    mutationFn: () => backupApi.importConfig(strategy),
    onSuccess: (s) => {
      if (!s) return; // 用户取消
      toast.success(
        `导入完成：新增 ${s.endpointsAdded} · 更新 ${s.endpointsUpdated} · 跳过 ${s.endpointsSkipped} 端点，凭证 ${s.credentials}，设置 ${s.configKeys}`,
      );
      qc.invalidateQueries();
    },
    onError: (e) => toast.error(`导入失败：${errMsg(e)}`),
  });

  const busy = exportM.isPending || importM.isPending;

  return (
    <section className="flex flex-col gap-3">
      <h2 className="text-sm font-medium text-ink-secondary">本地备份 / 配置迁移</h2>
      <p className="text-xs text-ink-mute">
        导出端点(含多密钥)与应用设置为 JSON，用于换机迁移。文件含明文 API Key，请妥善保管；不含 WebDAV 同步密码。
      </p>
      <div className="flex flex-wrap items-center gap-2">
        <Button
          variant="outline"
          size="sm"
          onClick={() => exportM.mutate()}
          disabled={busy}
        >
          <DownloadIcon className="size-4" /> 导出配置
        </Button>
        <div className="flex items-center gap-2">
          <Select value={strategy} onValueChange={(v) => setStrategy(v as ImportStrategy)}>
            <SelectTrigger className="h-8 w-32">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="skip">跳过同名</SelectItem>
              <SelectItem value="overwrite">覆盖同名</SelectItem>
            </SelectContent>
          </Select>
          <Button
            variant="outline"
            size="sm"
            onClick={() => importM.mutate()}
            disabled={busy}
          >
            <UploadIcon className="size-4" /> 导入配置
          </Button>
        </div>
      </div>
    </section>
  );
}
