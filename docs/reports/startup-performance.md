# ccMesh 启动白屏与性能优化方案

> 分析对象：tauri-gateway（产品名 ccMesh）
> 技术栈：Tauri 2 + React 19 + Vite 7 + Tailwind 4 + Rust（axum 网关 / SQLite）
> 文档日期：2026-06-09
> 数据来源：当前仓库源码与 `dist/` 构建产物实测

---

## 1. 背景与现象

应用为桌面端 API 网关（ccMesh），启动后存在**打开白屏 / 首屏内容出现慢**的体验问题。本文基于源码与构建产物实测，定位白屏根因、初始化与性能瓶颈，并给出分级落地方案。

本次分析全部结论均基于可验证的实测数据，关键证据均标注 `文件:行号`。

### 实测基线数据

| 指标 | 实测值 | 来源 |
| --- | --- | --- |
| 主 JS chunk | **1,050,187 字节（≈1.05 MB，未压缩）** | `dist/assets/index-p3K1cfwD.js` |
| JS chunk 数量 | **1 个（无任何代码分割）** | `dist/assets/*.js` |
| 主 CSS | 72,421 字节（≈70 KB） | `dist/assets/index-Dk-e6voW.css` |
| 字体文件 | **12 个 woff2，合计 312 KB** | `dist/assets/*.woff2` |
| 首屏 HTML | `#root` 为空，无骨架屏 | `index.html` / `dist/index.html` |

---

## 2. 启动链路时序剖析

理解白屏，必须看清从「双击图标」到「首屏可交互」之间发生了什么。链路分为后端（Rust）与前端（WebView）两条，并行又有先后。

### 2.1 后端 Rust 启动（`src-tauri/src/lib.rs`）

`run()` → `tauri::Builder::default()` → `.setup(|app| { ... })`，setup 闭包**全程同步执行**，顺序为：

1. `tracing_subscriber` 日志层初始化（`lib.rs:13-19`）
2. `utils::paths::db_path` → `modules::storage::db::create_pool`（`lib.rs:29-30`）
3. 取连接 → `run_migrations` 幂等迁移（`lib.rs:31-34`，迁移脚本见 `migration.rs`，当前 5 个版本）
4. 读取 `logLevel` 配置并设置日志级别（`lib.rs:37-42`）
5. `set_app_handle`（`lib.rs:43`）
6. `get_or_create_device_id`（`lib.rs:46-49`）
7. `StatsAggregator::new` 统计聚合器（`lib.rs:53-57`）
8. `app.manage(app_state)` 注入全局状态（`lib.rs:60-62`）
9. `build_tray` 构建系统托盘（`lib.rs:65-67`）
10. 注册窗口关闭事件（`lib.rs:70-102`）

### 2.2 前端 WebView 启动

1. WebView 加载 `index.html` —— `<div id="root"></div>` **为空**，head 中同步引入 `index-*.js` 与 `index-*.css`（`dist/index.html:8-13`）
2. 下载 → 解析 → 执行 **1.05 MB 的单一 JS chunk**
3. 同步 `import "@fontsource-variable/inter"` 与 `jetbrains-mono`（`main.tsx:9-10`）
4. `ReactDOM.createRoot().render()` 挂载 React 树（`main.tsx:21`）
5. Provider 链：`StrictMode → ThemeProvider(system) → QueryClientProvider → TooltipProvider`，并无条件渲染 `ReactQueryDevtools`（`main.tsx:21-37`）
6. `App` 挂载，运行 4 个 hook：`useThemeSync / useAutoTheme / useTrayActions / useUpdate`（`App.tsx:9-12`）
7. `AppLayout` 渲染，`VIEW_MAP` **同步实例化全部 6 个页面**（`AppLayout.tsx:15-22`）

### 2.3 白屏发生的时间窗

```
双击图标
  │
  ├─[Rust] setup() 同步执行（建池 / 迁移 / 托盘 …）
  │
  └─[窗口] 立即可见（未设 visible:false）────────┐
                                                  │  ← 此区间窗口为纯白
        [WebView] 下载+解析+执行 1.05MB JS ───────┤    （无骨架屏 + 大 bundle）
                                                  │
        React 挂载 → 首屏 paint ──────────────────┘
                  │
                  └─ useThemeSync 拿到后端主题 → 可能再次明暗切换（FOUC）
```

