import type { ReactNode } from "react";

import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Button } from "@/components/ui/button";
import { Switch } from "@/components/ui/switch";
import type { RulesConfig } from "@/services/modules/rules";

interface Props {
  section: "routing" | "circuitBreaker" | "degradation";
  value: RulesConfig;
  onChange: (next: RulesConfig) => void;
}

function Block({
  title,
  description,
  children,
}: {
  title: string;
  description: string;
  children: ReactNode;
}) {
  return (
    <section className="rounded-lg border border-edge bg-surface-raised/40 p-5">
      <div className="mb-4">
        <h2 className="text-sm font-medium text-foreground">{title}</h2>
        <p className="text-xs text-ink-mute">{description}</p>
      </div>
      <div className="flex flex-col gap-4">{children}</div>
    </section>
  );
}

function Field({
  label,
  description,
  example,
  children,
}: {
  label: string;
  description: string;
  example?: string;
  children: ReactNode;
}) {
  return (
    <div className="flex items-center justify-between gap-4 rounded-md border border-edge-subtle bg-background/70 px-4 py-3">
      <div className="flex-1">
        <div className="text-sm">{label}</div>
        <div className="text-xs text-ink-mute">{description}</div>
        {example ? <div className="mt-1 text-xs text-primary-soft">{example}</div> : null}
      </div>
      <div className="shrink-0">{children}</div>
    </div>
  );
}

function parseCodes(value: string): number[] {
  return value
    .split(",")
    .map((part) => Number(part.trim()))
    .filter((code) => Number.isFinite(code) && code > 0);
}

