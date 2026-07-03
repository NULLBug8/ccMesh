# React Query 缓存治理方案

## 一、现状分析

### 1.1 QueryKey 全景

| QueryKey | 数据源 | 负责页面/组件 |
|----------|--------|---------------|
| `["endpoints"]` | `endpointApi.list` | 端点管理、仪表盘 |
| `["health"]` | `healthApi.getHealth` | 仪表盘 ServiceCard |
| `["endpoint-health"]` | `healthApi.getEndpointHealth` | 仪表盘、端点管理 |
| `["stats"]` | `statsApi.getStats` | 仪表盘、统计页 |
| `["config"]` | `configApi.getConfig` | 设置、同步、主题 |
| `["app-config"]` | `configApi.getConfig` | 配置档案页 |
| `["backups"]` | `backupApi.list` | 同步页 |
| `["request-logs", "live"]` | `statsApi.getLogs` | 仪表盘实时监控 |
| `["usage", ...]` | `usageApi.*` | 统计页用量面板 |
| `["stats-history", page]` | `statsApi.getHistory` | 统计页历史弹窗 |
| `["autostart-enabled"]` | `autostartApi.isEnabled` | 设置页 |
| `["profile-channels", app]` | `configApi.getChannels` | 配置档案页 |

### 1.2 按页面分布

#### 仪表盘 (Dashboard)

```
["health"]              → ServiceCard 端点列表 + 代理状态
["endpoint-health"]     → ServiceCard 熔断/健康状态
["stats"]               → StatCard 今日请求数/失败/Token
["request-logs", "live"] → RequestMonitor 实时请求流
```

#### 端点管理 (Endpoints)

```
["endpoints"]           → 端点列表、拖拽排序、启停、测试、克隆、删除、模型映射
["endpoint-health"]     → 端点健康状态（熔断态）
```

#### 统计页 (Statistics)

```
["stats"]               → 四周期统计卡片
["usage", ...]          → 用量汇总 + 按日/模型明细
["stats-history", page] → 历史记录分页 + 删除
```

#### 设置页 (Settings)

```
["config"]              → 全局配置（代理、端口、UA、主题等）
["autostart-enabled"]   → 开机自启状态
```

#### 同步页 (Sync)

```
["config"]              → WebDAV 配置
["backups"]             → 备份列表、导出/导入、恢复、删除
```

#### 配置档案页 (ConfigProfiles)

```
["app-config"]          → 应用配置
["profile-channels", "claude"] → Claude 渠道列表
["profile-channels", "codex"]  → Codex 渠道列表
```

### 1.3 事件驱动失效关系

```
┌─────────────────────────────┬──────────────────────────────────────┐
│ 后端事件                     │ 失效的 QueryKey                      │
├─────────────────────────────┼──────────────────────────────────────┤
│ endpoints-changed           │ ["endpoints"], ["health"],           │
│                             │ ["endpoint-health"]                  │
├─────────────────────────────┼──────────────────────────────────────┤
│ endpoint-health-changed     │ ["endpoints"], ["health"],           │
│                             │ ["endpoint-health"]                  │
├─────────────────────────────┼──────────────────────────────────────┤
│ stats-updated               │ ["stats"]                            │
├─────────────────────────────┼──────────────────────────────────────┤
│ request-logged              │ ["request-logs", "live"]             │
└─────────────────────────────┴──────────────────────────────────────┘
```

### 1.4 Mutation 失效关系

| 页面 | Mutation | 失效的 QueryKey |
|------|----------|-----------------|
| 端点管理 | `toggle` / `test` / `clone` / `delete` / `save` / `reorder` | `["endpoints"]` |
| 端点管理 | `ModelMapping.save` | `["endpoints"]` |
| 统计页 | `sync` | `["usage"]` |
| 统计页 | `delRow` / `delDay` | `["stats-history"]`, `["stats"]` |
| 同步页 | `backup` / `restore` / `del` | `["backups"]` |
| 同步页 | WebDAV 保存 | `["config"]`, `["backups"]` |
| 设置页 | 配置保存 | `["config"]`, `["app-config"]` |
| 设置页 | 自启切换 | `["autostart-enabled"]` |
| 配置档案 | `saveCh` / `applyCfg` / `delCh` / `fetchModels` | `["profile-channels", ...]` |

---

## 二、设计合理性评估

### 2.1 合理的部分

#### 共享 Hook 封装

```typescript
// 多组件共享同一 queryKey，自动去重
export function useEndpoints() {
    return useQuery({ queryKey: ["endpoints"], queryFn: endpointApi.list });
}

export function useEndpointHealth() {
    return useQuery({ queryKey: ["endpoint-health"], queryFn: healthApi.getEndpointHealth });
}
```