**结论：白屏 = 窗口提前可见 + `#root` 空占位 + 1.05MB 单 bundle 解析执行耗时，三者叠加。**

---

## 3. 问题诊断（分级）

按对白屏/首屏体验的影响程度分为 P0（直接造成白屏）、P1（明显拖慢/闪烁）、P2（次要优化）。

### P0-1　窗口提前可见且无内容占位

- **现象**：窗口出现后是一片纯白（自定义无边框标题栏 `decorations:false`），数百毫秒后才出现 UI。
- **根因**：
  - `tauri.conf.json` 的 window 配置**未设置 `"visible": false`**（`tauri.conf.json:13-23`），窗口创建即显示。
  - `index.html` 的 `#root` 为空，**无骨架屏 / loading**（`index.html:11`、`dist/index.html:13`）。
- **影响**：从窗口可见到 React 首次 paint 之间全程白屏，是体验上最直接的「白屏」。

### P0-2　单一 1.05MB JS chunk，零代码分割

- **现象**：首屏需下载并执行约 1.05MB JS 才能渲染任何内容。
- **根因**：
  - `AppLayout.tsx:5-10` **静态 import 全部 6 个页面**（Dashboard / Endpoints / Statistics / Sync / Logs / Settings），`VIEW_MAP`（`AppLayout.tsx:15-22`）同步实例化所有页面 JSX。
  - 全仓 `grep lazy|Suspense|import(` **无匹配**——完全没有路由级懒加载。
  - `vite.config.ts` 无 `build.rollupOptions.manualChunks`，vendor 与业务代码、所有页面、重型库全部打进同一个 chunk。
- **影响**：首屏强制加载了大量非首屏依赖（见下），解析执行时间长，低端机尤其明显。

### P0-3　重型库被打进首屏却非首屏所需

- **CodeMirror**：`@codemirror/*` 全家桶 + `@uiw/react-codemirror`，**仅 `EndpointForm.tsx` 使用**（端点表单 JSON 编辑，`grep` 仅命中该文件），却随首屏 bundle 全量加载。CodeMirror 体积可观，是分割首选。
- **ReactQueryDevtools**：`main.tsx:5` 无条件 import、`main.tsx:34` 无条件渲染，**会被打进生产包**（已渲染的组件不会被 tree-shake）。生产环境本不需要。
- **dnd-kit**（`@dnd-kit/react` + helpers）：仅端点列表拖拽排序使用，同样进首屏。

### P1-1　字体全量打包（312KB / 12 个 woff2）

- **现象**：`dist/assets` 含 12 个 woff2 共 312KB，包含 cyrillic / greek / vietnamese 等子集。
- **根因**：`main.tsx:9-10` 直接 `import "@fontsource-variable/inter"` 与 `jetbrains-mono`，引入了可变字体的**全部语言子集**。本应用 UI 仅中/英文。
- **影响**：增大首屏资源体积与 CSS（@font-face 声明），字体加载也可能引起文字闪烁（FOUT）。

### P1-2　主题闪烁（FOUC）

- **现象**：启动瞬间可能先显示系统默认主题，随后切换为用户设置的主题，明暗闪一下。
- **根因**：`ThemeProvider defaultTheme="system"`（`main.tsx:24`）先以系统主题渲染；`useThemeSync`（`useThemeSync.ts:11-22`）在 `useEffect` 中异步 `configApi.getConfig()` 拿到后端主题后才 `setTheme`，时序上晚于首屏 paint。
- **影响**：首屏明暗跳变，观感廉价。

### P1-3　首屏重复请求后端配置

- **现象**：启动时 `getConfig` 被调用两次。
- **根因**：`useThemeSync`（`useThemeSync.ts:13`）与 `useAutoTheme`（`useAutoTheme.ts:15`）各自独立请求 `configApi.getConfig`，未共享缓存键。`useAutoTheme` 用了 `useQuery(["config"])`，而 `useThemeSync` 直接调用 API 绕过了 Query 缓存。
- **影响**：多一次 IPC 往返；逻辑分散。

