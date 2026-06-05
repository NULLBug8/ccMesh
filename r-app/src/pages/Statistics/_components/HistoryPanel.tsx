import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Trash2Icon } from "lucide-react";
import { toast } from "sonner";

import { TabularText } from "@/components/ui";
import { Button } from "@/components/ui/button";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { statsApi } from "@/services/modules/stats";

/** 历史归档：按月查看 / 删除统计。 */
export function HistoryPanel() {
  const qc = useQueryClient();
  const [month, setMonth] = useState("");

  const months = useQuery({
    queryKey: ["archive-months"],
    queryFn: statsApi.getArchiveMonths,
  });
  const data = useQuery({
    queryKey: ["archive", month],
    queryFn: () => statsApi.getMonthlyArchive(month),
    enabled: month !== "",
  });
  const del = useMutation({
    mutationFn: () => statsApi.deleteMonthlyStats(month),
    onSuccess: (n) => {
      toast.success(`已删除 ${month} 的 ${n} 条记录`);
      setMonth("");
      qc.invalidateQueries({ queryKey: ["archive-months"] });
      qc.invalidateQueries({ queryKey: ["stats"] });
    },
    onError: (e) =>
      toast.error(`删除失败：${e instanceof Error ? e.message : String(e)}`),
  });

  const monthList = months.data ?? [];
  const rows = data.data ?? [];

  return (
    <section className="flex flex-col gap-3">
      <div className="flex items-center justify-between">
        <h2 className="text-sm font-medium text-ink-secondary">历史归档</h2>
        <div className="flex items-center gap-2">
          <Select value={month} onValueChange={setMonth}>
            <SelectTrigger className="w-40">
              <SelectValue placeholder="选择月份" />
            </SelectTrigger>
            <SelectContent>
              {monthList.map((m) => (
                <SelectItem key={m} value={m}>
                  {m}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
          <Button
            variant="destructive"
            size="sm"
            disabled={month === "" || del.isPending}
            onClick={() => del.mutate()}
          >
            <Trash2Icon className="size-4" /> 删除该月
          </Button>
        </div>
      </div>

      {month !== "" &&
        (rows.length > 0 ? (
          <div className="overflow-hidden rounded-lg border border-edge">
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b border-edge text-xs text-ink-secondary">
                  <th className="px-4 py-2 text-left font-medium">日期</th>
                  <th className="px-4 py-2 text-left font-medium">端点</th>
                  <th className="px-4 py-2 text-right font-medium">请求</th>
                  <th className="px-4 py-2 text-right font-medium">错误</th>
                  <th className="px-4 py-2 text-right font-medium">Token</th>
                </tr>
              </thead>
              <tbody>
                {rows.map((r, i) => (
                  <tr
                    key={`${r.date}-${r.endpointName}-${i}`}
                    className="border-b border-edge-subtle last:border-0"
                  >
                    <td className="px-4 py-2">
                      <TabularText>{r.date}</TabularText>
                    </td>
                    <td className="px-4 py-2">{r.endpointName}</td>
                    <td className="px-4 py-2 text-right">
                      <TabularText>{r.requests}</TabularText>
                    </td>
                    <td className="px-4 py-2 text-right">
                      <TabularText>{r.errors}</TabularText>
                    </td>
                    <td className="px-4 py-2 text-right">
                      <TabularText>{r.inputTokens + r.outputTokens}</TabularText>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        ) : (
          <p className="text-sm text-ink-mute">该月暂无数据</p>
        ))}
    </section>
  );
}
