## Tauri Linux WebKitGTK 点击无响应

> 一句话结论：Linux 上设 WebKit 环境变量，show 后 nudge 窗口

**你会遇到这个问题的场景**
Tauri 2 桌面应用在 Ubuntu 等 Linux 上窗口可见、渲染正常，但鼠标点击、输入无响应。常见于 `visible: false` 启动后再由前端调用 `show()` 的流程。

**为什么会出错**
WebKitGTK 在 DMABUF（Direct Memory Access Buffer，GPU 缓冲直通）渲染模式下，与部分驱动/合成器组合时输入区域协商失败。延迟 show 加剧该问题；仅改前端无法修复底层 WebView 输入绑定。`WEBKIT_DISABLE_COMPOSITING_MODE` 曾作备选，但与 Radix Portal 等叠加时可能仍致点击失效，不宜与 DMABUF 一并强制开启。

**正确做法**
- **主修复**（Linux 且用户未自行设置时）：启动前注入 `WEBKIT_DISABLE_DMABUF_RENDERER=1`
- **补救**：Rust 侧 `nudge_main_window`——focus + ±1px resize 触发 WM 重算 hit-test（弥补 compositing 相关输入问题）
- 所有 show 路径（含静默启动后 reveal）在 `show()` 后 invoke nudge
- 不要覆盖用户已在环境中设置的同名变量

**反例**
❌ 错误：仅在前端 CSS 调 z-index，问题依旧  
✅ 正确：Linux 专用 DMABUF env + show 后 nudge，Windows/macOS 不受影响

---
_最后更新：2026-06-28_
