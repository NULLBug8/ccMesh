## React Query 跨页不同步：三因叠加诊断法

> 一句话结论：查 queryKey 不一致、后端未 emit、staleTime 过长

**你会遇到这个问题的场景**
Tauri/Electron 多页应用用 React Query（服务端状态缓存库）拉列表与健康状态。在 A 页修改端点，B 页（仪表盘、表单）仍显示旧数据，除非硬刷新。

**为什么会出错**
三类问题常叠加：①  mutation 后 `invalidateQueries` 的 queryKey 与 B 页 `useQuery` 的 key 不一致（如 `['endpoints']` vs `['health']`）；② 后端命令改了内存态但未写库、也未 emit 事件，前端无 invalidate 触发点；③ 全局 `staleTime: 60_000` 导致组件 remount 仍认为数据 fresh，不 refetch。

**正确做法**
- 梳理全 app 的 queryKey 命名表；相关视图共用 key 前缀或统一 invalidate 多个 key
- 后端 mutating 命令完成后 emit 领域事件（如 `endpoints-changed`）；**注意** create/delete 等命令若未 emit，mutation 须自行 invalidate
- 前端 hook 订阅事件并 invalidate 多 key；hook 须在相关页挂载，否则事件到达也无 invalidate
- 实时性高的列表缩短 staleTime 或 `refetchOnMount: 'always'`
- 诊断顺序：DevTools 看 key → 看 mutation 是否 invalidate → 看事件是否到达

**反例**
❌ 错误：toggle 只 invalidate `['endpoints']`，仪表盘读 `['endpointHealth']`  
✅ 正确：共享 `useEndpointHealth`，事件驱动 invalidate `['endpoints'], ['endpointHealth']`

---
_最后更新：2026-06-28_
