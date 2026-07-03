# 2038 Layout, Rules, Logs, and Web Console Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build per-page layout editing, a dedicated rules configuration page, four-stage request trace details in the logs page, and a browser-accessible web console that reuses the desktop UI.

**Architecture:** The work is split into two phases. Phase 1 extends the existing Tauri desktop architecture by adding a page-layout engine, structured rules configuration, and request trace persistence/UI. Phase 2 abstracts the frontend transport layer and exposes a browser-safe admin API and static UI host from the Rust service so the same React views can run in both Tauri and the browser.

**Tech Stack:** React 19, Zustand, TanStack Query, Tauri 2, Rust, Axum, rusqlite, Vitest

---

## File Map

**Frontend state and layout shell**

- Modify: `src/stores/modules/layout.ts`
- Create: `src/stores/modules/pageLayout.ts`
- Modify: `src/stores/index.ts`
- Create: `src/components/business/page-layout/PageLayoutEditor.tsx`
- Create: `src/components/business/page-layout/PageSectionHost.tsx`
- Create: `src/components/business/page-layout/pageLayoutTypes.ts`
- Modify: `src/layouts/SideNav.tsx`
- Modify: `src/layouts/TopNav.tsx`

**Per-page layout definitions**

- Create: `src/pages/Dashboard/layout.ts`
- Create: `src/pages/Endpoints/layout.ts`
- Create: `src/pages/ConfigProfiles/layout.ts`
- Create: `src/pages/Statistics/layout.ts`
- Create: `src/pages/Sync/layout.ts`
- Create: `src/pages/Logs/layout.ts`
- Create: `src/pages/Settings/layout.ts`
- Modify: `src/pages/Dashboard/index.tsx`
- Modify: `src/pages/Endpoints/index.tsx`
- Modify: `src/pages/Statistics/index.tsx`
- Modify: `src/pages/Logs/index.tsx`
- Modify: `src/pages/Settings/index.tsx`

**Rules configuration**

- Create: `src/pages/Rules/index.tsx`
- Create: `src/pages/Rules/_components/RulesForm.tsx`
- Create: `src/services/modules/rules.ts`
- Modify: `src/layouts/navConfig.tsx`
- Modify: `src/layouts/AppLayout.tsx`
- Modify: `src/stores/modules/layout.ts`
- Create: `src/__tests__/rulesConfig.test.ts`
- Create: `src-tauri/src/models/rules.rs`
- Modify: `src-tauri/src/models/mod.rs`
- Create: `src-tauri/src/commands/rules.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/models/config.rs`
- Modify: `src-tauri/src/modules/storage/config_repo.rs`
- Modify: `src-tauri/src/modules/proxy/circuit_breaker.rs`
- Modify: `src-tauri/src/modules/proxy/server.rs`
- Modify: `src-tauri/src/modules/transform/reasoning_effort.rs`

**Request trace persistence and logs page**

- Modify: `src/services/modules/stats.ts`
- Modify: `src/components/business/RequestMonitor.tsx`
- Create: `src/pages/Logs/_components/RequestTracePanel.tsx`
- Create: `src/__tests__/RequestTracePanel.test.tsx`
- Modify: `src-tauri/src/models/stats.rs`
- Modify: `src-tauri/src/modules/stats/aggregator.rs`
- Modify: `src-tauri/src/modules/storage/migration.rs`
- Modify: `src-tauri/src/modules/storage/request_logs_repo.rs`
- Modify: `src-tauri/src/modules/proxy/forward.rs`
- Create: `src-tauri/src/modules/proxy/trace_capture.rs`

**Web console transport and host**

- Create: `src/services/transport/types.ts`
- Create: `src/services/transport/desktop.ts`
- Create: `src/services/transport/web.ts`
- Modify: `src/services/request.ts`
- Create: `src/services/runtime.ts`
- Create: `src/main-web.tsx`
- Modify: `vite.config.ts`
- Create: `src-tauri/src/commands/web_admin.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/modules/proxy/server.rs`
- Create: `src-tauri/src/modules/web_admin/static_assets.rs`

---

### Task 1: Page Layout State and Navigation Entry

