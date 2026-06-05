import { useEffect, useState } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { json } from "@codemirror/lang-json";
import CodeMirror from "@uiw/react-codemirror";
import { useTheme } from "next-themes";
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
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { endpointApi, type Endpoint } from "@/services/modules/endpoint";

interface FormState {
  name: string;
  apiUrl: string;
  apiKey: string;
  transformer: string;
  model: string;
  remark: string;
}

const EMPTY: FormState = {
  name: "",
  apiUrl: "",
  apiKey: "",
  transformer: "claude",
  model: "",
  remark: "",
};

const errMsg = (e: unknown) => (e instanceof Error ? e.message : String(e));

interface Props {
  open: boolean;
  onOpenChange: (o: boolean) => void;
  editing: Endpoint | null;
}

export function EndpointForm({ open, onOpenChange, editing }: Props) {
  const qc = useQueryClient();
  const { resolvedTheme } = useTheme();
  const [form, setForm] = useState<FormState>(EMPTY);
  const [jsonText, setJsonText] = useState("");
  const [jsonErr, setJsonErr] = useState("");

  useEffect(() => {
    if (!open) return;
    const init: FormState = editing
      ? {
          name: editing.name,
          apiUrl: editing.apiUrl,
          apiKey: editing.apiKey,
          transformer: editing.transformer,
          model: editing.model,
          remark: editing.remark,
        }
      : EMPTY;
    setForm(init);
    setJsonText(JSON.stringify(init, null, 2));
    setJsonErr("");
  }, [open, editing]);

  const set = (k: keyof FormState, v: string) =>
    setForm((f) => {
      const next = { ...f, [k]: v };
      setJsonText(JSON.stringify(next, null, 2));
      return next;
    });

  const onJsonChange = (val: string) => {
    setJsonText(val);
    try {
      const parsed = JSON.parse(val);
      setForm((f) => ({ ...f, ...parsed }));
      setJsonErr("");
    } catch {
      setJsonErr("JSON 格式错误");
    }
  };

  const save = useMutation({
    mutationFn: () =>
      editing ? endpointApi.update(editing.id, form) : endpointApi.create(form),
    onSuccess: () => {
      toast.success(editing ? "已更新" : "已创建");
      qc.invalidateQueries({ queryKey: ["endpoints"] });
      onOpenChange(false);
    },
    onError: (e) => toast.error(errMsg(e)),
  });

  const fields: Array<{ k: keyof FormState; label: string; type?: string; ph?: string }> = [
    { k: "name", label: "名称" },
    { k: "apiUrl", label: "API URL", ph: "https://api.anthropic.com" },
    { k: "apiKey", label: "API Key", type: "password" },
    { k: "model", label: "模型（可选）" },
    { k: "remark", label: "备注（可选）" },
  ];

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-lg">
        <DialogHeader>
          <DialogTitle>{editing ? "编辑端点" : "新建端点"}</DialogTitle>
        </DialogHeader>

        <Tabs defaultValue="form">
          <TabsList>
            <TabsTrigger value="form">表单</TabsTrigger>
            <TabsTrigger value="json">JSON</TabsTrigger>
          </TabsList>

          <TabsContent value="form" className="flex flex-col gap-3">
            {fields.map((f) => (
              <div key={f.k} className="flex flex-col gap-1.5">
                <Label htmlFor={f.k}>{f.label}</Label>
                <Input
                  id={f.k}
                  type={f.type ?? "text"}
                  placeholder={f.ph}
                  value={form[f.k]}
                  onChange={(e) => set(f.k, e.target.value)}
                />
              </div>
            ))}
            <div className="flex flex-col gap-1.5">
              <Label>转换器</Label>
              <Select value={form.transformer} onValueChange={(v) => set("transformer", v)}>
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="claude">claude（直通）</SelectItem>
                  <SelectItem value="openai">openai（转换）</SelectItem>
                </SelectContent>
              </Select>
            </div>
          </TabsContent>

          <TabsContent value="json">
            <CodeMirror
              value={jsonText}
              height="240px"
              theme={resolvedTheme === "dark" ? "dark" : "light"}
              extensions={[json()]}
              onChange={onJsonChange}
            />
            {jsonErr ? <p className="mt-1 text-xs text-destructive">{jsonErr}</p> : null}
          </TabsContent>
        </Tabs>

        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            取消
          </Button>
          <Button
            onClick={() => save.mutate()}
            disabled={!!jsonErr || !form.name || !form.apiUrl || save.isPending}
          >
            保存
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
