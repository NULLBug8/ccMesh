# cc-switch 配置文件读写流程调研

> 来源：只读探索 `F:\IT\cc-switch`（Tauri2 + React/TS + Rust）。真实源码在根目录 `src/` 与 `src-tauri/`；`cc-switch-main/` 仅含 1 个预设 TS 文件，可忽略。

## 1. 配置目录 / 文件定位
- 跨平台 home：`src-tauri/src/config.rs:22-34` `get_home_dir()` → `dirs::home_dir()`（不直接用 `HOME` 环境变量，避免 MSYS 漂移）；测试用 `CC_SWITCH_TEST_HOME` 覆盖。
- Claude Code 配置：`~/.claude/settings.json`（`config.rs:74-87` `get_claude_settings_path()`，兼容 `claude.json`）。
- Codex：`~/.codex/auth.json`（`codex_config.rs:40-42`）+ `~/.codex/config.toml`（`codex_config.rs:45-47`）+ 模型目录 `~/.codex/cc-switch-model-catalog.json`。
- 应用数据：`~/.cc-switch/`；SSOT 存储 SQLite `~/.cc-switch/cc-switch.db`；设备设置 `~/.cc-switch/settings.json`。
- 目录可被 UI override：`settings.rs:774-796` `get_claude_override_dir()/get_codex_override_dir()`，路径解析支持 `~` 前缀。

## 2. 读取流程
- JSON：`serde_json`，`config.rs:153-161` `read_json_file<T>()`（不存在即报错 → 读字符串 → from_str）。
- TOML 校验/解析：`toml` crate 0.8（`toml::Table`）；需保注释/局部编辑用 `toml_edit` 0.22（`DocumentMut` AST）。
- Codex 聚合读取：`codex_config.rs:1027-1044` `read_codex_live_settings()` → `{ auth, config }`（auth 缺失→`{}`，两者皆空→报错）。

## 3. 写入流程（重点：原子写入）
- 核心 primitive：`config.rs:204-258` `atomic_write(path, data)`：
  1. 同目录建 `{filename}.tmp.{nanos}` 临时文件；
  2. `write_all` + `flush()`（**无显式 fsync**）；
  3. Unix 复制原文件权限；
  4. Windows 先 `remove_file(目标)` 再 `rename`；非 Windows 直接 `rename`（POSIX 原子替换）。
- JSON 写：`config.rs:181-193` `write_json_file()` → 递归**按字母序排序所有键** → pretty → atomic_write（**不保留**原字段顺序/注释）。
- 纯文本/TOML 写：`config.rs:196-201` `write_text_file()` → atomic_write（原样）。
- Codex 双文件事务：`codex_config.rs:84-131` `write_codex_live_atomic(auth, config_text)`：读旧值用于回滚 → TOML 预校验 → 写 auth.json → 写 config.toml（失败回滚 auth.json）。仅写 config.toml（保 OAuth）：`write_codex_live_config_atomic()` `:181-193`。

## 4. 多渠道 / Provider 存储模型
- SSOT = SQLite `providers` 表（`database/schema.rs:27-42`）：`id, app_type, name, settings_config(JSON 字符串), is_current, ...`，主键 `(id, app_type)`。
- Provider 结构 `provider.rs:11-44`：`settingsConfig` 存各 app 的 live 完整快照（Codex=`{auth, config, modelCatalog?}`）；`meta` 仅存 DB 不写 live。**SSOT 模式，不再写供应商副本文件**。
- 切换写 live 三部曲：`services/provider/mod.rs:1659-1734` `switch_normal`：
  1. backfill：读当前 live → strip common config → 保存回旧 provider；
  2. 设置 current（settings.json + DB is_current）；
  3. merge common config 后 `write_live_snapshot` 写 live。
- Codex 写入路由：`live.rs:740-755` / `codex_config.rs:1051-1065` `write_codex_live_for_provider`：official+有登录态 或 第三方未开 preserve → 写 auth+config；第三方+preserve → 只写 config.toml，把 API key 投影到 `experimental_bearer_token`。

## 5. 备份 / 归档
- legacy `config.json` 保存前备份 `~/.cc-switch/config.json.bak`（`app_config.rs:628-634`）。
- 导出备份 `~/.cc-switch/backups/backup_YYYYMMDD_HHMMSS.json`（保留 10 个）。
- 代理接管 live 备份存 `proxy_live_backup` 表。
- from-live 导入：`live.rs:1121-1252` `import_default_config()`：读 live → 建 id=default provider → 入库设 current（启动时 providers 空则自动触发）。
- **Live vs SSOT**：live=工具真实文件（运行时生效）；SSOT=provider 库；切换=DB→Live 单向写；backfill=Live→DB 回读保留手工改动。

## 6. 操作字段 vs 非操作字段（部分写 / 合并）
- Common config 片段（跨 provider 共享）：存 DB `config_snippets`；写 live 前 merge，backfill 时 strip。Claude 用 `json_deep_merge`/`json_deep_remove`；Codex 用 `toml_edit` 表级 merge/remove；Gemini 对 env 对象 merge/remove。
- JSON 写虽全量重写+排序，但 deep merge 发生在写入前的内存 Value 上，未被 merge 触及的字段随整文件保留。
- Codex 私有字段：`modelCatalog` 仅 DB，live 只放指针文件；`experimental_bearer_token` backfill 时回填到 `auth.OPENAI_API_KEY`。
- Claude 写 live 前剥离 cc-switch 私有字段（`live.rs:24-34`：`api_format` 等）。

## 7. 关键 Tauri 命令（配置读写相关）
- Provider：`get_providers / get_current_provider / add_provider / update_provider / delete_provider / switch_provider / import_default_config / read_live_provider_settings`（`commands/provider.rs`）。
- 路径/状态：`get_config_status / get_claude_code_config_path / get_config_dir / open_config_folder`（`commands/config.rs`）。
- Common config：`get/set/extract_common_config_snippet`。
- 前端 service：`src/lib/api/providers.ts`、`settings.ts` 用 `invoke`。

## 8. Codex 前端合并编辑
- `useCodexConfigState.ts`：两个 state `codexAuth`(JSON 字符串) + `codexConfig`(TOML 字符串)；API Key 优先 `auth.OPENAI_API_KEY` 回退 `experimental_bearer_token`，改 key 时同步两处；Base URL 经 `setCodexBaseUrlInConfig` 写 TOML；TOML 用 `smol-toml`。保存时把 `{auth, config, modelCatalog}` 作为 settingsConfig 提交。

## 9. 复刻到 ccMesh 的可借鉴要点
1. 分层 SSOT：工具 live 文件 ≠ 应用配置库；存各 app 完整快照 + 应用侧 meta 不写 live。
2. 切换三部曲（backfill → set current → merge 后写 live），避免丢失 live 手工改动。
3. 统一原子写 primitive：`tmp.{nanos}` + flush + rename（Windows 先删后 rename）。
4. 多文件事务（Codex）：顺序写 + 失败回滚。
5. TOML 两套策略：校验用 `toml`；保注释/局部更新用 `toml_edit`；整段替换可接受。
6. JSON 确定性输出：写前递归排序键，利于 diff/测试。
7. 部分字段路由：第三方只改 config.toml，OAuth 留 auth.json。
8. 路径解析 hardened：`dirs::home_dir()` + 可选 override + 测试用环境变量。