### P2-1　后端 setup 关键路径偏重

- **现象**：窗口内容就绪前，setup 串行完成建池、迁移、设备 ID、统计聚合器、托盘构建。
- **根因**：`lib.rs:25-104` setup 闭包全同步。其中：
  - `create_pool` 使用 `Pool::builder().build()`（`db.rs:21-23`）。r2d2 的 `build()` 会**预建立连接**（默认 `max_size=10`，`min_idle` 缺省等于 `max_size`），即同步打开多条 SQLite 连接并各跑一次 WAL PRAGMA。
  - 托盘构建、统计聚合器等**并非首屏渲染的必要前置**。
- **影响**：本地 SQLite 通常很快（首启迁移稍慢），但这些工作都堆在「窗口内容出现之前」，可压缩。
- **备注**：r2d2 `build()` 的预建连接行为建议在改动时以实际日志/计时验证（见第 5 节验证方法）。

### P2-2　React StrictMode 开发期双渲染

- **现象**：开发环境 effect/hook 执行两次（`main.tsx:22` `StrictMode`）。
- **影响**：仅开发期体感，生产无影响；首屏 IPC 在开发期会翻倍，调试时易误判。保留 StrictMode 是合理的，仅作认知提示。

### P2-3　无用依赖 motion

- **现象**：`package.json` 声明 `motion ^12.40.0`，但**源码从未 import**（全仓仅 CSS `prefers-reduced-motion` 命中，无 `motion/react` 引用）。
- **影响**：因未被 import，Vite 不会打进 bundle，**不增首屏体积**；但属冗余依赖，建议清理以降低安装体积与维护噪音。

---

## 4. 优化方案（分级 + 可落地代码）

方案按「收益 / 成本比」与优先级排序。**P0-A、P0-B、P0-C 三项落地即可基本消除白屏感知**。

### P0-A　窗口延迟显示 + 启动骨架屏（消除白屏的关键）

**目标**：窗口不再「先白后亮」，要么显示骨架屏，要么等首屏 ready 再显示窗口。

**做法 1：HTML 内联骨架屏（零依赖，强烈推荐）**

在 `index.html` 的 `#root` 内写入纯静态骨架，React 挂载后会自动替换它。骨架是内联的，随 HTML 秒出，彻底消除「纯白」：

```html
<!-- index.html -->
<body>
  <div id="root">
    <!-- 启动骨架屏：React 挂载后被 render 覆盖 -->
    <div id="boot-skeleton" style="
      position:fixed;inset:0;display:flex;align-items:center;justify-content:center;
      background:#09090b;color:#52525b;font-family:system-ui,sans-serif;">
      <div style="display:flex;flex-direction:column;align-items:center;gap:16px;">
        <div style="width:40px;height:40px;border-radius:50%;
          border:3px solid #27272a;border-top-color:#22c55e;
          animation:boot-spin .8s linear infinite;"></div>
        <span style="font-size:13px;">正在启动 ccMesh…</span>
      </div>
    </div>
  </div>
  <style>@keyframes boot-spin{to{transform:rotate(360deg)}}</style>
  <script type="module" src="/src/main.tsx"></script>
</body>
```

> 背景色应与应用默认主题底色一致（当前暗色底 `#09090b`，见 `index.css` 的 `.dark` 令牌），避免骨架与首屏之间的色块跳变。

**做法 2：配合窗口延迟显示（消除主题闪烁，进阶）**

1. `tauri.conf.json` 窗口设为初始隐藏：

```jsonc
// tauri.conf.json → app.windows[0]
{
  "title": "ccMesh",
  "width": 1200, "height": 800,
  "visible": false,        // 新增：先不显示
  "decorations": false,
  // …其余不变
}
```

2. 前端首屏 ready 后再显示窗口（在 React 首次有效渲染后调用）：

```ts
// 例如在 AppLayout 首帧 useEffect 中，或主题恢复完成后
import { getCurrentWindow } from "@tauri-apps/api/window";

useEffect(() => {
  // 首屏与主题就绪后再亮窗，避免用户看到白屏与明暗跳变
  getCurrentWindow().show();
}, []);
```

