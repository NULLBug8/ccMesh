import { lazy, Suspense, useEffect, useState } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { EyeIcon, EyeOffIcon, PlusIcon, RefreshCwIcon, XIcon } from "lucide-react";
import { useTheme } from "next-themes";
import { toast } from "sonner";

import { Badge } from "@/components/ui/badge";
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
import { Switch } from "@/components/ui/switch";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { endpointApi, type Endpoint } from "@/services/modules/endpoint";

const JsonEditor = lazy(() => import("@/components/common/JsonEditor"));

interface FormState {
  name: string;
  apiUrl: string;
  apiKey: string;
  transformer: string;
  model: string;
  models: string[];
  /** 点亮（对外公布）的模型子集：models 的子集。空数组=全部公布（兼容旧端点）。 */
  activeModels: string[];
  useProxy: boolean;
  remark: string;
}

const EMPTY: FormState = {
  name: "",
  apiUrl: "",
  apiKey: "",
  transformer: "claude",
  model: "",
  models: [],
  activeModels: [],
  useProxy: false,
  remark: "",
};

const errMsg = (e: unknown) => (e instanceof Error ? e.message : String(e));

/** 各转换器实际拼接的主请求路径（与后端 forward/test 拼法一致：base 去尾斜杠 + 完整后缀）。 */
const PATH_BY_TRANSFORMER: Record<string, string> = {
  claude: "/v1/messages",
  openai: "/v1/chat/completions",
  codex: "/v1/responses",
};

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
  const [modelInput, setModelInput] = useState("");
  const [showKey, setShowKey] = useState(false);
  const [tab, setTab] = useState("form");

  useEffect(() => {
    if (!open) return;
    const init: FormState = editing
      ? {
          name: editing.name,
          apiUrl: editing.apiUrl,
          apiKey: editing.apiKey,
          transformer: editing.transformer,
          model: editing.model,
          models: editing.models ?? [],
          activeModels: editing.activeModels ?? [],
          useProxy: editing.useProxy ?? false,
          remark: editing.remark,
        }
      : EMPTY;
    setForm(init);
    setJsonText(JSON.stringify(init, null, 2));
    setJsonErr("");
    setModelInput("");
    setShowKey(false);
    setTab("form");
  }, [open, editing]);

  const update = (patch: Partial<FormState>) =>
    setForm((f) => {
      const next = { ...f, ...patch };
      setJsonText(JSON.stringify(next, null, 2));
      return next;
    });

  const set = (k: keyof FormState, v: string) =>
    update({ [k]: v } as Partial<FormState>);

  const addModel = () => {
    const m = modelInput.trim();
    setModelInput("");
    if (!m || form.models.includes(m)) return;
    update({ models: [...form.models, m] });
  };
  const removeModel = (m: string) =>
    update({
      models: form.models.filter((x) => x !== m),
      // 移除模型时同步从点亮子集剔除，避免脏数据（后端也会规整）。
      activeModels: form.activeModels.filter((x) => x !== m),
    });

  // 点亮判定：仅 activeModels 中的模型显示为点亮。空集=未显式点亮任何项（由下方提示说明默认全部公布），
  // 这样点击某模型只影响它自身，不会牵连其它模型。
  const isLit = (m: string) => form.activeModels.includes(m);
  // 切换点亮：仅增删该模型自身；保持与 models 一致的顺序并剔除已不存在项。
  const toggleModel = (m: string) => {
    const next = form.activeModels.includes(m)
      ? form.activeModels.filter((x) => x !== m)
      : [...form.activeModels, m];
    update({ activeModels: form.models.filter((x) => next.includes(x)) });
  };

  const refresh = useMutation({
    mutationFn: () =>
      endpointApi.fetchModels(form.apiUrl, form.apiKey, form.transformer, form.useProxy),
    onSuccess: (ids) => {
      const merged = Array.from(new Set([...form.models, ...ids]));
      update({ models: merged });
      toast.success(`拉取到 ${ids.length} 个模型`);
    },
    onError: (e) => toast.error(errMsg(e)),
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

  const fields: Array<{ k: keyof FormState; label: string; ph?: string }> = [
    { k: "name", label: "名称" },
    { k: "apiUrl", label: "API URL", ph: "https://api.anthropic.com" },
    { k: "apiKey", label: "API Key" },
    { k: "model", label: "锁定模型（可选，填则强制覆盖请求 model）" },
    { k: "remark", label: "备注（可选）" },
  ];

  // api_url 辅助提示：按所选转换器实时预览完整请求地址；/v1 结尾会与后端追加的后缀叠成 /v1/v1。
  const apiUrlBase = form.apiUrl.trim().replace(/\/+$/, "");
  const hasV1Suffix = /\/v1$/i.test(apiUrlBase);
  const previewPath = PATH_BY_TRANSFORMER[form.transformer] ?? PATH_BY_TRANSFORMER.claude;

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-lg overflow-x-hidden">
        <DialogHeader>
          <DialogTitle>{editing ? "编辑端点" : "新建端点"}</DialogTitle>
        </DialogHeader>

        <Tabs value={tab} onValueChange={setTab} className="min-w-0 overflow-hidden">
          <TabsList>
            <TabsTrigger value="form">表单</TabsTrigger>
            <TabsTrigger value="json">JSON</TabsTrigger>
          </TabsList>

          <TabsContent value="form" className="flex flex-col gap-3">
            {fields.map((f) => (
              <div key={f.k} className="flex flex-col gap-1.5">
                <Label htmlFor={f.k}>{f.label}</Label>
                {f.k === "apiKey" ? (
                  <div className="relative">
                    <Input
                      id={f.k}
                      type={showKey ? "text" : "password"}
                      placeholder={f.ph}
                      value={form.apiKey}
                      onChange={(e) => set(f.k, e.target.value)}
                      className="pr-9"
                    />
                    <button
                      type="button"
                      onClick={() => setShowKey((v) => !v)}
                      aria-label={showKey ? "隐藏密钥" : "查看密钥"}
                      className="absolute inset-y-0 right-0 flex items-center px-2.5 text-ink-mute hover:text-ink-secondary"
                    >
                      {showKey ? (
                        <EyeOffIcon className="size-4" />
                      ) : (
                        <EyeIcon className="size-4" />
                      )}
                    </button>
                  </div>
                ) : (
                  <Input
                    id={f.k}
                    type="text"
                    placeholder={f.ph}
                    value={form[f.k] as string}
                    onChange={(e) => set(f.k, e.target.value)}
                  />
                )}
                {f.k === "apiUrl" &&
                  (hasV1Suffix ? (
                    <p className="px-1 text-xs text-destructive">
                      URL 不应以 /v1 结尾：实际请求会拼成 {apiUrlBase}
                      {previewPath}，出现重复的 /v1，请去掉结尾的 /v1
                    </p>
                  ) : (
                    <p className="px-1 text-xs text-ink-mute">
                      完整请求地址：{apiUrlBase || "{url}"}
                      {previewPath}（随转换器类型变化）
                    </p>
                  ))}
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
                  <SelectItem value="codex">codex（Responses）</SelectItem>
                </SelectContent>
              </Select>
            </div>

            <div className="flex flex-col gap-1.5">
              <div className="flex items-center justify-between">
                <Label>模型清单（点击标签切换点亮，亮=对外公布）</Label>
                <span className="text-xs text-ink-mute">
                  共 {form.models.length}
                  {form.models.length > 0 && `，点亮 ${form.activeModels.length}`}
                </span>
              </div>
              <div className="flex gap-2">
                <Input
                  placeholder="自定义模型名，回车或 + 添加"
                  value={modelInput}
                  onChange={(e) => setModelInput(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter") {
                      e.preventDefault();
                      addModel();
                    }
                  }}
                />
                <Button
                  type="button"
                  variant="outline"
                  size="icon"
                  onClick={addModel}
                  aria-label="添加模型"
                >
                  <PlusIcon className="size-4" />
                </Button>
                <Button
                  type="button"
                  variant="outline"
                  size="icon"
                  onClick={() => refresh.mutate()}
                  disabled={refresh.isPending || !form.apiUrl}
                  aria-label="刷新拉取模型"
                >
                  <RefreshCwIcon className="size-4" />
                </Button>
              </div>
              {form.models.length > 0 && (
                <>
                  <div className="flex max-h-40 flex-wrap gap-1.5 overflow-auto rounded-md border border-edge p-2">
                    {form.models.map((m) => {
                      const lit = isLit(m);
                      return (
                        <Badge
                          key={m}
                          variant={lit ? "default" : "muted"}
                          className={lit ? "gap-1" : "gap-1 opacity-60"}
                        >
                          <button
                            type="button"
                            onClick={() => toggleModel(m)}
                            aria-label={`${lit ? "取消点亮" : "点亮"} ${m}`}
                            aria-pressed={lit}
                            className="cursor-pointer"
                          >
                            {m}
                          </button>
                          <button
                            type="button"
                            onClick={() => removeModel(m)}
                            aria-label={`移除 ${m}`}
                            className="cursor-pointer"
                          >
                            <XIcon className="size-3" />
                          </button>
                        </Badge>
                      );
                    })}
                  </div>
                  <div className="flex items-center justify-between">
                    <span className="text-xs text-ink-mute">
                      全部未点亮时默认全部公布（兼容旧配置）
                    </span>
                    <button
                      type="button"
                      className="text-xs text-ink-mute hover:text-ink-secondary"
                      onClick={() => update({ models: [], activeModels: [] })}
                    >
                      清除全部
                    </button>
                  </div>
                </>
              )}
            </div>

            <div className="flex items-center justify-between">
              <Label>启用代理（经设置中的全局代理地址出网）</Label>
              <Switch
                checked={form.useProxy}
                onCheckedChange={(v) => update({ useProxy: v })}
              />
            </div>
          </TabsContent>

          <TabsContent value="json" className="w-full min-w-0 overflow-hidden">
            {tab === "json" ? (
              <Suspense
                fallback={
                  <div className="flex h-[240px] items-center justify-center text-xs text-ink-mute">
                    加载编辑器…
                  </div>
                }
              >
                <JsonEditor
                  value={jsonText}
                  theme={resolvedTheme === "dark" ? "dark" : "light"}
                  onChange={onJsonChange}
                />
              </Suspense>
            ) : null}
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
