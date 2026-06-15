# 配置文件管理页面（Claude Code / Codex 抽取-存储-应用-覆盖）

## Goal
在 ccMesh 中新增一个「配置文件」页面，让用户以"渠道"为单位管理本机 Claude Code 与 Codex 的配置：从真实配置**抽取**基线、以表单**存储**操作字段、**应用**整合出完整配置、并原子**覆盖**写回真实配置文件，且不破坏用户原有的非操作字段/注释。

## Requirements
- 顶部 Tab 切换 Claude / Codex 两套配置界面。
- 三栏布局 + 底部固定操作区：
  - 左栏：当前工具的已保存渠道列表（读工作目录），顶部有"新增"图标按钮；列表项可删除（右键或行内入口）。
  - 中栏：渠道表单（顶部子 Tab：`端点配置写入` / `自定义配置写入`）+ 表单下方"操作字段编辑器"（随表单实时联动，支持格式化，默认可编辑）。
  - 右栏：整合后的完整配置编辑器，顶部开关默认关闭（只读），开启后可改，带格式化按钮。
  - 底部：固定操作区，未加载渠道时按钮置灰；点击"应用"提交覆写。
- 中栏初始空态：中间空图标占位、右侧空编辑器、底部按钮灰；点"新增"后加载空配置页。
- `端点配置写入` 子 Tab：base_url 自动取本机网关地址（地址+端口），模型从"可对外暴露的模型"中选择。
- `自定义配置写入` 子 Tab：base_url 与模型均自由填写。

## Implementation Decisions
- **范围**：本期仅 Claude Code 与 Codex；其它工具（Desktop/Gemini/OpenCode 等）不做。
- **存储模型 = 每渠道完整快照**：渠道目录存该工具的**完整配置**（操作字段+非操作字段一起）。新增渠道时从真实源配置抽取一份基线快照；表单只编辑操作字段，非操作字段随快照保留。应用时把"快照中的非操作字段 + 表单/编辑器中的操作字段"整合为完整配置整体写回。
- **工作目录**：放 ccMesh 应用数据目录下 `profiles/`（由 Tauri `app_data_dir` 解析，Windows 即 `%APPDATA%/<bundleId>/profiles/`）。
  - `profiles/claude_code/<渠道>/settings.json`
  - `profiles/claude_code/claude.record.json`（源 `~/.claude/settings.json` 的抽取基线备份，非渠道）
  - `profiles/codex/<渠道>/config.json`（含 auth.json 内容 + config.toml 原文/解析）
  - `profiles/codex/codex.record.json`（源 `~/.codex/{auth.json,config.toml}` 的抽取基线备份，非渠道）
- **Claude 字段契约**（操作字段）：
  - 地址 → `env.ANTHROPIC_BASE_URL`
  - 秘钥 → `env.ANTHROPIC_API_KEY`（**按需求正文，使用 API_KEY 而非 AUTH_TOKEN**）
  - Sonnet/Opus/Haiku 显示名 → `env.ANTHROPIC_DEFAULT_SONNET_MODEL / _OPUS_MODEL / _HAIKU_MODEL`
  - 默认兜底模型 → `env.ANTHROPIC_MODEL`（可留空）
  - 每个模型可勾选 `[1m]`：勾选则模型名追加 `[1m]` 后缀（上下文能力声明），不勾选用原名。
- **Codex 字段契约**（操作字段）：
  - 秘钥 → `auth.json` 的 `OPENAI_API_KEY`
  - 地址 → `config.toml` 中 active provider 的 `base_url`
  - 默认模型 → `config.toml` 的 `model`
  - 审核模型 → `config.toml` 的 `review_model`
  - 其余 TOML 字段为模板/非操作字段，原样保留。