> 二选一或叠加：仅做法 1 即可去白屏；叠加做法 2 可同时消除 P1-2 主题闪烁，但需确保 `show()` 一定会被调用（建议加超时兜底，防止异常时窗口永不显示）。

---

### P0-B　路由级代码分割（React.lazy + Suspense）

**目标**：首屏只加载当前激活页面，其余页面按需加载，主 chunk 大幅瘦身。

改造 `src/layouts/AppLayout.tsx`：

```tsx
import { lazy, Suspense, useEffect, type ReactNode } from "react";
import { cn } from "@/lib/utils";
import { useLayoutStore, type ViewId } from "@/stores";
import { TopNav } from "./TopNav";
import { SideNav } from "./SideNav";
import { TitleBar } from "./TitleBar";

// 6 个页面改为懒加载（各自拆成独立 chunk）
const Dashboard  = lazy(() => import("@/pages/Dashboard").then(m => ({ default: m.Dashboard })));
const Endpoints  = lazy(() => import("@/pages/Endpoints").then(m => ({ default: m.Endpoints })));
const Statistics = lazy(() => import("@/pages/Statistics").then(m => ({ default: m.Statistics })));
const Sync       = lazy(() => import("@/pages/Sync").then(m => ({ default: m.Sync })));
const Logs       = lazy(() => import("@/pages/Logs").then(m => ({ default: m.Logs })));
const Settings   = lazy(() => import("@/pages/Settings").then(m => ({ default: m.Settings })));

const VIEW_MAP: Record<ViewId, ReactNode> = {
  dashboard: <Dashboard />, endpoints: <Endpoints />, statistics: <Statistics />,
  sync: <Sync />, logs: <Logs />, settings: <Settings />,
};

export function AppLayout() {
  const navMode = useLayoutStore((s) => s.navMode);
  const activeView = useLayoutStore((s) => s.activeView);
  // …媒体查询 useEffect 不变…

  return (
    <div className="flex h-screen w-screen flex-col overflow-hidden bg-background text-foreground">
      <TitleBar />
      <div className={cn("flex flex-1 overflow-hidden", navMode === "vertical" ? "flex-row" : "flex-col")}>
        {navMode === "horizontal" ? <TopNav /> : <SideNav />}
        <main className="flex-1 overflow-y-auto p-8">
          <Suspense fallback={<PagePlaceholder />}>{VIEW_MAP[activeView]}</Suspense>
        </main>
      </div>
    </div>
  );
}
```

> 注意：当前 `VIEW_MAP` 一次性实例化所有 `<Page />`，即使懒加载也会触发全部 chunk 请求。建议改为**按 `activeView` 惰性渲染**——只渲染当前视图，切换时再加载目标页 chunk：
>
> ```tsx
> const PAGES: Record<ViewId, React.ComponentType> = {
>   dashboard: Dashboard, endpoints: Endpoints, /* … */
> };
> // 渲染处：
> const Active = PAGES[activeView];
> <Suspense fallback={<PagePlaceholder />}><Active /></Suspense>
> ```
>
> `PagePlaceholder` 复用现有 `src/components/common/Placeholder.tsx`，保证切页不闪。

**预期**：首屏 chunk 仅含 Provider 链 + 布局 + 默认 Dashboard 页，CodeMirror/dnd-kit 等随对应页面分离，主 chunk 预计下降数百 KB。

---

### P0-C　CodeMirror 等重型库懒加载 + Vite 手动分包

**1）CodeMirror 仅在端点表单打开时加载**

`EndpointForm.tsx` 是 CodeMirror 唯一使用方，将其编辑器部分抽成懒加载组件：

```tsx
const JsonEditor = lazy(() => import("./JsonEditor")); // 内部封装 @uiw/react-codemirror
// 使用处用 <Suspense fallback={…}> 包裹
```

配合 P0-B 后，端点页本身已是独立 chunk；再将 CodeMirror 收敛进「打开表单才加载」可进一步压缩端点页首次进入的体积。