**Files:**
- Create: `src/stores/modules/pageLayout.ts`
- Modify: `src/stores/modules/layout.ts`
- Modify: `src/stores/index.ts`
- Modify: `src/layouts/SideNav.tsx`
- Modify: `src/layouts/TopNav.tsx`
- Test: `src/__tests__/pageLayoutStore.test.ts`

- [ ] **Step 1: Write the failing store test**

```tsx
import { describe, expect, it } from "vitest";
import { usePageLayoutStore } from "@/stores/modules/pageLayout";

describe("pageLayoutStore", () => {
  it("stores per-view edit mode and layout preferences independently", () => {
    const store = usePageLayoutStore.getState();
    store.resetAll();
    store.setEditMode("dashboard", true);
    store.setLayout("dashboard", {
      mode: "two-column",
      sections: [
        { id: "service", visible: true },
        { id: "stats", visible: true },
      ],
    });
    store.setLayout("logs", {
      mode: "split",
      sections: [
        { id: "log-stream", visible: true },
        { id: "request-trace", visible: true },
      ],
    });

    expect(usePageLayoutStore.getState().isEditing("dashboard")).toBe(true);
    expect(usePageLayoutStore.getState().getLayout("logs")?.mode).toBe("split");
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npm test -- src/__tests__/pageLayoutStore.test.ts`
Expected: FAIL with module-not-found for `pageLayout.ts` or missing store methods.

- [ ] **Step 3: Write the minimal layout store and nav toggle API**

```ts
export type PageLayoutMode = "stack" | "two-column" | "split";
export type PageSectionState = { id: string; visible: boolean };
export type PageLayoutConfig = {
  mode: PageLayoutMode;
  sections: PageSectionState[];
};

interface PageLayoutState {
  editModeByView: Partial<Record<ViewId, boolean>>;
  layoutByView: Partial<Record<ViewId, PageLayoutConfig>>;
  setEditMode: (view: ViewId, editing: boolean) => void;
  setLayout: (view: ViewId, layout: PageLayoutConfig) => void;
  resetView: (view: ViewId) => void;
  resetAll: () => void;
  isEditing: (view: ViewId) => boolean;
  getLayout: (view: ViewId) => PageLayoutConfig | undefined;
}
```

- [ ] **Step 4: Add the layout editor entry next to nav-mode switches**

```tsx
<Button
  variant="ghost"
  size="icon"
  aria-label="切换布局编辑"
  onClick={() => toggleEditing(activeView)}
>
  <PanelsTopLeftIcon className="size-4" />
</Button>
```

- [ ] **Step 5: Run tests and commit**

Run: `npm test -- src/__tests__/pageLayoutStore.test.ts`
Expected: PASS

```bash
git add src/stores/modules/pageLayout.ts src/stores/modules/layout.ts src/stores/index.ts src/layouts/SideNav.tsx src/layouts/TopNav.tsx src/__tests__/pageLayoutStore.test.ts
git commit -m "feat: add page layout editing state"
```

### Task 2: Shared Page Layout Engine and Page Migrations

**Files:**
- Create: `src/components/business/page-layout/pageLayoutTypes.ts`
- Create: `src/components/business/page-layout/PageLayoutEditor.tsx`
- Create: `src/components/business/page-layout/PageSectionHost.tsx`
- Create: `src/pages/Dashboard/layout.ts`
- Create: `src/pages/Endpoints/layout.ts`
- Create: `src/pages/ConfigProfiles/layout.ts`
- Create: `src/pages/Statistics/layout.ts`
- Create: `src/pages/Sync/layout.ts`
- Create: `src/pages/Logs/layout.ts`
- Create: `src/pages/Settings/layout.ts`
- Modify: `src/pages/Dashboard/index.tsx`
- Modify: `src/pages/Endpoints/index.tsx`
- Modify: `src/pages/Statistics/index.tsx`
- Modify: `src/pages/Logs/index.tsx`
- Modify: `src/pages/Settings/index.tsx`
- Test: `src/__tests__/pageLayoutRenderer.test.tsx`

- [ ] **Step 1: Write the failing renderer test**

