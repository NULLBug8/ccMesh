# 2027 配置文件管理页面（Claude/Codex 抽取-存储-应用-覆盖）

## 目标
落地 PRD：新增「配置文件」页面，按渠道管理 Claude Code / Codex 配置，跑通 抽取→存储→应用→覆盖 主链路，原子写入 + 覆写前备份 + 保留非操作字段/注释。

## 现状（根因）
- 前端无 react-router，靠 `useLayoutStore.activeView` + 懒加载切页；Endpoints 提供 CRUD/表单/CodeMirror 范本但布局是单栏，需自拼三栏。
- 后端 **无任何 Claude/Codex 配置读写命令**，无 `toml`/`toml_edit`/`dirs` 依赖，无通用原子写工具。`utils/paths.rs` 仅有 `app_data_dir/db_path/home_dir`。

## 关键文件/落点
### 后端（src-tauri/src）
- 新增 `utils/atomic_write.rs`：`atomic_write(path,&[u8])`（同目录 `*.tmp.{nanos}` + flush + rename，Windows 先删后 rename）。
- 改 `utils/paths.rs`：加 `claude_settings_path()`=`~/.claude/settings.json`、`codex_auth_path()`=`~/.codex/auth.json`、`codex_config_path()`=`~/.codex/config.toml`、`profiles_dir(app)`=`app_data_dir/profiles`、`claude_profiles_dir/codex_profiles_dir`。
- 改 `utils/mod.rs`：导出 atomic_write。
- 新增 `models/tool_config.rs`：DTO（见数据契约）；改 `models/mod.rs` 导出。
- 新增 `modules/tool_config/mod.rs`、`claude.rs`、`codex.rs`：抽取/读写渠道/合并/应用；改 `modules/mod.rs` 导出。
- 新增 `commands/tool_config.rs`：命令薄层；改 `commands/mod.rs` 导出。
- 改 `lib.rs`：`invoke_handler` 注册新命令。
- 改 `Cargo.toml`：加 `toml`、`toml_edit`。
### 前端（src）
- 改 `stores/modules/layout.ts`：`ViewId` 加 `"configProfiles"`。
- 改 `layouts/navConfig.tsx`：加导航项「配置文件」(icon `FileCogIcon`/`Settings2Icon`)。
- 改 `layouts/AppLayout.tsx`：lazy import + `PAGES` 注册。
- 新增 `pages/ConfigProfiles/index.tsx` + `_components/{ChannelList,ChannelForm,ClaudeForm,CodexForm,OperFieldsEditor,MergedConfigEditor,ApplyBar,FeatureToggles}.tsx`。
- 新增 `components/common/JsonEditor.tsx`（从 Endpoints 提升 + `readOnly/onFormat/height/lang` props）；改 `pages/Endpoints/_components/EndpointForm.tsx` 复用（可选）。
- 新增 `services/modules/tool_config.ts` + 改 `services/index.ts`；新增类型（`services/types` 或就近）。
- 新增 `hooks/useToolConfigChannels.ts`。
- 新增纯函数 `lib/toolConfig/*`：操作字段↔表单、整合预览、`[1m]` 后缀、网关地址拼接（便于单测）。

## 任务拆解
- **2029.1** 后端基础：`utils/atomic_write.rs` + `paths.rs` 路径扩展 + Cargo 加 toml/toml_edit + 单测（atomic_write）。
- **2029.2** 后端 DTO：`models/tool_config.rs`（ChannelMeta / ClaudeChannel / CodexChannel / OperationFields / ApplyRequest / ExtractResult）。
- **2029.3** `modules/tool_config/claude.rs`：抽取 record、列/读/存/删渠道、操作字段 deep merge 进 settings.json 快照、整合完整配置、备份+原子覆写 + 单测。
- **2029.4** `modules/tool_config/codex.rs`：auth/config 双文件读写、`toml_edit` 字段级更新（base_url/model/review_model 保注释）、record、整合、备份+双文件原子写（失败回滚）+ 单测。
- **2029.5** `modules/tool_config/mod.rs` + `commands/tool_config.rs` + `lib.rs` 注册：list/get/save/delete/extract/preview/apply 命令。
- **2029.6** 前端 service + 类型 + `useToolConfigChannels` hook。
- **2029.7** 共享 `components/common/JsonEditor.tsx`（readOnly/format/height/lang）+ 纯函数 `lib/toolConfig/*` + 单测。
- **2029.8** 页面骨架 + 导航接入：ViewId/navConfig/AppLayout/三栏布局+顶 Tab(Claude/Codex)+底部 ApplyBar 灰态/空态占位。
- **2029.9** 左栏 ChannelList（读列表/新增/删除/选中）。
- **2029.10** 中栏 ClaudeForm/CodexForm（端点/自定义子 Tab + 模型 `[1m]` + 操作字段编辑器实时联动）。
- **2029.11** 右栏 MergedConfigEditor（只读开关 + 格式化）+ 应用流程（确认→备份→覆写→toast→刷新）。
- **2029.12** 开关项 FeatureToggles（data-driven，按确认映射）+ 整体回归（tsc+cargo check+vitest+cargo test）+ 本地核对清单。