**2）Vite 手动分包（vendor 稳定缓存）**

在 `vite.config.ts` 增加 `build.rollupOptions.manualChunks`，把稳定的第三方库与业务代码分离，提升更新后缓存命中：

```ts
// vite.config.ts
export default defineConfig(async () => ({
  plugins: [react(), tailwindcss()],
  resolve: { alias: { "@": path.resolve(__dirname, "./src") } },
  build: {
    rollupOptions: {
      output: {
        manualChunks: {
          "react-vendor": ["react", "react-dom"],
          "query-vendor": ["@tanstack/react-query"],
          "editor-vendor": [
            "@uiw/react-codemirror", "@codemirror/state", "@codemirror/view",
            "@codemirror/lang-json", "@codemirror/commands",
            "@codemirror/search", "@codemirror/theme-one-dark",
          ],
          "dnd-vendor": ["@dnd-kit/react", "@dnd-kit/helpers"],
          "ui-vendor": ["radix-ui", "lucide-react"],
        },
      },
    },
    chunkSizeWarningLimit: 700,
  },
  // …server 配置不变…
}));
```

> 分包键名以 `package.json` 实际依赖为准；`editor-vendor` 与 P0-B/上面的懒加载叠加后，仅在需要时加载。

---

### P0-D　ReactQueryDevtools 仅开发环境引入

`main.tsx` 改为按环境条件加载，避免 devtools 进生产包：

```tsx
// main.tsx
const ReactQueryDevtools = import.meta.env.DEV
  ? lazy(() => import("@tanstack/react-query-devtools").then(m => ({ default: m.ReactQueryDevtools })))
  : () => null;

// 渲染处：
{import.meta.env.DEV && (
  <Suspense fallback={null}><ReactQueryDevtools initialIsOpen={false} /></Suspense>
)}
```

> Vite 会在生产构建中将 `import.meta.env.DEV` 常量折叠为 `false`，配合动态 import 即可让 devtools 完全不进生产 chunk。

### P1-A　字体按需子集 + 预加载

可变字体只引入实际使用的拉丁子集，去除 cyrillic/greek/vietnamese：

```ts
// main.tsx —— 用 fontsource 子集化入口替代全量
// 仅拉丁子集（按 @fontsource-variable/inter 提供的子集导出名为准）
import "@fontsource-variable/inter/wght.css";          // 或具体 latin 子集入口
import "@fontsource-variable/jetbrains-mono/wght.css";
```

> 具体子集导出名需对照已安装版本的 `node_modules/@fontsource-variable/inter` 实际 `*.css`。
> 进阶：改用 `font-display: swap`（fontsource 默认已带）并对首屏关键字重做 `<link rel="preload" as="font">`，减少 FOUT。
> 目标：12 个 woff2 / 312KB 收敛到中英文所需的 2-4 个。

### P1-B　消除主题闪烁（FOUC）

两条思路，二选一：

1. **窗口延迟显示**（见 P0-A 做法 2）：主题恢复完成后再 `window.show()`，用户看不到中间态。
2. **启动前置脚本**：在 `index.html` 的 `<head>` 内联一段极短脚本，从持久化的偏好（如 zustand persist 的 `layout-prefs` 或单独的 theme 键）同步读取并提前给 `<html>` 加 `class="dark"`，先于 React 定主题：

```html
<script>
  try {
    var t = localStorage.getItem("theme"); // 若主题改为持久化到 localStorage
    if (t === "dark" || (!t && matchMedia("(prefers-color-scheme: dark)").matches))
      document.documentElement.classList.add("dark");
  } catch (e) {}
</script>
```

> 注意：当前主题来源是**后端配置**（`useThemeSync`），localStorage 没有 theme 键。若采用思路 2，需先把主题偏好同步落一份到 localStorage（后端仍作为跨设备同步源）。否则优先用思路 1（窗口延迟显示），改动更小。

### P1-C　合并首屏重复的 getConfig

让 `useThemeSync` 与 `useAutoTheme` 共用同一个 React Query 缓存键，避免两次 IPC：