- **Codex TOML 保真**：渠道额外保留 `config.toml` **原始文本（含注释）**；覆写时基于原文用 `toml_edit` 做字段级更新，不做 TOML→JSON→TOML 全量重生成。`config.json` 内同时存解析出的 JSON 供表单展示。
- **覆写前自动备份**：每次"应用"覆写真实文件前，先把原文件复制一份带时间戳的备份到工作目录（可回滚）。
- **原子写入**：所有写真实配置文件与渠道文件，统一走"同目录临时文件 + flush + rename"（Windows 先删目标再 rename）。
- **开关项（本期纳入，data-driven）**：表单上方一排开关（Claude：隐藏 AI 署名 / Teammates 模式 / 启用 Tool Search / 最大强度思考 等；Codex：启用 Goal mode / 启用远程压缩 / 写入通用配置 等）以**声明式映射表**驱动（每项定义：作用工具、目标键、开启写入值、关闭行为）。**具体开关清单与字段映射待产品确认**（见 feature.md 附录占位）。
- **端点配置写入 数据来源**：base_url 取 ccMesh 自身网关（复用 `get_config` 的 port 拼 `http://127.0.0.1:<port>`，Codex 末尾补 `/v1`）；可选模型取已配置的对外暴露模型。
- **i18n**：与现有页面一致，本期文案以中文为主（导航沿用 label/labelEn）。

## Acceptance Criteria
- [ ] 新增导航项"配置文件"，点击进入新页面；Claude/Codex Tab 可切换。
- [ ] 左栏正确读出工作目录下的渠道列表；新增、删除渠道可用且即时刷新。
- [ ] 新增渠道时能从真实源配置抽取基线并生成/更新 `*.record.json`。
- [ ] `端点配置写入` Tab 的 base_url 自动填本机网关地址+端口，模型来自对外模型列表。
- [ ] 中栏操作字段编辑器随表单实时联动、可格式化。
- [ ] 右栏整合编辑器默认只读、开关开启可编辑、可格式化，内容为完整配置。
- [ ] 点"应用"：先备份原文件，再原子覆写真实配置文件；Claude 写 `~/.claude/settings.json`，Codex 写 `~/.codex/auth.json` + `~/.codex/config.toml`（保留注释/非操作字段）。
- [ ] 开关项按确认后的映射表正确写入/移除对应字段。
- [ ] Rust 端单测覆盖：原子写入、操作字段合并、Codex TOML 字段级更新保注释、快照往返。
- [ ] `pnpm check`（tsc + cargo check）与 `pnpm test` / `cargo test` 通过。

## Definition of Done
- 主链路（抽取→存储→应用→覆盖）在 Claude 与 Codex 上跑通；
- 单测通过、类型检查通过；
- 无头环境无法验证的 GUI/真实文件覆写交互显式声明本地核对清单；
- 按模块 scoped 提交，progress.csv 更新，任务归档。

## User Stories
- 作为多渠道用户，我希望为 Claude/Codex 各保存多套配置渠道并一键应用，以便在不同中转/模型间快速切换而不手改文件。
- 作为重度用户，我希望应用某渠道时保留我配置文件里的自定义/注释字段，以便不丢失个人设置。
- 作为接入 ccMesh 网关的用户，我希望"端点配置写入"自动填好本机网关地址与可用模型，以便零手填接入。

## Testing Decisions
- Rust：`#[cfg(test)]` 覆盖 atomic_write（temp dir）、Claude 操作字段 deep merge、Codex `toml_edit` 字段级更新（断言注释/未触及字段保留）、record 抽取与快照往返。
- 前端：操作字段↔表单映射、整合预览、`[1m]` 后缀、网关地址拼接等纯函数单测；JsonEditor 格式化/只读 smoke。
- GUI 三栏交互、真实覆写 `~/.claude`、`~/.codex` 文件：无头不可验，给本地核对清单。

## Out of Scope
- Claude Desktop / Gemini / OpenCode / OpenClaw / Hermes 等其它工具。
- 渠道间的代理热切换 / proxy takeover（cc-switch 的 takeover 机制）。
- 通用配置（common config 片段）的完整管理 UI（"写入通用配置"开关若纳入则仅做最小写入，编辑通用配置入口可后置）。
- 云端/WebDAV 同步这些渠道文件。

## Technical Notes
- 参考 `research/cc-switch-config-io.md`（原子写、双文件事务、TOML 保注释、快照模型）与 `research/ccmesh-architecture.md`（页面落点、命令注册、缺 toml/原子写工具）。
- 新增后端 crate 依赖：`toml`（解析）+ `toml_edit`（保注释字段级写）。
- 复用：`utils/paths.rs`（home/app_data）、`request()` 封装、`@uiw/react-codemirror`、shadcn UI（Tabs/Switch/Dialog/Button/ScrollArea）。