#### 事件驱动失效

```typescript
// 后端事件 → 前端缓存失效，保证数据新鲜
RELATED_KEYS.forEach((queryKey) => qc.invalidateQueries({ queryKey }));
```

#### Mutation 成功后失效

```typescript
// 写操作后立即失效相关查询
const toggle = useMutation({
    mutationFn: ...,
    onSuccess: () => qc.invalidateQueries({ queryKey: ["endpoints"] }),
});
```

---

### 2.2 存在的问题

#### 问题 1：`["config"]` 和 `["app-config"]` 查询相同数据

```typescript
// Settings/index.tsx
useQuery({ queryKey: ["config"], queryFn: configApi.getConfig });

// ConfigProfiles/ClaudeWorkspace.tsx
useQuery({ queryKey: ["app-config"], queryFn: configApi.getConfig });
```

**问题**：同一个 API、同一份数据，两个 queryKey，缓存不共享，会请求两次。

#### 问题 2：`["health"]` 和 `["endpoint-health"]` 粒度重叠

```typescript
// ServiceCard 同时查询两个
const { data: health } = useQuery({ queryKey: ["health"], ... });
const { data: epHealth } = useEndpointHealth(); // ["endpoint-health"]
```

| QueryKey | 返回数据 | 来源 |
|----------|----------|------|
| `["health"]` | `HealthInfo`（端点列表 + 代理状态 + 设备ID） | `get_health` |
| `["endpoint-health"]` | `EndpointHealth[]`（熔断态 + 成功率） | `get_endpoint_health` |

**评估**：设计上关注点分离合理，但 `health` 查询包含了端点列表（与 `["endpoints"]` 重复）。

#### 问题 3：`["health"]` 与 `["endpoints"]` 数据重复

```typescript
// healthApi.getHealth 返回
interface HealthInfo {
    endpoints: MaskedEndpoint[];  // ← 与 endpointApi.list 重复
    ...
}

// ServiceCard 使用 health.endpoints 而非 endpoints
const endpoints = (health?.endpoints ?? []).filter((e) => e.enabled);
```

**问题**：ServiceCard 用 `health.endpoints` 而非 `useEndpoints()`，导致端点管理页和仪表盘可能显示不一致。

#### 问题 4：失效范围过广

```typescript
// useEndpointHealth.ts
const RELATED_KEYS = [["endpoints"], ["health"], ["endpoint-health"]];
```

| 事件 | 当前失效 | 实际需要失效 | 多余 |
|------|----------|--------------|------|
| `endpoints-changed` | `["endpoints"]`, `["health"]`, `["endpoint-health"]` | `["endpoints"]` | `["endpoint-health"]` |
| `endpoint-health-changed` | `["endpoints"]`, `["health"]`, `["endpoint-health"]` | `["endpoint-health"]` | `["endpoints"]`, `["health"]` |

**问题**：配置变更不会改变熔断态，熔断变更不会改变配置，但当前会互相触发重请求。

#### 问题 5：`["request-logs", "live"]` 缺少共享 Hook

```typescript
// RequestMonitor.tsx 内联定义
const { data } = useQuery({
    queryKey: ["request-logs", "live", pageSize],
    queryFn: () => statsApi.getLogs({ limit: pageSize }),
});
```

**问题**：直接在组件内定义，没有封装成共享 Hook，扩展性差。

#### 问题 6：`["autostart-enabled"]` 粒度过细

```typescript
// Settings/index.tsx
useQuery({ queryKey: ["autostart-enabled"], queryFn: autostartApi.isEnabled });
```

**问题**：自启状态是配置的一部分，单独一个 queryKey 增加认知负担和失效管理复杂度。

---

### 2.3 问题汇总

| 严重程度 | 问题 | 影响 |
|----------|------|------|
| **中** | `["config"]` / `["app-config"]` 重复 | 多余请求，可能数据不一致 |
| **中** | 失效范围过广 | 不必要的重请求，浪费带宽 |
| **低** | `health.endpoints` 与 `["endpoints"]` 重复 | 数据来源不统一 |
| **低** | `["request-logs", "live"]` 无共享 Hook | 扩展性差 |
| **低** | `["autostart-enabled"]` 粒度过细 | 认知负担 |

---

## 三、目标架构

### 3.1 QueryKey 清单（目标）