```tsx
import { render, screen } from "@testing-library/react";
import { PageSectionHost } from "@/components/business/page-layout/PageSectionHost";

it("renders sections in configured order and hides disabled sections", () => {
  render(
    <PageSectionHost
      mode="stack"
      layout={{ mode: "stack", sections: [{ id: "b", visible: true }, { id: "a", visible: false }] }}
      registry={{
        a: { title: "A", render: () => <div>A</div> },
        b: { title: "B", render: () => <div>B</div> },
      }}
    />
  );

  expect(screen.getByText("B")).toBeInTheDocument();
  expect(screen.queryByText("A")).not.toBeInTheDocument();
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npm test -- src/__tests__/pageLayoutRenderer.test.tsx`
Expected: FAIL because shared layout components do not exist.

- [ ] **Step 3: Implement the shared section registry and editor shell**

```ts
export type PageSectionRegistry = Record<
  string,
  { title: string; render: () => React.ReactNode; className?: string }
>;
```

```tsx
export function PageSectionHost({ layout, registry }: Props) {
  const ordered = layout.sections
    .filter((section) => section.visible && registry[section.id])
    .map((section) => registry[section.id]);
  return <div className={containerClass(layout.mode)}>{ordered.map((entry) => entry.render())}</div>;
}
```

- [ ] **Step 4: Migrate each page to a registry-driven layout**

```ts
export const dashboardLayout = {
  defaultMode: "stack",
  defaultSections: [
    { id: "service", visible: true },
    { id: "stats", visible: true },
    { id: "requests", visible: true },
  ],
};
```

```tsx
return (
  <PageLayoutEditor view="dashboard" title="仪表盘" registry={registry} defaultLayout={dashboardLayout}>
    <PageSectionHost layout={layout} registry={registry} />
  </PageLayoutEditor>
);
```

- [ ] **Step 5: Run page-layout tests and commit**

Run: `npm test -- src/__tests__/pageLayoutRenderer.test.tsx src/__tests__/pageLayoutStore.test.ts`
Expected: PASS

```bash
git add src/components/business/page-layout src/pages/Dashboard src/pages/Endpoints src/pages/Statistics src/pages/Logs src/pages/Settings src/pages/ConfigProfiles src/pages/Sync
git commit -m "feat: apply shared layout engine to pages"
```

### Task 3: Backend Rules Configuration Model and Runtime Wiring

**Files:**
- Create: `src-tauri/src/models/rules.rs`
- Modify: `src-tauri/src/models/mod.rs`
- Modify: `src-tauri/src/models/config.rs`
- Modify: `src-tauri/src/modules/storage/config_repo.rs`
- Create: `src-tauri/src/commands/rules.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/modules/proxy/circuit_breaker.rs`
- Modify: `src-tauri/src/modules/proxy/server.rs`
- Modify: `src-tauri/src/modules/transform/reasoning_effort.rs`
- Test: `src-tauri/src/modules/storage/config_repo.rs`
- Test: `src-tauri/src/modules/proxy/circuit_breaker.rs`

- [ ] **Step 1: Add failing Rust tests for rules round-trip and runtime config**

```rust
#[test]
fn rules_config_roundtrips_from_app_config_store() {
    let c = db();
    set_value(&c, "rules_circuitBreaker", r#"{"failureThreshold":5}"#).unwrap();
    let cfg = get_config(&c).unwrap();
    assert_eq!(cfg.rules.circuit_breaker.failure_threshold, 5);
}

#[test]
fn breaker_registry_uses_injected_config() {
    let reg = BreakerRegistry::new(CircuitBreakerConfig {
        failure_threshold: 5,
        ..CircuitBreakerConfig::default()
    });
    assert_eq!(reg.config().failure_threshold, 5);
}
```

- [ ] **Step 2: Run Rust tests to verify they fail**

Run: `cargo test --manifest-path src-tauri/Cargo.toml config_repo breaker_registry_uses_injected_config`
Expected: FAIL because rules config does not exist in `AppConfig`.