```ts
// useThemeSync.ts —— 复用 useQuery(["config"]) 而非直接调用 configApi.getConfig
const { data: cfg } = useQuery({ queryKey: ["config"], queryFn: configApi.getConfig });
// 拿到 cfg.theme 后再 setTheme；写回逻辑保持不变
```

> 两个 hook 共享 `["config"]` 后，首屏只发一次请求；`staleTime` 已在 `main.tsx:13-18` 配为 60s。

### P2-A　后端 setup 关键路径瘦身

把**非首屏必需**的初始化移出关键路径，让窗口内容更快就绪：

1. **连接池惰性化**：若确认 r2d2 `build()` 预建连接拖慢首启，可用 `Pool::builder().min_idle(Some(1)).max_size(8).build()` 降低预建数量，或用 `build_unchecked()` 改为惰性建连：

```rust
// db.rs —— 降低首启同步建连成本（示例）
Pool::builder()
    .max_size(8)
    .min_idle(Some(1))         // 首启只预建 1 条
    .build(manager)
    .map_err(|e| AppError::Db(format!("创建连接池失败: {e}")))
```

2. **托盘 / 统计聚合器延后**：`build_tray`、`StatsAggregator` 等非渲染前置项，可在 setup 内 `tauri::async_runtime::spawn` 异步初始化，或挪到首个相关命令首次调用时再做。迁移 `run_migrations` 必须保留在数据访问前（不可省）。

> 该项收益取决于实测：本地 SQLite 通常很快。**先测量再优化**（见第 5 节），不要为臆想的瓶颈过度改造后端。

### P2-B　清理无用依赖 motion

源码无任何 `motion` import，可从 `package.json` 移除 `motion ^12.40.0`：

```bash
pnpm remove motion
```

> 不影响 bundle 体积（本就未打入），属依赖卫生清理。移除后跑一次 `pnpm build` 确认无引用残留。

---

## 5. 实施优先级与预期收益

| 优先级 | 方案 | 改动文件 | 成本 | 预期收益 |
| --- | --- | --- | --- | --- |
| **P0-A** | 骨架屏 + 窗口延迟显示 | `index.html`、`tauri.conf.json`、布局 hook | 低 | **直接消除白屏感知** |
| **P0-B** | 路由级代码分割 | `AppLayout.tsx` | 中 | 首屏 chunk 大幅瘦身 |
| **P0-C** | CodeMirror 懒加载 + manualChunks | `EndpointForm.tsx`、`vite.config.ts` | 中 | 进一步缩小首屏、提升缓存命中 |
| **P0-D** | devtools 仅 dev | `main.tsx` | 极低 | 生产包去除 devtools |
| **P1-A** | 字体子集化 | `main.tsx` | 低 | 312KB 字体显著缩减 |
| **P1-B** | 主题 FOUC 消除 | 见 P0-A 做法 2 或前置脚本 | 低 | 去除明暗闪烁 |
| **P1-C** | 合并 getConfig | `useThemeSync.ts` | 极低 | 首屏少一次 IPC |
| **P2-A** | 后端 setup 瘦身 | `db.rs`、`lib.rs` | 中 | 首启略快（需实测） |
| **P2-B** | 清理 motion 依赖 | `package.json` | 极低 | 依赖卫生 |

**落地建议路线**：

1. **第一步（半天，立竿见影）**：P0-A + P0-D + P1-C。骨架屏直接解决「白屏」观感，devtools/IPC 是顺手的零风险优化。
2. **第二步（1-2 天，核心瘦身）**：P0-B + P0-C。代码分割是体积优化的主力，需回归测试各页切换。
3. **第三步（按需）**：P1-A 字体、P1-B 主题、P2-A 后端、P2-B 依赖清理。

---

## 6. 验证方法与度量指标

优化必须以**实测**验证，避免「自我感觉良好」。

### 6.1 构建产物体积

```bash
pnpm build
ls -la dist/assets/*.js          # 观察 chunk 数量与大小
wc -c dist/assets/*.js           # 精确字节
du -ch dist/assets/*.woff2 | tail -1   # 字体总量
```

