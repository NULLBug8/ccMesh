# 构建与发布（ccNexus）

> 阶段 11（P11-3）交付物。本文件说明本地/CI 出包流程、更新构件签名与注意事项。
> 应用元信息（productName=ccNexus、identifier=com.ccnexus.desktop、窗口 1200×800/最小 940×600/居中/无边框）、
> 图标（`src-tauri/icons/`）、CSP 与 capabilities 收敛已在 P11-1/P11-2 落地。

## 新环境是否需要安装依赖？

**结论：项目依赖无需手动逐个安装。** 所有 Rust crate 与 npm 包都由清单 + 锁文件固定，克隆后一次性还原即可——开发期间用过的 `cargo add` / `pnpm add` 是「写入清单」的一次性动作，新环境不需要重复。

```bash
# 1) 还原前端依赖（读 package.json + pnpm-lock.yaml）
pnpm -C r-app install

# 2) 启动开发（首次会自动用 cargo 还原并编译全部 Rust crate）
pnpm -C r-app tauri dev
#   或仅校验后端： (cd r-app/src-tauri && cargo check)
```

> 首次 `cargo` 编译较慢（需编译内置 SQLite 的 C 源码、tokio、reqwest、axum、tauri 等），之后增量缓存。

**需要手动安装的是「系统级前置」（不在锁文件里）：**

| 前置 | 用途 | 备注 |
|------|------|------|
| Rust stable + rustup | 后端编译 | 已验证 1.96；Windows 用 `x86_64-pc-windows-msvc` |
| C/C++ 构建工具 | **必需** | `rusqlite`/`r2d2_sqlite` 用 `bundled` 编译 SQLite C 源；Windows 装「MSVC Build Tools / VS C++ 生成工具」，Linux 用 `gcc`，macOS 用 Xcode CLT |
| Node + pnpm | 前端依赖与脚本 | 本仓库 pnpm 走 npmmirror |
| WebView2 Runtime | Tauri 运行时（Windows） | Win10/11 多已预装 |
| 平台 GUI 依赖 | Tauri 运行（非 Windows） | Linux：`libwebkit2gtk-4.1`、`libayatana-appindicator3`、`librsvg2` 等；macOS：Xcode CLT |
| NSIS / WiX v3 | **仅打包** `tauri build` 出 `.exe`/`.msi` | 开发（`tauri dev`）不需要 |
| 更新签名密钥 | **仅发布更新构件** | 见下文「自动更新构件与签名」 |

## 依赖清单（已写入清单/锁文件，自动还原）

**后端 Rust（`src-tauri/Cargo.toml`）**
- 框架/插件：`tauri`（features `tray-icon`/`image-png`）、`tauri-plugin-updater`、`tauri-plugin-process`、`tauri-build`
- 代理/网络：`axum`、`hyper`、`tower`、`tower-http`、`reqwest`（gzip/json/stream）、`reqwest_dav`、`tokio`（full）、`tokio-util`、`tokio-stream`、`futures`、`async-trait`
- 存储：`rusqlite`（bundled）、`r2d2`、`r2d2_sqlite`（bundled）
- 序列化/工具：`serde`、`serde_json`、`thiserror`、`chrono`（serde）、`uuid`（v4）、`tracing`、`tracing-subscriber`（env-filter）

> 说明：阶段 11 已移除未使用的 `tauri-plugin-opener`（后端插件 + 能力 + 前端 `@tauri-apps/plugin-opener` 一并清理）。updater/process 仅经后端命令封装调用，前端未引入对应 JS 插件包。

**前端 npm（`package.json`）**
- UI/样式：`tailwindcss` 4 + `@tailwindcss/vite`、`tw-animate-css`、`radix-ui`、`class-variance-authority`、`clsx`、`tailwind-merge`、`lucide-react`、`motion`、`sonner`、`next-themes`
- 状态/数据：`zustand`、`@tanstack/react-query`(+devtools)
- 编辑器/字体：`@uiw/react-codemirror` + `@codemirror/*`、`@fontsource-variable/{inter,jetbrains-mono}`
- 运行时/框架：`react`/`react-dom` 19、`@tauri-apps/api`
- devDeps：`vite`、`@vitejs/plugin-react`、`typescript`、`@tauri-apps/cli`、`vitest`、`jsdom`、`@testing-library/{react,jest-dom}`、`@types/*`

## 前置工具（打包用，摘要）

- Windows 打包：WebView2 Runtime、NSIS（`.exe`）/ WiX Toolset v3（`.msi`）
- 跨平台：macOS 需 Xcode CLT；Linux 需 `libwebkit2gtk`、`libayatana-appindicator` 等

## 构建命令

```bash
# 前端类型检查 + 构建（已由 beforeBuildCommand 自动触发）
pnpm -C r-app build

# 产出安装包（Windows: nsis + msi；产物在 src-tauri/target/release/bundle/）
pnpm -C r-app tauri build
```

产物位置：`r-app/src-tauri/target/release/bundle/{nsis,msi}/`，主程序 `…/release/ccNexus.exe`。

## 自动更新构件与签名（待分发渠道确定后启用）

`tauri.conf.json` 已设 `bundle.createUpdaterArtifacts: true`，`plugins.updater.{endpoints,pubkey}` 暂留空。
正式发布更新时：

```bash
# 1. 生成签名密钥对（一次）
pnpm -C r-app tauri signer generate -w ~/.tauri/ccnexus.key

# 2. 将公钥填入 tauri.conf.json -> plugins.updater.pubkey
#    将更新服务器地址填入 plugins.updater.endpoints（如 GitHub Releases / 自托管 JSON）

# 3. 构建时提供私钥（否则 createUpdaterArtifacts 会因缺少签名密钥而失败）
TAURI_SIGNING_PRIVATE_KEY="$(cat ~/.tauri/ccnexus.key)" \
TAURI_SIGNING_PRIVATE_KEY_PASSWORD="<密码>" \
pnpm -C r-app tauri build
```

> 注：在未配置签名密钥的环境中如仅需普通安装包，可临时将 `createUpdaterArtifacts` 置为 `false`。

## 状态

- 配置层（元信息/图标/CSP/capabilities/更新构件开关）已就绪，`cargo check` + `pnpm build` + `pnpm test` 全绿。
- 实际安装包产出与更新构件签名需在具备打包工具链（NSIS/WiX）与签名密钥的构建机 / CI 上执行（参考旧版 `.github/workflows/build.yml`）。
- `endpoints` / `pubkey` 待分发渠道确定后填入。