- [ ] **Step 3: Introduce structured rules config models**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RulesConfig {
    pub routing: RoutingRules,
    pub circuit_breaker: CircuitBreakerRules,
    pub degradation: DegradationRules,
}
```

```rust
pub struct CircuitBreakerRules {
    pub failure_threshold: u32,
    pub success_threshold: u32,
    pub timeout_seconds: u64,
    pub error_rate_threshold: f64,
    pub min_requests: u32,
}
```

- [ ] **Step 4: Wire config storage and commands**

```rust
#[tauri::command]
pub fn get_rules_config(state: State<AppState>) -> AppResult<RulesConfig> { ... }

#[tauri::command]
pub async fn set_rules_config(...) -> AppResult<RulesConfig> { ... }

#[tauri::command]
pub fn reset_rules_config(...) -> AppResult<RulesConfig> { ... }
```

```rust
breakers: BreakerRegistry::new(CircuitBreakerConfig::from_rules(&cfg.rules.circuit_breaker)),
rectifier_config: RectifierConfig::from_rules(&cfg.rules.degradation),
```

- [ ] **Step 5: Re-run Rust tests and commit**

Run: `cargo test --manifest-path src-tauri/Cargo.toml config_repo circuit_breaker`
Expected: PASS

```bash
git add src-tauri/src/models/rules.rs src-tauri/src/models/mod.rs src-tauri/src/models/config.rs src-tauri/src/modules/storage/config_repo.rs src-tauri/src/commands/rules.rs src-tauri/src/commands/mod.rs src-tauri/src/lib.rs src-tauri/src/modules/proxy/circuit_breaker.rs src-tauri/src/modules/proxy/server.rs src-tauri/src/modules/transform/reasoning_effort.rs
git commit -m "feat: add backend rules configuration model"
```

### Task 4: Rules Configuration Frontend Page

**Files:**
- Create: `src/services/modules/rules.ts`
- Create: `src/pages/Rules/index.tsx`
- Create: `src/pages/Rules/_components/RulesForm.tsx`
- Modify: `src/layouts/navConfig.tsx`
- Modify: `src/layouts/AppLayout.tsx`
- Modify: `src/stores/modules/layout.ts`
- Test: `src/__tests__/rulesConfig.test.ts`

- [ ] **Step 1: Write the failing frontend rules page test**

```tsx
import { render, screen } from "@testing-library/react";
import { Rules } from "@/pages/Rules";