export function RulesForm({ section, value, onChange }: Props) {
  if (section === "routing") {
    return (
      <Block title="路由规则" description="控制默认选路策略与模型、请求头亲和性。">
        <Field
          label="策略"
          description="控制默认候选端点选择方式。"
          example="示例：balanced 表示按当前轮转位置在可用端点中均衡转发。"
        >
          <Select
            value={value.routing.strategy}
            onValueChange={(strategy) =>
              onChange({
                ...value,
                routing: {
                  ...value.routing,
                  strategy,
                },
              })
            }
          >
            <SelectTrigger className="w-44">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="balanced">均衡轮询</SelectItem>
              <SelectItem value="sticky">粘性优先</SelectItem>
              <SelectItem value="manual">手动选择</SelectItem>
            </SelectContent>
          </Select>
        </Field>

        <Field
          label="模型亲和"
          description="同模型请求优先命中已支持该模型的端点。"
          example="示例：请求 GPT-5.5 时，只在声明 GPT-5.5 或映射 GPT-5.5 的站点中轮转。"
        >
          <Switch
            checked={value.routing.modelAffinity}
            onCheckedChange={(modelAffinity) =>
              onChange({
                ...value,
                routing: {
                  ...value.routing,
                  modelAffinity,
                },
              })
            }
            aria-label="model-affinity"
          />
        </Field>

        <Field
          label="请求头亲和"
          description="保留显式端点头优先级，避免被轮转逻辑覆盖。"
          example="示例：请求头 X-CCmomo-Endpoint: daily relay 会优先指定该站点。"
        >
          <Switch
            checked={value.routing.headerAffinity}
            onCheckedChange={(headerAffinity) =>
              onChange({
                ...value,
                routing: {
                  ...value.routing,
                  headerAffinity,
                },
              })
            }
            aria-label="header-affinity"
          />
        </Field>

        <Field
          label="模型映射策略"
          description="决定原生模型和映射模型的尝试顺序。默认站点优先。"
          example="示例：站点 A 同时配置原生 GPT-5.5 和映射 GPT-5.5 时，站点优先会先尝试 A；全局原生优先会先尝试所有原生 GPT-5.5。"
        >
          <div className="flex gap-2">
            <Button
              type="button"
              variant={value.routing.modelMappingStrategy === "site-first" ? "default" : "outline"}
              onClick={() =>
                onChange({
                  ...value,
                  routing: {
                    ...value.routing,
                    modelMappingStrategy: "site-first",
                  },
                })
              }
            >
              站点优先
            </Button>
            <Button
              type="button"
              variant={
                value.routing.modelMappingStrategy === "global-native-first"
                  ? "default"
                  : "outline"
              }
              onClick={() =>
                onChange({
                  ...value,
                  routing: {
                    ...value.routing,
                    modelMappingStrategy: "global-native-first",
                  },
                })
              }
            >
              全局原生优先
            </Button>
          </div>
        </Field>

        <Field
          label="最大重试预算"
          description="0 表示使用系统按候选数计算的默认重试次数。"
          example="示例：3 表示一次请求最多尝试 3 个上游候选。"
        >
          <Input
            type="number"
            className="w-28"
            aria-label="max-retries"
            value={String(value.routing.maxRetries)}
            onChange={(event) =>
              onChange({
                ...value,
                routing: {
                  ...value.routing,
                  maxRetries: Number(event.target.value || 0),
                },
              })
            }
          />
        </Field>

        <Field
          label="请求超时"
          description="0 表示使用代理默认超时。"
          example="示例：30 表示单次上游请求最多等待 30 秒。"
        >
          <Input
            type="number"
            className="w-28"
            aria-label="request-timeout-seconds"
            value={String(value.routing.requestTimeoutSeconds)}
            onChange={(event) =>
              onChange({
                ...value,
                routing: {
                  ...value.routing,
                  requestTimeoutSeconds: Number(event.target.value || 0),
                },
              })
            }
          />
        </Field>
      </Block>
    );
  }

  if (section === "circuitBreaker") {
    return (
      <Block title="熔断规则" description="控制失败阈值、恢复阈值、超时和错误率门槛。">
        <div className="grid gap-4 md:grid-cols-2">
          <Field
            label="失败阈值"
            description="连续失败达到该值后打开熔断。"
            example="示例：4 表示同一站点连续 4 次失败后进入熔断。"
          >
            <Input
              type="number"
              className="w-28"
              aria-label="failure-threshold"
              value={String(value.circuitBreaker.failureThreshold)}
              onChange={(event) =>
                onChange({
                  ...value,
                  circuitBreaker: {
                    ...value.circuitBreaker,
                    failureThreshold: Number(event.target.value || 0),
                  },
                })
              }
            />
          </Field>

          <Field
            label="恢复阈值"
            description="半开状态下成功达到该值后恢复闭合。"
            example="示例：2 表示半开探测连续成功 2 次后恢复使用。"
          >
            <Input
              type="number"
              className="w-28"
              aria-label="success-threshold"
              value={String(value.circuitBreaker.successThreshold)}
              onChange={(event) =>
                onChange({
                  ...value,
                  circuitBreaker: {
                    ...value.circuitBreaker,
                    successThreshold: Number(event.target.value || 0),
                  },
                })
              }
            />
          </Field>

          <Field
            label="冷却时间"
            description="熔断后等待多久再进入半开探测。"
            example="示例：60 表示熔断 60 秒后允许一次半开探测。"
          >
            <Input
              type="number"
              className="w-28"
              aria-label="timeout-seconds"
              value={String(value.circuitBreaker.timeoutSeconds)}
              onChange={(event) =>
                onChange({
                  ...value,
                  circuitBreaker: {
                    ...value.circuitBreaker,
                    timeoutSeconds: Number(event.target.value || 0),
                  },
                })
              }
            />
          </Field>

          <Field
            label="错误率阈值"
            description="请求量达到最小样本后，错误率超过该值触发熔断。"
            example="示例：0.6 表示最近样本错误率超过 60% 时熔断。"
          >
            <Input
              type="number"
              step="0.05"
              className="w-28"
              aria-label="error-rate-threshold"
              value={String(value.circuitBreaker.errorRateThreshold)}
              onChange={(event) =>
                onChange({
                  ...value,
                  circuitBreaker: {
                    ...value.circuitBreaker,
                    errorRateThreshold: Number(event.target.value || 0),
                  },
                })
              }
            />
          </Field>

          <Field
            label="最小样本"
            description="参与错误率判断的最小请求数。"
            example="示例：10 表示至少积累 10 次请求后才计算错误率。"
          >
            <Input
              type="number"
              className="w-28"
              aria-label="min-requests"
              value={String(value.circuitBreaker.minRequests)}
              onChange={(event) =>
                onChange({
                  ...value,
                  circuitBreaker: {
                    ...value.circuitBreaker,
                    minRequests: Number(event.target.value || 0),
                  },
                })
              }
            />
          </Field>

          <Field
            label="失败状态码"
            description="这些 HTTP 状态码会计入熔断失败。"
            example="示例：429,500,502,503,504 表示限流和常见上游错误都计入失败。"
          >
            <Input
              className="w-48"
              aria-label="failure-status-codes"
              value={value.circuitBreaker.failureStatusCodes.join(",")}
              onChange={(event) =>
                onChange({
                  ...value,
                  circuitBreaker: {
                    ...value.circuitBreaker,
                    failureStatusCodes: parseCodes(event.target.value),
                  },
                })
              }
            />
          </Field>
        </div>
      </Block>
    );
  }

  return (
    <Block title="降级规则" description="控制响应重试时的降级与请求整流行为。">
      <Field
        label="启用降级"
        description="关闭后不执行自动降级与请求整流。"
        example="示例：关闭后 reasoning_effort 或 thinking 签名错误会直接返回，不自动重试。"
      >
        <Switch
          checked={value.degradation.enabled}
          onCheckedChange={(enabled) =>
            onChange({
              ...value,
              degradation: {
                ...value.degradation,
                enabled,
              },
            })
          }
          aria-label="degradation-enabled"
        />
      </Field>

      <Field
        label="reasoning_effort 回退"
        description="遇到不支持的上游时自动降低 effort 重试。"
        example="示例：上游不支持 high 时自动降到 medium 或移除该字段后重试。"
      >
        <Switch
          checked={value.degradation.reasoningEffortFallback}
          onCheckedChange={(reasoningEffortFallback) =>
            onChange({
              ...value,
              degradation: {
                ...value.degradation,
                reasoningEffortFallback,
              },
            })
          }
          aria-label="reasoning-effort-fallback"
        />
      </Field>

      <Field
        label="thinking 签名整流"
        description="遇到签名错误时自动清洗 thinking/signature 后重试。"
        example="示例：Claude thinking signature 报错时清洗签名字段后再转发一次。"
      >
        <Switch
          checked={value.degradation.requestThinkingSignature}
          onCheckedChange={(requestThinkingSignature) =>
            onChange({
              ...value,
              degradation: {
                ...value.degradation,
                requestThinkingSignature,
              },
            })
          }
          aria-label="request-thinking-signature"
        />
      </Field>

      <Field
        label="流式失败转非流式"
        description="预留降级项：流式不稳定时允许改为非流式重试。"
        example="示例：某些中转站 SSE 经常断流时，可以关闭 stream 后再试。"
      >
        <Switch
          checked={value.degradation.retryWithoutStream}
          onCheckedChange={(retryWithoutStream) =>
            onChange({
              ...value,
              degradation: {
                ...value.degradation,
                retryWithoutStream,
              },
            })
          }
          aria-label="retry-without-stream"
        />
      </Field>

      <Field
        label="降级温度"
        description="预留降级项：需要时可在重试请求中覆盖 temperature。"
        example="示例：0 表示降级重试时使用更稳定的确定性输出。"
      >
        <Input
          type="number"
          step="0.1"
          className="w-28"
          aria-label="fallback-temperature"
          value={String(value.degradation.fallbackTemperature)}
          onChange={(event) =>
            onChange({
              ...value,
              degradation: {
                ...value.degradation,
                fallbackTemperature: Number(event.target.value || 0),
              },
            })
          }
        />
      </Field>
    </Block>
  );
}