| QueryKey | 职责 | 数据源 | 事件失效 |
|----------|------|--------|----------|
| `["endpoints"]` | 端点配置列表 | `endpointApi.list` | `endpoints-changed` |
| `["endpoint-health"]` | 端点运行时健康/熔断态 | `healthApi.getEndpointHealth` | `endpoint-health-changed` |
| `["config"]` | 全局配置（含自启、代理设置） | `configApi.getConfig` | 手动失效 |
| `["proxy-status"]` | 代理运行态（端口、当前端点） | `proxyApi.status` | `proxy-status-changed` |
| `["stats"]` | 四周期统计 | `statsApi.getStats` | `stats-updated` |
| `["request-logs", ...]` | 请求日志 | `statsApi.getLogs` | `request-logged` |
| `["usage", ...]` | 用量统计 | `usageApi.*` | 手动失效 |
| `["backups"]` | 备份列表 | `backupApi.list` | 手动失效 |
| `["profile-channels", app]` | 配置档案渠道 | `configApi.getChannels` | 手动失效 |

### 3.2 废弃/合并项

| 原 QueryKey | 处理方式 | 目标 |
|-------------|----------|------|
| `["app-config"]` | 合并 | → `["config"]` |
| `["health"]` | 拆分 | → `["proxy-status"]` + `["endpoints"]` |
| `["autostart-enabled"]` | 合并 | → `["config"]` 的派生字段 |

---

## 四、实施计划

### 阶段一：统一 config 查询

**目标**：消除 `["config"]` / `["app-config"]` 重复

**改动**：
- [ ] `ConfigProfiles/ClaudeWorkspace.tsx`: `["app-config"]` → `["config"]`
- [ ] `ConfigProfiles/CodexWorkspace.tsx`: `["app-config"]` → `["config"]`
- [ ] `Settings/index.tsx`: 失效 `["app-config"]` → `["config"]`

**影响范围**：配置档案页、设置页

---

### 阶段二：拆分 health 查询

**目标**：消除 `["health"]` 职责过多问题

**改动**：
- [ ] 新增 `proxyApi.getStatus()` 命令（返回 port, running, currentEndpoint, deviceId）
- [ ] 新增 `useProxyStatus()` Hook，queryKey 为 `["proxy-status"]`
- [ ] `ServiceCard.tsx`: 拆分为 `useProxyStatus()` + `useEndpoints()` + `useEndpointHealth()`
- [ ] 废弃 `healthApi.getHealth()` 或保留但不再前端使用 `endpoints` 字段

**影响范围**：仪表盘 ServiceCard

---

### 阶段三：精确化失效策略

**目标**：事件失效范围最小化

**改动**：
- [ ] 重写 `useEndpointHealthEvents.ts`：

```typescript
export function useEndpointEvents() {
    const qc = useQueryClient();
    useEffect(() => {
        // 配置变更 → 只失效配置相关
        endpointApi.onChanged(() => {
            qc.invalidateQueries({ queryKey: ["endpoints"] });
        });
        // 健康变更 → 只失效健康相关
        healthApi.onHealthChanged(() => {
            qc.invalidateQueries({ queryKey: ["endpoint-health"] });
        });
    }, [qc]);
}
```

- [ ] 各页面移除内联的事件订阅，统一使用共享 Hook

**影响范围**：所有使用端点/健康数据的页面

---

### 阶段四：粒度优化

**目标**：合并过细的 QueryKey

**改动**：
- [ ] `["autostart-enabled"]` → 合并到 `["config"]`，在 config 中增加 `autostart` 字段
- [ ] `["request-logs", "live"]` → 封装为 `useRequestLogs(pageSize)` 共享 Hook

**影响范围**：设置页、仪表盘实时监控

---

## 五、验证清单

### 5.1 功能验证

- [ ] 端点管理：拖拽排序后仪表盘即时更新
- [ ] 端点管理：启停/编辑后所有页面同步
- [ ] 仪表盘：熔断状态变化即时反映
- [ ] 设置页：配置修改后全局生效
- [ ] 配置档案：渠道增删改后即时刷新

### 5.2 性能验证

- [ ] 打开 DevTools Network，确认无多余重复请求
- [ ] 端点配置变更时，`/endpoint-health` 不重请求
- [ ] 熔断状态变更时，`/endpoints` 不重请求

---

## 六、风险评估

| 风险 | 概率 | 影响 | 缓解措施 |
|------|------|------|----------|
| 拆分 health 影响现有功能 | 中 | 中 | 保留旧 API，渐进迁移 |
| 失效策略变更导致数据过期 | 低 | 高 | 全量回归测试 |
| 共享 Hook 变更影响多页面 | 低 | 中 | 保持 Hook 签名不变 |