it("renders routing, circuit breaker, and degradation sections", async () => {
  render(<Rules />);
  expect(await screen.findByText("路由规则")).toBeInTheDocument();
  expect(screen.getByText("熔断规则")).toBeInTheDocument();
  expect(screen.getByText("降级规则")).toBeInTheDocument();
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npm test -- src/__tests__/rulesConfig.test.ts`
Expected: FAIL because `Rules` page and rules service do not exist.

- [ ] **Step 3: Add the service module and page registration**

```ts
export const rulesApi = {
  getConfig: () => request<RulesConfig>("get_rules_config"),
  setConfig: (config: RulesConfig) => request<RulesConfig>("set_rules_config", { config }),
  resetConfig: () => request<RulesConfig>("reset_rules_config"),
};
```

```ts
export type ViewId =
  | "dashboard"
  | "endpoints"
  | "configProfiles"
  | "statistics"
  | "sync"
  | "logs"
  | "rules"
  | "settings";
```

- [ ] **Step 4: Build the structured rules form**

```tsx
<section>
  <h2>熔断规则</h2>
  <Input value={draft.circuitBreaker.failureThreshold} />
  <Input value={draft.circuitBreaker.timeoutSeconds} />
</section>
```

- [ ] **Step 5: Run tests and commit**

Run: `npm test -- src/__tests__/rulesConfig.test.ts`
Expected: PASS

```bash
git add src/services/modules/rules.ts src/pages/Rules src/layouts/navConfig.tsx src/layouts/AppLayout.tsx src/stores/modules/layout.ts src/__tests__/rulesConfig.test.ts
git commit -m "feat: add rules configuration page"
```

### Task 5: Request Trace Data Model, Migration, and Repository

**Files:**
- Modify: `src-tauri/src/models/stats.rs`
- Modify: `src-tauri/src/modules/stats/aggregator.rs`
- Modify: `src-tauri/src/modules/storage/migration.rs`
- Modify: `src-tauri/src/modules/storage/request_logs_repo.rs`
- Test: `src-tauri/src/modules/storage/migration.rs`
- Test: `src-tauri/src/modules/storage/request_logs_repo.rs`

- [ ] **Step 1: Add failing migration and round-trip tests for four-stage traces**

```rust
#[test]
fn v11_adds_request_trace_columns() {
    let c = Connection::open_in_memory().unwrap();
    run_migrations(&c).unwrap();
    let cols = request_log_columns(&c);
    assert!(cols.contains(&"received_request".to_string()));
    assert!(cols.contains(&"forwarded_request".to_string()));
    assert!(cols.contains(&"upstream_response".to_string()));
    assert!(cols.contains(&"response_payload".to_string()));
}

#[test]
fn request_trace_roundtrips() {
    let mut log = log(100, "a", false);
    log.received_request = Some(sample_trace("接收请求"));
    insert_batch(&mut c, &[log], "dev").unwrap();
    let (items, _) = query_page(&c, None, None, None, 50, 0).unwrap();
    assert_eq!(items[0].received_request.as_ref().unwrap().label, "接收请求");
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --manifest-path src-tauri/Cargo.toml request_trace_roundtrips v11_adds_request_trace_columns`
Expected: FAIL because trace columns and structs do not exist.

- [ ] **Step 3: Add structured request-trace types to the stats model**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestStageTrace {
    pub label: String,
    pub method: String,
    pub url: String,
    pub headers: Vec<TraceHeader>,
    pub body: Option<String>,
    pub status_code: Option<i64>,
}
```

- [ ] **Step 4: Extend migration and repository persistence**

```rust
"ALTER TABLE request_logs ADD COLUMN received_request TEXT;
 ALTER TABLE request_logs ADD COLUMN forwarded_request TEXT;
 ALTER TABLE request_logs ADD COLUMN upstream_response TEXT;
 ALTER TABLE request_logs ADD COLUMN response_payload TEXT;"
```

```rust
stmt.execute(params![
    ...,
    serde_json::to_string(&l.received_request).ok(),
    serde_json::to_string(&l.forwarded_request).ok(),
    serde_json::to_string(&l.upstream_response).ok(),
    serde_json::to_string(&l.response_payload).ok(),
])?;
```

- [ ] **Step 5: Re-run Rust tests and commit**

Run: `cargo test --manifest-path src-tauri/Cargo.toml migration request_logs_repo`
Expected: PASS

```bash
git add src-tauri/src/models/stats.rs src-tauri/src/modules/stats/aggregator.rs src-tauri/src/modules/storage/migration.rs src-tauri/src/modules/storage/request_logs_repo.rs
git commit -m "feat: persist four-stage request traces"
```

### Task 6: Proxy Trace Capture and Stats Emission

**Files:**
- Create: `src-tauri/src/modules/proxy/trace_capture.rs`
- Modify: `src-tauri/src/modules/proxy/forward.rs`
- Modify: `src-tauri/src/modules/stats/aggregator.rs`
- Test: `src-tauri/src/modules/proxy/forward.rs`

- [ ] **Step 1: Add failing unit tests for trace body truncation and header redaction**

```rust
#[test]
fn redact_sensitive_headers() {
    let trace = TraceCapture::from_headers("接收请求", &headers_with_auth());
    assert_eq!(trace.headers[0].value, "***");
}

#[test]
fn truncate_large_bodies_preserves_utf8() {
    let body = capture_body("测".repeat(5000).as_bytes());
    assert!(body.unwrap().contains("已截断"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml redact_sensitive_headers truncate_large_bodies_preserves_utf8`
Expected: FAIL because trace-capture helpers do not exist.

- [ ] **Step 3: Implement a shared capture helper**

```rust
pub fn sanitize_header_value(name: &str, value: &str) -> String {
    match name.to_ascii_lowercase().as_str() {
        "authorization" | "x-api-key" | "cookie" => "***".into(),
        _ => value.to_string(),
    }
}
```

```rust
pub fn capture_stage(
    label: &str,
    method: &Method,
    url: &str,
    headers: &HeaderMap,
    body: &[u8],
) -> RequestStageTrace { ... }
```

- [ ] **Step 4: Attach traces during proxy forwarding**

```rust
let received_request = trace_capture::capture_stage("接收请求", &method, &path, &headers, &body);
let forwarded_request = trace_capture::capture_stage("转发请求", &method, &url, &headers, &attempt_body);
let upstream_response = trace_capture::capture_response("接收转发的请求", status, &resp_headers, &err_bytes);
let response_payload = trace_capture::capture_response("响应请求", status, &out_headers, &final_bytes);
```

- [ ] **Step 5: Re-run Rust tests and commit**

Run: `cargo test --manifest-path src-tauri/Cargo.toml forward`
Expected: PASS

```bash
git add src-tauri/src/modules/proxy/trace_capture.rs src-tauri/src/modules/proxy/forward.rs src-tauri/src/modules/stats/aggregator.rs
git commit -m "feat: capture request traces during proxy flow"
```

### Task 7: Logs Page Request Trace UI

**Files:**
- Modify: `src/services/modules/stats.ts`
- Modify: `src/pages/Logs/index.tsx`
- Create: `src/pages/Logs/_components/RequestTracePanel.tsx`
- Modify: `src/components/business/RequestMonitor.tsx`
- Test: `src/__tests__/RequestMonitor.test.tsx`
- Test: `src/__tests__/RequestTracePanel.test.tsx`

- [ ] **Step 1: Add failing UI tests for four-stage rendering**

```tsx
it("renders the four request-trace stages for a selected log", () => {
  render(<RequestTracePanel log={logWithTrace} />);
  expect(screen.getByText("接收请求")).toBeInTheDocument();
  expect(screen.getByText("转发请求")).toBeInTheDocument();
  expect(screen.getByText("接收转发的请求")).toBeInTheDocument();
  expect(screen.getByText("响应请求")).toBeInTheDocument();
});
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `npm test -- src/__tests__/RequestMonitor.test.tsx src/__tests__/RequestTracePanel.test.tsx`
Expected: FAIL because trace fields and panel component do not exist.

- [ ] **Step 3: Extend the frontend stats types**

```ts
export interface RequestStageTrace {
  label: string;
  method: string;
  url: string;
  headers: { key: string; value: string }[];
  body: string | null;
  statusCode: number | null;
}
```

- [ ] **Step 4: Add a split logs layout with trace detail panel**

```tsx
const [selectedRequest, setSelectedRequest] = useState<RequestLog | null>(null);

<PageSectionHost
  layout={layout}
  registry={{
    "log-stream": { title: "日志流", render: renderLogStream },
    "request-trace": { title: "请求详情", render: () => <RequestTracePanel log={selectedRequest} /> },
  }}
/>
```

- [ ] **Step 5: Re-run frontend tests and commit**

Run: `npm test -- src/__tests__/RequestMonitor.test.tsx src/__tests__/RequestTracePanel.test.tsx`
Expected: PASS

```bash
git add src/services/modules/stats.ts src/pages/Logs/index.tsx src/pages/Logs/_components/RequestTracePanel.tsx src/components/business/RequestMonitor.tsx src/__tests__/RequestMonitor.test.tsx src/__tests__/RequestTracePanel.test.tsx
git commit -m "feat: show four-stage request traces in logs"
```

### Task 8: Transport Abstraction and Browser Admin API

**Files:**
- Create: `src/services/transport/types.ts`
- Create: `src/services/transport/desktop.ts`
- Create: `src/services/transport/web.ts`
- Modify: `src/services/request.ts`
- Create: `src/services/runtime.ts`
- Create: `src-tauri/src/commands/web_admin.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/modules/proxy/server.rs`
- Test: `src/__tests__/transport.test.ts`

- [ ] **Step 1: Add failing transport-adapter tests**

```tsx
import { createTransport } from "@/services/runtime";

it("creates a web transport when window.__CCMESH_WEB__ is true", () => {
  (window as any).__CCMESH_WEB__ = true;
  expect(createTransport().kind).toBe("web");
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npm test -- src/__tests__/transport.test.ts`
Expected: FAIL because runtime transport factories do not exist.

- [ ] **Step 3: Extract transport contracts from request.ts**

```ts
export interface AppTransport {
  kind: "desktop" | "web";
  request<T>(command: string, args?: Record<string, unknown>): Promise<T>;
  subscribe<T>(event: string, cb: (payload: T) => void): Promise<() => void>;
}
```

- [ ] **Step 4: Expose web-safe admin API and events from the Rust service**

```rust
Router::new()
  .route("/__admin/api/config", get(get_config_http).post(set_config_http))
  .route("/__admin/api/rules", get(get_rules_http).post(set_rules_http))
  .route("/__admin/api/logs/recent", get(get_logs_http))
  .route("/__admin/events", get(events_sse_route))
```

- [ ] **Step 5: Re-run tests and commit**

Run: `npm test -- src/__tests__/transport.test.ts`
Expected: PASS

```bash
git add src/services/transport src/services/request.ts src/services/runtime.ts src-tauri/src/commands/web_admin.rs src-tauri/src/commands/mod.rs src-tauri/src/lib.rs src-tauri/src/modules/proxy/server.rs src/__tests__/transport.test.ts
git commit -m "feat: add shared transport layer and admin api"
```

### Task 9: Web Console Entry, Static Host, and Final Verification

**Files:**
- Create: `src/main-web.tsx`
- Modify: `vite.config.ts`
- Create: `src-tauri/src/modules/web_admin/static_assets.rs`
- Modify: `src/layouts/TitleBar.tsx`
- Modify: `src/layouts/WindowControls.tsx`
- Modify: `src/components/common/CloseDialog.tsx`
- Test: `src/__tests__/webConsoleShell.test.tsx`

- [ ] **Step 1: Add failing shell test for browser rendering**

```tsx
it("hides desktop-only window controls in web mode", () => {
  (window as any).__CCMESH_WEB__ = true;
  render(<App />);
  expect(screen.queryByLabelText("最小化")).not.toBeInTheDocument();
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npm test -- src/__tests__/webConsoleShell.test.tsx`
Expected: FAIL because the app does not have a dedicated web entry or host-mode guards.

- [ ] **Step 3: Create the browser entry and static asset host**

```ts
import { AppLayout } from "@/layouts/AppLayout";

(window as any).__CCMESH_WEB__ = true;
createRoot(document.getElementById("root")!).render(<AppLayout />);
```

```rust
pub fn static_admin_asset(path: &str) -> Option<(&'static str, &'static [u8])> { ... }
```

- [ ] **Step 4: Add desktop-only guards and final verification commands**

```tsx
if (isWebRuntime()) return null;
```

Run: `npm run check:front`
Expected: PASS

Run: `cargo test --manifest-path src-tauri/Cargo.toml`
Expected: PASS

Run: `npm run build`
Expected: PASS

- [ ] **Step 5: Commit and capture manual verification notes**

```bash
git add src/main-web.tsx vite.config.ts src-tauri/src/modules/web_admin/static_assets.rs src/layouts/TitleBar.tsx src/layouts/WindowControls.tsx src/components/common/CloseDialog.tsx src/__tests__/webConsoleShell.test.tsx
git commit -m "feat: add browser-hosted web console"
```

---

## Plan Self-Review

### Spec Coverage

- Story 1 is covered by Tasks 1-2.
- Story 2 is covered by Tasks 3-4.
- Story 3 is covered by Tasks 5-7.
- Story 4 is covered by Tasks 8-9.

No PRD requirement is left without a corresponding task.

### Placeholder Scan

- Removed all `TBD`/`TODO` placeholders.
- Each task includes explicit files, code skeletons, verification commands, and commit messages.

### Type Consistency

- `RulesConfig`, `PageLayoutConfig`, and `RequestStageTrace` names are used consistently across backend and frontend tasks.
- `rulesApi` mirrors `get_rules_config` / `set_rules_config` / `reset_rules_config`.
- `AppTransport` is the single transport contract for both desktop and web hosts.