## 数据契约
```jsonc
// 渠道列表项
ChannelMeta { id: string, name: string, appType: "claude"|"codex", updatedAt: string }

// Claude 渠道存储（profiles/claude_code/<id>/settings.json 即完整快照）
// 表单操作字段（前端态）
ClaudeOperationFields {
  baseUrl: string,            // env.ANTHROPIC_BASE_URL
  apiKey: string,             // env.ANTHROPIC_API_KEY
  sonnetModel: string, sonnetIs1m: boolean,   // env.ANTHROPIC_DEFAULT_SONNET_MODEL (+[1m])
  opusModel: string,   opusIs1m: boolean,     // env.ANTHROPIC_DEFAULT_OPUS_MODEL
  haikuModel: string,  haikuIs1m: boolean,    // env.ANTHROPIC_DEFAULT_HAIKU_MODEL
  defaultModel: string,       // env.ANTHROPIC_MODEL（可空）
  toggles: Record<string,boolean>
}

// Codex 渠道存储（profiles/codex/<id>/config.json）
CodexChannelFile {
  auth: { OPENAI_API_KEY: string, ... },   // auth.json 全量
  configToml: string,                       // config.toml 原文（保注释，覆写真相源）
  config: object                            // toml 解析出的 JSON（仅表单展示）
}
CodexOperationFields {
  apiKey: string,             // auth.OPENAI_API_KEY
  baseUrl: string,            // [model_providers.<active>].base_url
  model: string,              // model
  reviewModel: string,        // review_model
  toggles: Record<string,boolean>
}

// 命令（snake_case 命令名 / camelCase 参数）
list_profile_channels(appType) -> ChannelMeta[]
extract_source_record(appType) -> { exists: bool, snapshot: object }   // 读真实配置 → 写 *.record.json
get_profile_channel(appType, id) -> 完整渠道数据（快照 + 解析出的操作字段）
save_profile_channel(appType, channel) -> ChannelMeta   // 写渠道目录文件（原子）
delete_profile_channel(appType, id) -> void
preview_merged_config(appType, channel) -> string       // 整合完整配置文本（也可前端算）
apply_profile_config(appType, id, fullConfig) -> void   // 备份原文件 → 原子覆写真实配置
```

## 验收标准
对齐 prd.md「Acceptance Criteria」全部条目。

## 测试点
- Rust：atomic_write 写后内容一致且无残留 tmp；Claude deep merge 保留非操作字段；Codex toml_edit 改 base_url/model 后注释与其它表/键不变；record 抽取与快照往返；备份文件生成。
- 前端：操作字段↔settings.json 片段映射；`[1m]` 后缀拼接；网关地址拼接（Claude 无 /v1、Codex 有 /v1）；整合预览合并；JsonEditor 格式化/只读。

## 提交策略（scoped，按模块）
1. `docs(task-plan): config-entry PRD/feature/research/progress`（仅 docs/task-plan 下本任务文件）。
2. `feat(tauri): atomic_write + paths + toml deps`（2029.1）。
3. `feat(tauri): tool_config models`（2029.2）。
4. `feat(tauri): claude config extract/merge/apply`（2029.3）。
5. `feat(tauri): codex config toml-edit/apply`（2029.4）。
6. `feat(tauri): tool_config commands + register`（2029.5）。
7. `feat(web): tool_config service/hook/types + shared JsonEditor`（2029.6/2029.7）。
8. `feat(web): config profiles page skeleton + nav`（2029.8/2029.9）。
9. `feat(web): channel forms + operation editor`（2029.10）。
10. `feat(web): merged editor + apply flow`（2029.11）。
11. `feat(web): feature toggles + regression`（2029.12）。
> 绝不 `git add -A`/`.`；每组只 `git add` 精确文件，提交前 `git status --short` 核对。

## 附录 A：开关项映射表（已对齐 cc-switch，2029.12 已实现）
> 来源：cc-switch `src/components/providers/forms/CommonConfigEditor.tsx`、`CodexConfigSections.tsx`、`utils/providerConfigUtils.ts`（见 research）。
> 已实现：以下 6 个干净映射的开关（前端 `lib/toolConfig.ts` 的 `CLAUDE_TOGGLE_DEFS` + 后端 `codex::apply_goal_mode_doc`）。

### Claude（settings.json）— 已实现（5）
| 开关 | 目标键 | 开启值 | 关闭行为 |
|---|---|---|---|
| 隐藏 AI 署名 | `attribution` | `{commit:"",pr:""}` | 删除 `attribution` |
| Teammates 模式 | `env.CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS` | `"1"` | 删键 |
| 启用 Tool Search | `env.ENABLE_TOOL_SEARCH` | `"true"` | 删键 |
| 最大强度思考 | `env.CLAUDE_CODE_EFFORT_LEVEL` | `"max"` | 删键 |
| 禁用自动升级 | `env.DISABLE_AUTOUPDATER` | `"1"` | 删键 |

### Codex（config.toml）— 已实现（1）
| 开关 | 目标键 | 开启值 | 关闭行为 |
|---|---|---|---|
| 启用 Goal mode | `features.goals` | `true` | 删键；`[features]` 空则删表 |

### 本期暂缓（依赖 ccMesh 暂无的基础设施，UI 已给出说明）
| 开关 | 暂缓原因 |
|---|---|
| 启用远程压缩 | cc-switch 通过改 `model_providers.<active>.name="OpenAI"` 给其代理发信号；ccMesh 网关无此命名约定，照搬无意义。 |
| 写入通用配置 | 依赖跨渠道共享的"通用配置片段"存储 + 编辑 UI（cc-switch common config snippet），属独立特性，见 prd.md Out of Scope。 |
