# ccMesh

ccMesh 是一个 Web 管理界面 + Rust HTTP 后端的 AI 代理网关。它提供端点管理、模型映射、路由轮询、熔断/降级规则、余额查询、请求日志、统计和 OpenAI / Claude / Codex 协议转发能力。

## 功能

- Web 控制台：仪表盘、端点、余额、规则、统计、日志、设置。
- 代理服务：按顺序轮询端点，支持模型映射、健康测试、熔断、降级和请求追踪。
- 协议兼容：OpenAI、Claude、Codex Responses 相关转发与必要转换。
- 运维配置：端口、代理、User-Agent 伪装、全局测试模型、Token 估算。

## 开发

```bash
pnpm install
pnpm dev
```

前端开发服务默认监听 `http://127.0.0.1:5173`。

## 后端运行

```bash
pnpm server:dev
```

常用环境变量：

- `CCMESH_PORT`：后端 HTTP 端口，例如 `3001`。
- `CCMESH_DATA_DIR`：数据目录；不设置时默认使用系统应用数据目录下的 `ccmesh`。

示例：

```powershell
$env:CCMESH_PORT="3001"
$env:CCMESH_DATA_DIR="C:\Users\big-s\AppData\Roaming\ccmesh"
pnpm server:dev
```

访问：`http://127.0.0.1:3001/`。

## 构建

```bash
pnpm build
pnpm server:build
```

构建后的后端二进制在 `src-tauri/target/release/`，Web 静态资源在 `dist/`。

## 服务器部署

完整教程见：[docs/deploy/server.md](docs/deploy/server.md)。

生产部署核心步骤：

```bash
pnpm install
pnpm build
pnpm server:build
CCMESH_PORT=3999 CCMESH_DATA_DIR=/var/lib/ccmesh ./src-tauri/target/release/ccmesh
```

访问：`http://服务器IP:3999/`。

## 验证

```bash
pnpm test
pnpm check:front
pnpm check:rust
cargo test --manifest-path src-tauri/Cargo.toml
```

## 技术栈

Rust、axum、reqwest、SQLite、React 19、TypeScript、Vite、TanStack Query、Tailwind CSS、CodeMirror。
