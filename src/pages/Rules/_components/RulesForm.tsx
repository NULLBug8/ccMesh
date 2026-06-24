import type { ReactNode } from "react";

import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
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
  children,
}: {
  label: string;
  description: string;
  children: ReactNode;
}) {
  return (
    <div className="flex items-center justify-between gap-4 rounded-md border border-edge-subtle bg-background/70 px-4 py-3">
      <div className="flex-1">
        <div className="text-sm">{label}</div>
        <div className="text-xs text-ink-mute">{description}</div>
      </div>
      <div className="shrink-0">{children}</div>
    </div>
  );
}

export function RulesForm({ section, value, onChange }: Props) {
  if (section === "routing") {
    return (
      <Block title="路由规则" description="控制默认选路策略与模型、请求头亲和性。">
        <Field label="策略" description="当前默认使用轮转均衡，可预设未来扩展策略。">
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
              <SelectItem value="balanced">balanced</SelectItem>
              <SelectItem value="sticky">sticky</SelectItem>
              <SelectItem value="manual">manual</SelectItem>
            </SelectContent>
          </Select>
        </Field>

        <Field label="模型亲和" description="同模型请求优先命中已支持该模型的端点。">
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

        <Field label="请求头亲和" description="保留显式端点头优先级，避免被轮转逻辑覆盖。">
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
      </Block>
    );
  }

  if (section === "circuitBreaker") {
    return (
      <Block title="熔断规则" description="控制失败阈值、恢复阈值、超时和错误率门槛。">
        <div className="grid gap-4 md:grid-cols-2">
          <Field label="失败阈值" description="连续失败达到该值后打开熔断。">
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

          <Field label="恢复阈值" description="半开状态下成功达到该值后恢复闭合。">
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

          <Field label="冷却时间" description="熔断后等待多久再进入半开探测。">
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

          <Field label="错误率阈值" description="请求量达到最小样本后，错误率超过该值触发熔断。">
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

          <Field label="最小样本" description="参与错误率判断的最小请求数。">
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
        </div>
      </Block>
    );
  }

  return (
    <Block title="降级规则" description="控制响应重试时的降级与请求整流行为。">
      <Field label="启用降级" description="关闭后不执行自动降级与请求整流。">
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

      <Field label="reasoning_effort 回退" description="遇到不支持的上游时自动降低 effort 重试。">
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

      <Field label="thinking 签名整流" description="遇到签名错误时自动清洗 thinking/signature 后重试。">
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
    </Block>
  );
}
