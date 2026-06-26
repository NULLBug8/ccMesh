# Balance, Rules, and Mapping Strategy Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use test-driven-development to implement behavior changes task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add relay balance querying, richer rule configuration help, and configurable model-mapping routing strategy.

**Architecture:** Endpoint balance query configuration is persisted with each endpoint and exposed through both endpoint cards and a centralized balance page. Rule configuration remains in `RulesConfig`, with new fields propagated to the runtime resolver. Model mapping strategy is selected from rules and applied before retry/rotation.

**Tech Stack:** Tauri Rust backend, SQLite migrations, reqwest, React 19, TanStack Query, Vitest, cargo tests.

---

### Task 2038.10: Balance Query Data Model and Commands

**Files:**
- Modify: `src-tauri/src/models/endpoint.rs`
- Modify: `src-tauri/src/modules/storage/migration.rs`
- Modify: `src-tauri/src/modules/storage/endpoint_repo.rs`
- Modify: `src-tauri/src/commands/endpoint.rs`
- Modify: `src-tauri/src/commands/web_admin.rs`
- Modify: `src/services/modules/endpoint.ts`

- [ ] Write failing Rust tests for endpoint balance config persistence.
- [ ] Add `BalanceQueryConfig` and migration column.
- [ ] Add `query_endpoint_balance` command using method/url/headers/body plus JSON path extraction.
- [ ] Add common presets and custom template support.

### Task 2038.11: Balance UI

**Files:**
- Modify: `src/layouts/navConfig.tsx`
- Modify: `src/layouts/AppLayout.tsx`
- Modify: `src/pages/Endpoints/_components/EndpointCard.tsx`
- Modify: `src/pages/Endpoints/_components/EndpointForm.tsx`
- Create: `src/pages/Balances/index.tsx`
- Create: `src/pages/Balances/layout.ts`
- Create: `src/__tests__/balanceConfig.test.tsx`

- [ ] Write failing UI tests for balance query controls and centralized page.
- [ ] Show one-click balance query on endpoint cards.
- [ ] Add balance template editor in endpoint form.
- [ ] Add centralized balance page with batch refresh and custom template visibility.

### Task 2038.12: Richer Rules and Help Examples

**Files:**
- Modify: `src-tauri/src/models/rules.rs`
- Modify: `src-tauri/src/modules/proxy/circuit_breaker.rs`
- Modify: `src-tauri/src/modules/transform/thinking_rectifier.rs`
- Modify: `src/services/modules/rules.ts`
- Modify: `src/pages/Rules/_components/RulesForm.tsx`
- Modify: `src/__tests__/rulesConfig.test.tsx`

- [ ] Write failing tests for new rule fields and example help text.
- [ ] Add fields for retry budget, request timeout, failure status codes, mapping strategy, and degradation controls.
- [ ] Render each config item with a concrete example under the control.

### Task 2038.13: Model Mapping Routing Strategy

**Files:**
- Modify: `src-tauri/src/modules/proxy/resolver.rs`
- Modify: `src-tauri/src/modules/proxy/forward.rs`
- Modify: `src-tauri/src/models/rules.rs`
- Modify: `src/pages/Rules/_components/RulesForm.tsx`

- [ ] Write failing resolver tests for `site-first` and `global-native-first`.
- [ ] Default to `site-first`.
- [ ] Apply ordered candidates before breaker filtering and rotation.
- [ ] Explain the two strategies in the Rules page.

### Task 2038.14: Verification and Release Check

**Files:**
- Modify: `docs/task-plan/progress.csv`
- Modify: `docs/task-plan/tasks/2038-layout-routing-logs-web/task.md`

- [ ] Run targeted Vitest and cargo tests.
- [ ] Run `npm run build`.
- [ ] Run `cargo check --manifest-path src-tauri/Cargo.toml`.
- [ ] Rebuild frontend dist and verify `http://127.0.0.1:3001/` in browser.

### Task 2038.15: Smart Balance Template Detection

**Files:**
- Modify: `src-tauri/src/commands/endpoint.rs`
- Modify: `src-tauri/src/commands/web_admin.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src/services/modules/endpoint.ts`
- Modify: `src/pages/Endpoints/_components/EndpointForm.tsx`
- Modify: `src/__tests__/balanceConfig.test.tsx`

- [x] Write failing frontend and Rust tests for probe classification and no-sample AI blocking.
- [x] Add built-in template probing with matched/sampleAvailable/allFailed classification.
- [x] Show all failed template reasons and custom path re-probe entry when every URL fails.
- [x] Add AI-assisted template generation only after a sanitized response sample is available.
- [x] Verify on `http://127.0.0.1:3001/` without touching the old `3000` instance.
