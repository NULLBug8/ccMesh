# 警告修复记录（npm run tauri dev）

> 日期：2026-06-07
> 背景：`npm run tauri dev` 编译阶段(`cargo run`，dev profile)输出 10 条 Rust 死代码警告。
> 目标：全部消除，使 `tauri dev` 零警告零错误。

## 验证结果

- `cargo build`：**0 警告 0 错误**
- `cargo test --lib`：**61 通过**
- 实跑 `npm run tauri dev`：cargo `Finished dev profile`（无任何 warning）、Vite 正常启动、应用启动、迁移 v3/v4 在真实库成功应用。
- 唯一保留的 `Warn Waiting for your frontend dev server to start...` 是 tauri-cli 等待 Vite 启动的正常提示，非代码/构建警告，无法也无需消除。

## 修复清单（10 条警告）

| # | 位置 | 警告 | 处理方式 |
|---|------|------|----------|
| 1 | `error.rs` | variant `Transform` is never constructed | 删除未使用的 `AppError::Transform` 变体 |
| 2 | `models/endpoint.rs` | struct `EndpointCredential` is never constructed | 删除孤立结构体（无任何引用；`endpoint_credentials` 表保留） |
| 3 | `models/proxy.rs` | enum `ClientFormat` is never used | 删除未使用枚举（实际使用的是 `transform::UpstreamFormat`） |
| 4 | `modules/proxy/forward.rs` | method `has_active` is never used | 删除 `ActiveRequests::has_active` |
| 5 | `modules/proxy/forward.rs` | field `app_handle` is never read | 删除 `ProxyState.app_handle` 字段（代理事件经 `stats` 推送，该字段冗余）；连带清理 `start_proxy` 形参与 `AppHandle` import |
| 6 | `modules/proxy/resolver.rs` | field `model_override` is never read | 加 `#[allow(dead_code)]` + 注释保留（`@端点/模型` 语法已解析且有单测，转发侧应用待接入，非孤立代码） |
| 7 | `modules/proxy/streaming.rs` | function `is_event_stream` is never used | 删除整个文件（P1 遗留，已被 `transform/streaming.rs` 取代）+ 移除 `proxy/mod.rs` 中模块声明 |
| 8 | `modules/storage/db.rs` | type alias `DbConn` is never used | 删除未使用类型别名 |
| 9 | `modules/transform/transformer.rs` | methods `name` and `transform_response` are never used | 从 `Transformer` trait 及 `IdentityTransformer` / `ClaudeOpenAiTransformer` 实现中删除这两个方法（转发只用 `transform_request`；响应转换走自由函数 `openai_response_to_claude`） |
| 10 | `modules/transform/types.rs` | field `finish_reason_sent` is never read | 删除 `StreamContext.finish_reason_sent` 字段 |

## 修复过程中触发并解决的编译错误

| 位置 | 错误 | 处理方式 |
|------|------|----------|
| `commands/proxy.rs` / `commands/config.rs` | `E0061: start_proxy 期望 3 个参数但提供了 4 个`（删除 `app_handle` 形参后的连带影响） | 更新两处 `start_server(...)` 调用去掉 `app.clone()`；`set_config` 的 `app: AppHandle` 形参随之无用，一并删除并移除 `AppHandle` import |

## 处理原则

- 编译器确认的孤立项（无任何引用、无测试、无意图信号）→ **直接删除**，符合“零死代码”。
- 有单测/有明确意图但尚未接入热路径的项（`model_override`）→ **`#[allow(dead_code)]` 保留**，不删除已测试的设计，也不在“修警告”任务中擅自改动代理热路径行为。
- 删除字段/形参时同步更新所有调用点与 import，保护调用链不破坏现有功能（61 单测全绿验证）。
