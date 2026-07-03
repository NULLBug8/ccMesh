## 操作字段与非操作字段分离 + TOML 保注释

> 一句话结论：表单只改操作字段，覆写时 merge 快照，TOML 用 AST 局部更新

**你会遇到这个问题的场景**
应用管理多个第三方 CLI 的配置文件（JSON/TOML），UI 表单只编辑部分字段（API Key、Base URL），用户还在文件里手工加了注释、实验项、私有段。点「应用」不能把未编辑部分冲掉。

**为什么会出错**
全量 `serde` 序列化再写回会丢注释、字段顺序和用户自定义键。TOML 若走「parse → JSON 编辑 → 再 stringify」会重建整文件。表单字段与「渠道完整快照」混在同一层，切换渠道时易丢 live 手工改动。

**正确做法**
- 渠道存**完整配置快照**（SSOT 在应用 DB）；表单仅定义**操作字段**列表
- 覆写 live = 快照中的**非操作字段** + 表单**操作字段** deep merge
- JSON：merge 在内存 Value 上完成，再整文件写（可接受丢注释时用排序键保证确定性）
- TOML：**校验**用 `toml` crate；**保注释/局部改**用 `toml_edit`（TOML 文档 AST）按 key path 更新
- 禁止 TOML→JSON→TOML 全量重生成
- 覆写前带时间戳备份；Codex 等双文件按事务顺序写 + 失败回滚

**反例**
❌ 错误：读 TOML → 转 JSON 编辑 → `toml::to_string` 写回  
✅ 正确：`DocumentMut` 加载原文，`doc["model"] = ...`，写回保留注释

---
_最后更新：2026-06-28_
