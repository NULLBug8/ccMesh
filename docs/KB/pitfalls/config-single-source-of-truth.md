## 配置存储单一真相源防分叉

> 一句话结论：同一语义只用一个配置键，读写路径必须共用解析函数

**你会遇到这个问题的场景**
应用有 UI 设置页、后台服务、默认模板多处读取「代理端口」「功能开关」等。历史迭代引入 `port` 与 `proxy_port` 两个键，或 UI 写 A 键、daemon 读 B 键。

**为什么会出错**
写入路径只更新新键，读取路径仍读旧键 → UI 显示已改、服务仍监听默认端口。`build_status` 与 `start()` 若各读不同键，会出现「状态显示已启动、实际未监听」的分叉。

**正确做法**
- 选定单一 canonical 键名，废弃别名并在迁移中合并
- 抽取 `read_port()` 等 canonical 读取函数；其余配置经单一 `get_config` 入口
- 配置变更后 emit 领域事件并 invalidate queryKey：端口变更 emit `proxy-status-changed`；其它键依赖 mutation `onSuccess` invalidate——create/delete 等若未 emit，须在 mutation 侧显式 invalidate
- grep 全仓旧键名，确保无遗漏读取点

**反例**
❌ 错误：设置页写 `port`，proxy 读 `proxy_port.unwrap_or(3000)`  
✅ 正确：全仓 `read_port(config)` 只读 `port`

---
_最后更新：2026-06-28_
