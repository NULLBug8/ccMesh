import { useMutation, useQueryClient } from "@tanstack/react-query";
import {
  ActivityIcon,
  CopyIcon,
  GripVerticalIcon,
  PencilIcon,
  Trash2Icon,
} from "lucide-react";
import { toast } from "sonner";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Switch } from "@/components/ui/switch";
import { endpointApi, type Endpoint } from "@/services/modules/endpoint";
import { TestBadge } from "./TestBadge";

const errMsg = (e: unknown) => (e instanceof Error ? e.message : String(e));

interface Props {
  endpoint: Endpoint;
  onEdit: (e: Endpoint) => void;
  draggable: boolean;
}

export function EndpointCard({ endpoint, onEdit, draggable }: Props) {
  const qc = useQueryClient();
  const invalidate = () => qc.invalidateQueries({ queryKey: ["endpoints"] });

  const toggle = useMutation({
    mutationFn: (v: boolean) => endpointApi.update(endpoint.id, { enabled: v }),
    onSuccess: invalidate,
    onError: (e) => toast.error(errMsg(e)),
  });
  const test = useMutation({
    mutationFn: () => endpointApi.test(endpoint.id),
    onSuccess: (r) => {
      r.success
        ? toast.success(`${endpoint.name}：${r.message} (${r.latencyMs}ms)`)
        : toast.error(`${endpoint.name}：${r.message}`);
      invalidate();
    },
    onError: (e) => toast.error(errMsg(e)),
  });
  const clone = useMutation({
    mutationFn: () => endpointApi.clone(endpoint.id),
    onSuccess: () => {
      toast.success("已克隆");
      invalidate();
    },
    onError: (e) => toast.error(errMsg(e)),
  });
  const del = useMutation({
    mutationFn: () => endpointApi.remove(endpoint.id),
    onSuccess: () => {
      toast.success("已删除");
      invalidate();
    },
    onError: (e) => toast.error(errMsg(e)),
  });

  return (
    <Card>
      <CardContent className="flex items-center gap-3 px-4 py-3">
        <GripVerticalIcon
          className={`size-4 shrink-0 ${draggable ? "cursor-grab text-ink-mute" : "text-ink-disabled"}`}
        />
        <div className="flex min-w-0 flex-1 flex-col gap-1">
          <div className="flex items-center gap-2">
            <span className="truncate font-medium">{endpoint.name}</span>
            <Badge variant="muted">{endpoint.transformer}</Badge>
            <TestBadge status={endpoint.testStatus} />
          </div>
          <span className="truncate text-xs text-ink-secondary">
            {endpoint.apiUrl}
            {endpoint.model ? ` · ${endpoint.model}` : ""}
          </span>
        </div>
        <Switch
          checked={endpoint.enabled}
          onCheckedChange={(v) => toggle.mutate(v)}
          aria-label="启用"
        />
        <div className="flex gap-0.5">
          <Button
            size="icon"
            variant="ghost"
            aria-label="测试"
            onClick={() => test.mutate()}
            disabled={test.isPending}
          >
            <ActivityIcon className="size-4" />
          </Button>
          <Button size="icon" variant="ghost" aria-label="克隆" onClick={() => clone.mutate()}>
            <CopyIcon className="size-4" />
          </Button>
          <Button size="icon" variant="ghost" aria-label="编辑" onClick={() => onEdit(endpoint)}>
            <PencilIcon className="size-4" />
          </Button>
          <Button size="icon" variant="ghost" aria-label="删除" onClick={() => del.mutate()}>
            <Trash2Icon className="size-4" />
          </Button>
        </div>
      </CardContent>
    </Card>
  );
}
