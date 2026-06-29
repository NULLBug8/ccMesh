import { lazy, Suspense, useEffect, useState } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { EyeIcon, EyeOffIcon, InfoIcon, PlusIcon, RefreshCwIcon, XIcon } from "lucide-react";
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
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import {
  BALANCE_QUERY_PRESETS,
  DEFAULT_BALANCE_QUERY,
  endpointApi,
  type BalanceQueryConfig,
  type BalanceProbeTemplateResult,
  type BalanceProbeResult,
  type EndpointBalanceResult,
  type Endpoint,
} from "@/services/modules/endpoint";

const JsonEditor = lazy(() => import("@/components/common/JsonEditor"));

interface FormState {
  name: string;
  apiUrl: string;
  apiKey: string;
  transformer: string;
  model: string;
  testModel: string;
  models: string[];
  /** 点亮（对外公布）的模型子集：models 的子集。空数组=全部公布（兼容旧端点）。 */
  activeModels: string[];
  useProxy: boolean;
  balanceQuery: BalanceQueryConfig;
  remark: string;
}

const EMPTY: FormState = {
  name: "",
  apiUrl: "",
  apiKey: "",
  transformer: "claude",
  model: "",
  testModel: "",
  models: [],
  activeModels: [],
  useProxy: false,
  balanceQuery: DEFAULT_BALANCE_QUERY,
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
  const [balanceProbe, setBalanceProbe] = useState<BalanceProbeResult | null>(null);
  const [balanceTestResult, setBalanceTestResult] = useState<EndpointBalanceResult | null>(null);
  const [customProbePath, setCustomProbePath] = useState("");
  const [selectedAiModel, setSelectedAiModel] = useState("");

  useEffect(() => {
    if (!open) return;
    const init: FormState = editing
      ? {
          name: editing.name,
          apiUrl: editing.apiUrl,
          apiKey: editing.apiKey,
          transformer: editing.transformer,
          model: editing.model,
          testModel: editing.testModel ?? "",
          models: editing.models ?? [],
          activeModels: editing.activeModels ?? [],
          useProxy: editing.useProxy ?? false,
          balanceQuery: editing.balanceQuery ?? DEFAULT_BALANCE_QUERY,
          remark: editing.remark,
        }
      : EMPTY;
    setForm(init);
    setJsonText(JSON.stringify(init, null, 2));
    setJsonErr("");
    setModelInput("");
    setShowKey(false);
    setTab("form");
    setBalanceProbe(null);
    setBalanceTestResult(null);
    setCustomProbePath("");
    setSelectedAiModel("");
  }, [open, editing]);

  const update = (patch: Partial<FormState>) =>
    setForm((f) => {
      const next = { ...f, ...patch };
      setJsonText(JSON.stringify(next, null, 2));
      return next;
    });

  const set = (k: keyof FormState, v: string) =>
    update({ [k]: v } as Partial<FormState>);
  const updateBalance = (patch: Partial<BalanceQueryConfig>) =>
    update({ balanceQuery: { ...form.balanceQuery, ...patch } });
  const updateExtraction = (patch: Partial<BalanceQueryConfig["extraction"]>) =>
    updateBalance({
      extraction: {
        ...form.balanceQuery.extraction,
        ...patch,
      },
    });
  const updateLimitExtraction = (
    index: number,
    patch: Partial<NonNullable<BalanceQueryConfig["extraction"]["limits"]>[number]>,
  ) => {
    const limits = [...(form.balanceQuery.extraction.limits ?? [])];
    limits[index] = { ...limits[index], ...patch };
    updateExtraction({ limits });
  };
  const removeLimitExtraction = (index: number) => {
    const limits = (form.balanceQuery.extraction.limits ?? []).filter((_, i) => i !== index);
    updateExtraction({ limits });
  };
  const addLimitExtraction = () => {
    updateExtraction({
      limits: [
        ...(form.balanceQuery.extraction.limits ?? []),
        { label: "新额度", balancePath: "", usedPath: "", expiresAtPath: "" },
      ],
    });
  };

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
      // activeModels 为空=全部公布（默认行为），保持空；已有值=用户显式点亮，保留并剔除已移除模型。
      const autoActive =
        form.activeModels.length === 0
          ? []
          : form.activeModels.filter((m) => merged.includes(m));
      update({ models: merged, activeModels: autoActive });
      toast.success(`拉取到 ${ids.length} 个模型`);
    },
    onError: (e) => toast.error(errMsg(e)),
  });

  const probeBalance = useMutation({
    mutationFn: (customPath?: string) => {
      if (!editing) throw new Error("请先保存端点后再识别余额模板");
      return endpointApi.probeBalanceTemplates(editing.id, customPath);
    },
    onSuccess: (result) => {
      setBalanceProbe(result);
      if (result.matched?.config) {
        update({ balanceQuery: { ...result.matched.config, enabled: true } });
        toast.success(`已识别余额模板：${result.matched.templateId}`);
      } else if (result.status === "sampleAvailable") {
        toast.info("余额接口有返回数据，但需要配置提取规则");
      } else {
        toast.error("内置模板 URL 均未请求成功，请填写自定义余额接口路径再探测");
      }
    },
    onError: (e) => toast.error(errMsg(e)),
  });

  const generateBalance = useMutation({
    mutationFn: (samples: BalanceProbeTemplateResult[]) => {
      if (!editing) throw new Error("请先保存端点后再生成余额模板");
      if (!selectedAiModel.trim()) throw new Error("请选择此站点下的 AI 模型");
      return endpointApi.generateBalanceTemplate(
        editing.id,
        selectedAiModel,
        samples.map((sample) => ({
          templateId: sample.templateId,
          path: sample.path,
          statusCode: sample.statusCode,
          sample: sample.sample,
        })),
      );
    },
    onSuccess: (config) => {
      update({ balanceQuery: { ...config, enabled: true } });
      toast.success("AI 已生成余额模板");
    },
    onError: (e) => toast.error(errMsg(e)),
  });

  const testBalanceTemplate = useMutation({
    mutationFn: () => {
      if (!editing) throw new Error("请先保存端点后再测试余额模板");
      return endpointApi.testBalanceTemplate(editing.id, form.balanceQuery);
    },
    onSuccess: (result) => {
      setBalanceTestResult(result);
      if (result.success) {
        toast.success("余额模板测试通过");
      } else {
        toast.error(result.message);
      }
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
    { k: "testModel", label: "连通性测试模型（可选，优先用于测试按钮）" },
    { k: "remark", label: "备注（可选）" },
  ];

  // api_url 辅助提示：按所选转换器实时预览完整请求地址；/v1 结尾会与后端追加的后缀叠成 /v1/v1。
  const apiUrlBase = form.apiUrl.trim().replace(/\/+$/, "");
  const hasV1Suffix = /\/v1$/i.test(apiUrlBase);
  const previewPath = PATH_BY_TRANSFORMER[form.transformer] ?? PATH_BY_TRANSFORMER.claude;
  const aiModels = Array.from(
    new Set([form.model, ...(form.models ?? [])].map((m) => m.trim()).filter(Boolean)),
  );
  const hasReachableBalanceProbe = balanceProbe?.results.some((item) => item.urlReachable) ?? false;

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-3xl overflow-x-hidden">
        <DialogHeader>
          <DialogTitle>{editing ? "编辑端点" : "新建端点"}</DialogTitle>
        </DialogHeader>

        <Tabs value={tab} onValueChange={setTab} className="min-w-0 overflow-hidden">
          <TabsList>
            <TabsTrigger value="form">表单</TabsTrigger>
            <TabsTrigger value="balance">余额</TabsTrigger>
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
                      {previewPath}
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
                <div className="flex items-center gap-1.5">
                  <Label>模型清单</Label>
                  <Tooltip>
                    <TooltipTrigger asChild>
                      <button
                        type="button"
                        aria-label="模型点亮说明"
                        className="text-ink-mute hover:text-ink-secondary"
                      >
                        <InfoIcon className="size-3.5" />
                      </button>
                    </TooltipTrigger>
                    <TooltipContent>通过点亮模型对外公布可用模型</TooltipContent>
                  </Tooltip>
                </div>
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
                          className="gap-1"
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
                      全部未点亮时默认全部公布
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

          <TabsContent value="balance" className="flex flex-col gap-3">
            <div className="rounded-xl border border-edge bg-surface/80 p-4">
              <div className="flex flex-col gap-3 md:flex-row md:items-start md:justify-between">
                <div>
                  <div className="text-sm font-semibold">智能识别余额模板</div>
                  <p className="mt-1 max-w-xl text-xs text-ink-mute">
                    自动尝试内置余额接口。命中后会直接填入模板；如果 URL 全部失败，不会调用 AI，只展示失败原因并允许自定义路径复探。
                  </p>
                </div>
                <Button
                  type="button"
                  variant="outline"
                  onClick={() => probeBalance.mutate(undefined)}
                  disabled={!editing || probeBalance.isPending}
                  aria-label="智能识别余额模板"
                >
                  {probeBalance.isPending ? "识别中..." : "智能识别余额模板"}
                </Button>
              </div>

              {!editing ? (
                <p className="mt-3 rounded-lg bg-warning/10 px-3 py-2 text-xs text-warning">
                  新建端点需要先保存一次，才能用真实密钥探测余额接口。
                </p>
              ) : null}

              {balanceProbe ? (
                <div className="mt-4 space-y-3">
                  {balanceProbe.status === "matched" && balanceProbe.matched ? (
                    <div className="rounded-lg border border-success/30 bg-success/10 px-3 py-2 text-sm text-success">
                      已命中 {balanceProbe.matched.templateId}
                      {balanceProbe.matched.balance ? `，余额 ${balanceProbe.matched.balance}` : ""}
                    </div>
                  ) : null}

                  {balanceProbe.status === "sampleAvailable" ? (
                    <div className="rounded-lg border border-warning/30 bg-warning/10 px-3 py-2 text-sm text-warning">
                      URL 已返回数据，但内置 JSON Path 没提取到余额。下一步可以让 AI 基于所有可用返回样本生成模板。
                    </div>
                  ) : null}

                  {balanceProbe.status === "allFailed" ? (
                    <div className="rounded-lg border border-destructive/30 bg-destructive/10 px-3 py-2 text-sm text-destructive">
                      {hasReachableBalanceProbe
                        ? "模板 URL 有返回，但没有可用于 AI 的余额样本"
                        : "全部模板 URL 都没有请求成功"}
                    </div>
                  ) : null}

                  <div className="overflow-hidden rounded-lg border border-edge-subtle">
                    <div className="grid grid-cols-[1.1fr_0.8fr_1.2fr] bg-background/70 px-3 py-2 text-xs font-medium text-ink-mute">
                      <span>模板 / 路径</span>
                      <span>状态</span>
                      <span>结果</span>
                    </div>
                    {balanceProbe.results.map((item) => (
                      <div
                        key={`${item.templateId}-${item.path}`}
                        className="grid grid-cols-[1.1fr_0.8fr_1.2fr] gap-2 border-t border-edge-subtle px-3 py-2 text-xs"
                      >
                        <div className="min-w-0">
                          <div className="truncate font-medium">{item.templateId}</div>
                          <div className="truncate text-ink-mute" title={item.path}>
                            {item.path}
                          </div>
                        </div>
                        <div>
                          {item.success
                            ? "已命中"
                            : item.urlReachable
                              ? "URL 可用"
                              : "URL 失败"}
                          {item.statusCode ? ` · HTTP ${item.statusCode}` : ""}
                        </div>
                        <div className="text-ink-mute">{item.message}</div>
                      </div>
                    ))}
                  </div>

                  {balanceProbe.status === "allFailed" ? (
                    <div className="grid gap-2 rounded-lg border border-edge-subtle bg-background/60 p-3 md:grid-cols-[1fr_auto]">
                      <div className="flex flex-col gap-1.5">
                        <Label htmlFor="custom-balance-probe-path">自定义余额接口路径</Label>
                        <Input
                          id="custom-balance-probe-path"
                          aria-label="自定义余额接口路径"
                          value={customProbePath}
                          placeholder="/api/user/self"
                          onChange={(e) => setCustomProbePath(e.target.value)}
                        />
                      </div>
                      <Button
                        type="button"
                        variant="secondary"
                        className="self-end"
                        onClick={() => probeBalance.mutate(customProbePath)}
                        disabled={!customProbePath.trim() || probeBalance.isPending}
                      >
                        用自定义路径再探测
                      </Button>
                    </div>
                  ) : null}

                  {balanceProbe.status === "sampleAvailable" ? (
                    <div className="rounded-lg border border-edge-subtle bg-background/60 p-3">
                      <p className="text-xs text-ink-mute">
                        AI 生成模板会使用当前站点自己的 API 地址、Key 和模型，请先确保此站点已经拉取或填写模型。发送给模型的样本已脱敏，不会发送 API Key。
                      </p>
                      {aiModels.length === 0 ? (
                        <div className="mt-3 rounded-lg border border-warning/30 bg-warning/10 px-3 py-2 text-sm text-warning">
                          请先在此站点下添加或拉取模型
                        </div>
                      ) : (
                        <div className="mt-3 grid gap-2 md:grid-cols-[1fr_auto]">
                          <div className="flex flex-col gap-1.5">
                            <Label htmlFor="balance-ai-model">AI 配置模型</Label>
                            <select
                              id="balance-ai-model"
                              aria-label="AI 配置模型"
                              className="h-9 rounded-sm border border-input bg-surface-raised px-3 text-sm"
                              value={selectedAiModel}
                              onChange={(e) => setSelectedAiModel(e.target.value)}
                            >
                              <option value="">请选择</option>
                              {aiModels.map((model) => (
                                <option key={model} value={model}>
                                  {model}
                                </option>
                              ))}
                            </select>
                          </div>
                          <Button
                            type="button"
                            variant="outline"
                            className="self-end"
                            onClick={() => {
                              generateBalance.mutate(balanceProbe.usableSamples);
                            }}
                            disabled={!selectedAiModel || generateBalance.isPending}
                          >
                            {generateBalance.isPending ? "生成中..." : "让 AI 生成模板"}
                          </Button>
                        </div>
                      )}
                    </div>
                  ) : null}
                </div>
              ) : null}
            </div>

            <div className="flex flex-col gap-3 rounded-lg border border-edge-subtle bg-background/70 px-4 py-3 md:flex-row md:items-center md:justify-between">
              <div>
                <div className="text-sm font-medium">启用余额查询</div>
                <p className="text-xs text-ink-mute">在端点卡片和余额查询页显示查询入口。</p>
              </div>
              <div className="flex items-center gap-3">
                <Button
                  type="button"
                  variant="outline"
                  onClick={() => testBalanceTemplate.mutate()}
                  disabled={!editing || testBalanceTemplate.isPending}
                >
                  {testBalanceTemplate.isPending ? "测试中..." : "测试当前模板"}
                </Button>
                <Switch
                  checked={form.balanceQuery.enabled}
                  onCheckedChange={(enabled) => updateBalance({ enabled })}
                  aria-label="balance-query-enabled"
                />
              </div>
            </div>
            {balanceTestResult ? (
              <div className="rounded-lg border border-edge-subtle bg-surface/80 px-4 py-3 text-sm">
                <div className={balanceTestResult.success ? "text-success" : "text-destructive"}>
                  {balanceTestResult.success
                    ? `余额 ${balanceTestResult.balance ?? "-"}${balanceTestResult.currency ? ` ${balanceTestResult.currency}` : ""}`
                    : balanceTestResult.message}
                </div>
                {balanceTestResult.limits.length > 0 ? (
                  <div className="mt-2 space-y-1 text-xs text-ink-mute">
                    {balanceTestResult.limits.map((limit) => (
                      <div key={limit.label}>
                        {limit.label}：剩余 {limit.balance ?? "-"}
                        {limit.used ? `，已用 ${limit.used}` : ""}
                        {limit.expiresAt ? `，到期 ${limit.expiresAt}` : ""}
                      </div>
                    ))}
                  </div>
                ) : null}
              </div>
            ) : null}

            <div className="flex flex-col gap-1.5">
              <Label>常见模板</Label>
              <Select
                value={form.balanceQuery.templateId}
                onValueChange={(templateId) => {
                  const preset = BALANCE_QUERY_PRESETS.find((item) => item.templateId === templateId);
                  update({
                    balanceQuery: preset
                      ? { ...preset, enabled: form.balanceQuery.enabled }
                      : { ...form.balanceQuery, templateId },
                  });
                }}
              >
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {BALANCE_QUERY_PRESETS.map((preset) => (
                    <SelectItem key={preset.templateId} value={preset.templateId}>
                      {preset.templateId}
                    </SelectItem>
                  ))}
                  <SelectItem value="custom">custom</SelectItem>
                </SelectContent>
              </Select>
            </div>

            <div className="grid gap-3 md:grid-cols-[140px_1fr]">
              <div className="flex flex-col gap-1.5">
                <Label>Method</Label>
                <Select
                  value={form.balanceQuery.method}
                  onValueChange={(method) => updateBalance({ method })}
                >
                  <SelectTrigger>
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="GET">GET</SelectItem>
                    <SelectItem value="POST">POST</SelectItem>
                  </SelectContent>
                </Select>
              </div>
              <div className="flex flex-col gap-1.5">
                <Label>查询路径</Label>
                <Input
                  value={form.balanceQuery.path}
                  placeholder="/api/user/self"
                  onChange={(e) => updateBalance({ path: e.target.value, templateId: "custom" })}
                />
              </div>
            </div>

            <div className="grid gap-3 md:grid-cols-2">
              <div className="flex flex-col gap-1.5">
                <Label>余额 JSON Path</Label>
                <Input
                  value={form.balanceQuery.extraction.balancePath}
                  placeholder="$.data.quota"
                  onChange={(e) => updateExtraction({ balancePath: e.target.value })}
                />
              </div>
              <div className="flex flex-col gap-1.5">
                <Label>货币 JSON Path</Label>
                <Input
                  value={form.balanceQuery.extraction.currencyPath}
                  placeholder="$.data.currency"
                  onChange={(e) => updateExtraction({ currencyPath: e.target.value })}
                />
              </div>
              <div className="flex flex-col gap-1.5">
                <Label>已用 JSON Path</Label>
                <Input
                  value={form.balanceQuery.extraction.usedPath}
                  placeholder="$.data.used_quota"
                  onChange={(e) => updateExtraction({ usedPath: e.target.value })}
                />
              </div>
              <div className="flex flex-col gap-1.5">
                <Label>过期时间 JSON Path</Label>
                <Input
                  value={form.balanceQuery.extraction.expiresAtPath}
                  placeholder="$.data.expires_at"
                  onChange={(e) => updateExtraction({ expiresAtPath: e.target.value })}
                />
              </div>
            </div>

            <div className="rounded-lg border border-edge-subtle bg-background/60 p-3">
              <div className="flex items-center justify-between gap-3">
                <div>
                  <div className="text-sm font-medium">多周期额度 JSON Path</div>
                  <p className="text-xs text-ink-mute">
                    用于 3 小时、一天、1 周等额外限额，AI 识别后会自动填入这里。
                  </p>
                </div>
                <Button type="button" variant="outline" onClick={addLimitExtraction}>
                  添加额度
                </Button>
              </div>
              {(form.balanceQuery.extraction.limits ?? []).length > 0 ? (
                <div className="mt-3 space-y-3">
                  {(form.balanceQuery.extraction.limits ?? []).map((limit, index) => (
                    <div
                      key={index}
                      className="grid gap-2 rounded-lg border border-edge-subtle bg-surface/70 p-3 md:grid-cols-2"
                    >
                      <div className="flex flex-col gap-1.5">
                        <Label>额度名称</Label>
                        <Input
                          value={limit.label}
                          placeholder="3小时额度"
                          onChange={(e) => updateLimitExtraction(index, { label: e.target.value })}
                        />
                      </div>
                      <div className="flex flex-col gap-1.5">
                        <Label>剩余额度 JSON Path</Label>
                        <Input
                          value={limit.balancePath}
                          placeholder="$.data.three_hour.remain"
                          onChange={(e) =>
                            updateLimitExtraction(index, { balancePath: e.target.value })
                          }
                        />
                      </div>
                      <div className="flex flex-col gap-1.5">
                        <Label>已用 JSON Path</Label>
                        <Input
                          value={limit.usedPath}
                          placeholder="$.data.three_hour.used"
                          onChange={(e) => updateLimitExtraction(index, { usedPath: e.target.value })}
                        />
                      </div>
                      <div className="flex flex-col gap-1.5">
                        <Label>重置/到期 JSON Path</Label>
                        <div className="flex gap-2">
                          <Input
                            value={limit.expiresAtPath}
                            placeholder="$.data.three_hour.reset_at"
                            onChange={(e) =>
                              updateLimitExtraction(index, { expiresAtPath: e.target.value })
                            }
                          />
                          <Button
                            type="button"
                            variant="ghost"
                            onClick={() => removeLimitExtraction(index)}
                          >
                            删除
                          </Button>
                        </div>
                      </div>
                    </div>
                  ))}
                </div>
              ) : null}
            </div>

            <div className="flex flex-col gap-1.5">
              <Label>请求体模板</Label>
              <Input
                value={form.balanceQuery.body}
                placeholder='{"user":"{{endpointName}}"}'
                onChange={(e) => updateBalance({ body: e.target.value, templateId: "custom" })}
              />
              <p className="text-xs text-ink-mute">
                可使用 {"{{apiKey}}"}、{"{{apiUrl}}"}、{"{{endpointName}}"}；本阶段不执行 JS。
              </p>
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
