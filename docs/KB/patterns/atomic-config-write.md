## 本地配置文件原子写 primitive

> 一句话结论：同目录 tmp + flush + rename，Windows 先删后换

**你会遇到这个问题的场景**
桌面应用或 CLI 工具直接读写用户 home 下的 JSON/TOML 配置。进程崩溃、杀进程或断电时，半写文件会导致 live 配置损坏，工具无法启动。

**为什么会出错**
直接 `write(path)` 非原子：写到一半进程退出则文件截断。跨盘 rename 不原子。Windows 上 `rename` 覆盖已存在目标可能失败。

**正确做法**
- 同目录创建 `{filename}.tmp.{nanos}` 临时文件
- `write_all` + `flush()`（是否 fsync 视 durability 需求权衡）
- Unix：复制原文件权限后 `rename(tmp, target)`（POSIX 原子替换）
- Windows：若目标存在先 `remove_file(target)` 再 `rename`
- JSON 写前**可**递归按字母序排序键，便于 diff 与确定性测试（配置写路径常跳过，排序多用于 API payload 规范化）
- 多文件（如 auth.json + config.toml）：顺序写，后写失败则回滚先写成功的文件

**反例**
❌ 错误：`std::fs::write(settings_path, bytes)` 直写目标路径  
✅ 正确：`atomic_write(path, data)` 封装 tmp→rename

---
_最后更新：2026-06-28_
