import { request } from "../request";

export interface RoutingRules {
  strategy: string;
  modelAffinity: boolean;
  headerAffinity: boolean;
}

export interface CircuitBreakerRules {
  failureThreshold: number;
  successThreshold: number;
  timeoutSeconds: number;
  errorRateThreshold: number;
  minRequests: number;
}

export interface DegradationRules {
  enabled: boolean;
  reasoningEffortFallback: boolean;
  requestThinkingSignature: boolean;
}

export interface RulesConfig {
  routing: RoutingRules;
  circuitBreaker: CircuitBreakerRules;
  degradation: DegradationRules;
}

export const rulesApi = {
  getConfig: () => request<RulesConfig>("get_rules_config"),
  setConfig: (config: RulesConfig) => request<RulesConfig>("set_rules_config", { config }),
  resetConfig: () => request<RulesConfig>("reset_rules_config"),
};