- **基线（优化前）**：单 chunk 1,050,187 字节；12 woff2 / 312KB；CSS 72,421 字节。
- **目标**：首屏入口 chunk 显著下降并拆为多个 chunk；字体收敛到中英文子集。

### 6.2 首屏时间（前端）

在 `main.tsx` 顶部与 React 首帧 effect 中打点：

```ts
performance.mark("app-script-start");          // main.tsx 顶部
// 首帧 useEffect 中：
performance.measure("first-paint", "app-script-start");
console.log(performance.getEntriesByName("first-paint")[0].duration);
```

或用 WebView 自带 DevTools 的 Performance / Network 面板观察脚本下载、解析、首次渲染时间。

### 6.3 后端 setup 耗时（验证 P2-A 是否值得做）

在 `lib.rs` setup 关键步骤间插入计时日志：

```rust
let t = std::time::Instant::now();
let pool = modules::storage::db::create_pool(&db_file)?;
tracing::info!(ms = t.elapsed().as_millis(), "create_pool 完成");
// 迁移、托盘等同理分段计时
```

> 若 `create_pool` / `run_migrations` 实测仅数毫秒，则 **P2-A 不必做**——遵循「先测量、避免过度设计」。

### 6.4 功能回归（代码分割后必测）

- 6 个页面逐一切换，确认懒加载 chunk 正常加载、`Suspense` fallback 不闪烁、无白屏。
- 端点表单打开/关闭，确认 CodeMirror 懒加载正常、JSON 编辑可用。
- 明暗主题切换、自动主题、托盘启停代理、更新检查均正常。

---

## 7. 落地检查清单

- [ ] `index.html` 加入内联骨架屏，底色与默认主题一致（P0-A）
- [ ] （可选）`tauri.conf.json` 设 `visible:false` + 首屏就绪后 `window.show()`（P0-A/P1-B）
- [ ] `AppLayout.tsx` 6 页面改 `lazy` + `Suspense`，按 `activeView` 惰性渲染（P0-B）
- [ ] `EndpointForm.tsx` 的 CodeMirror 抽为懒加载（P0-C）
- [ ] `vite.config.ts` 增加 `manualChunks`（P0-C）
- [ ] `main.tsx` devtools 改为仅 `import.meta.env.DEV` 加载（P0-D）
- [ ] `main.tsx` 字体改为拉丁子集入口（P1-A）
- [ ] `useThemeSync.ts` 复用 `useQuery(["config"])`，合并重复请求（P1-C）
- [ ] （实测后再定）`db.rs` 连接池惰性化 / setup 非必需项异步化（P2-A）
- [ ] `pnpm remove motion` 清理无用依赖（P2-B）
- [ ] 构建后对照第 6 节指标记录优化前后数据
- [ ] 6 页面 + 端点表单 + 主题 + 托盘 功能回归通过

---

## 附：核心证据索引

| 结论 | 证据位置 |
| --- | --- |
| 单一 1.05MB chunk、无分割 | `dist/assets/index-p3K1cfwD.js`（1,050,187 B）；`grep lazy/Suspense/import(` 无匹配 |
| 全部 6 页面静态实例化 | `src/layouts/AppLayout.tsx:5-22` |
| `#root` 空、无骨架屏 | `index.html:11`、`dist/index.html:13` |
| 窗口未延迟显示 | `src-tauri/tauri.conf.json:13-23`（无 `visible`） |
| devtools 进生产 | `src/main.tsx:5,34` |
| 字体全量 | `src/main.tsx:9-10`；`dist/assets/*.woff2`（12 个 / 312KB） |
| 主题 FOUC | `src/main.tsx:24` + `src/hooks/useThemeSync.ts:11-22` |
| getConfig 重复 | `src/hooks/useThemeSync.ts:13` + `src/hooks/useAutoTheme.ts:15` |
| setup 全同步 | `src-tauri/src/lib.rs:25-104`；`src-tauri/src/modules/storage/db.rs:21-23` |
| CodeMirror 仅端点表单用 | `grep codemirror` 仅命中 `src/pages/Endpoints/_components/EndpointForm.tsx` |
| motion 未使用 | `grep motion/react` 无匹配（仅 CSS `prefers-reduced-motion`） |
