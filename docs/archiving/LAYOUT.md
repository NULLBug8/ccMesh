# Layout 设计方案

> 基于 `Unified-Dark-Stripe-design-language` 设计语言  
> 支持两种导航形态：水平顶部导航（Horizontal）/ 垂直侧边栏（Vertical）  
> 支持垂直模式下的展开 / 折叠（Expanded / Collapsed）

---

## 一、核心概念

整个 App 由一个单一的 `AppLayout` 组件驱动，导航形态通过全局状态切换，**不存在两套 Layout 并存的情况**。

```
AppLayout
├── NavigationBar          ← 根据 navMode 渲染不同形态
│   ├── mode="horizontal"  → TopNav
│   └── mode="vertical"    → SideNav（expanded | collapsed）
└── <main>
    └── <Outlet />         ← 页面内容，永远不含导航
```

---

## 二、状态定义

```ts
type NavMode = 'horizontal' | 'vertical'
type SidebarState = 'expanded' | 'collapsed'  // 仅 vertical 模式有效

interface LayoutState {
  navMode: NavMode
  sidebarState: SidebarState
}
```

**持久化策略**：写入 `localStorage`，刷新后保持用户上次选择。

---

## 三、布局形态详解

### 3.1 水平模式（Horizontal）

```
┌─────────────────────────────────────────────────────┐  h: 56px
│  Logo │ 账号管理  API反代  流量日志  Token统计 … │ 工具栏 │  bg: #000
└─────────────────────────────────────────────────────┘
┌─────────────────────────────────────────────────────┐
│                                                     │
│                   <Outlet />                        │  flex: 1, overflow-y: auto
│                                                     │
└─────────────────────────────────────────────────────┘
```

**尺寸规范**

| 属性 | 值 |
|---|---|
| TopNav 高度 | `56px` |
| 背景色 | `surface` `#000` |
| 底部边框 | `1px solid edge` `#222` |
| Logo 区宽度 | `160px` |
| Nav item 间距 | `gap: 4px` |
| Nav item padding | `8px 12px`，`rounded-pill` |
| 激活态 | `bg: primary #22c55e`，`text: on-primary #000` |
| 悬停态 | `bg: surface-hover #181818` |
| 工具栏（右侧） | 图标按钮 `32×32px`，`gap: 8px` |

**CSS 骨架**

```css
.app-shell {
  display: flex;
  flex-direction: column;
  height: 100vh;
  background: var(--color-surface);       /* #000 */
}

.top-nav {
  display: flex;
  align-items: center;
  height: 56px;
  padding: 0 24px;
  background: var(--color-surface);
  border-bottom: 1px solid var(--color-edge);  /* #222 */
  flex-shrink: 0;
  gap: 4px;
}

.page-content {
  flex: 1;
  overflow-y: auto;
  padding: 32px;
}
```

---

### 3.2 垂直模式 — 展开（Vertical Expanded）

```
┌──────────┬───────────────────────────────────────────┐
│  Logo    │                                           │
│──────────│                                           │
│ ⚡ 账号  │                <Outlet />                 │
│ 🔗 API   │                                           │
│ 📋 日志  │                                           │
│ 📊 Token │                                           │
│ 👤 用户  │                                           │
│ 🌐 IP    │                                           │
│──────────│                                           │
│ ⚙ 设置  │                                           │
└──────────┴───────────────────────────────────────────┘
  w: 220px              flex: 1
```

**尺寸规范**

| 属性 | 值 |
|---|---|
| SideNav 宽度 | `220px` |
| 背景色 | `surface` `#000` |
| 右侧边框 | `1px solid edge` `#222` |
| Logo 区高度 | `56px`，与 TopNav 等高，保持视觉对齐 |
| Nav item 高度 | `40px` |
| Nav item padding | `0 12px` |
| Nav item 圆角 | `rounded.sm` `6px` |
| Nav item margin | `2px 8px`（上下 2px，左右 8px 内缩） |
| 图标尺寸 | `16px`，`gap: 10px` |
| 激活态 | `bg: primary-bg-subdued rgba(34,197,94,0.12)`，`text: primary-soft #4ade80`，`icon: primary` |
| 悬停态 | `bg: surface-hover #181818`，`text: ink-primary #ededef` |
| 分组标题 | `micro-cap` 10px/500，`ink-mute #8a8a97`，`margin: 16px 20px 4px` |
| 底部固定区 | 设置入口 + 折叠按钮，`border-top: 1px solid edge` |
| 折叠触发 | 右下角 `chevron-left` 按钮 |

**CSS 骨架**

```css
.app-shell.vertical {
  display: flex;
  flex-direction: row;
  height: 100vh;
}

.side-nav {
  width: 220px;
  display: flex;
  flex-direction: column;
  background: var(--color-surface);
  border-right: 1px solid var(--color-edge);
  flex-shrink: 0;
  transition: width 240ms cubic-bezier(0.4, 0, 0.2, 1);
}

.side-nav .nav-logo {
  height: 56px;
  display: flex;
  align-items: center;
  padding: 0 16px;
  border-bottom: 1px solid var(--color-edge-subtle);
  flex-shrink: 0;
}

.side-nav .nav-items {
  flex: 1;
  overflow-y: auto;
  padding: 8px 0;
}

.nav-item {
  display: flex;
  align-items: center;
  gap: 10px;
  height: 40px;
  padding: 0 12px;
  margin: 2px 8px;
  border-radius: 6px;
  font-size: 14px;
  color: var(--color-ink-secondary);
  cursor: pointer;
  transition: background 120ms, color 120ms;
}

.nav-item:hover {
  background: var(--color-surface-hover);
  color: var(--color-ink-primary);
}

.nav-item.active {
  background: var(--color-primary-bg-subdued);
  color: var(--color-primary-soft);
}

.side-nav .nav-footer {
  border-top: 1px solid var(--color-edge);
  padding: 8px 0;
  flex-shrink: 0;
}
```

---

### 3.3 垂直模式 — 折叠（Vertical Collapsed）

```
┌────┬──────────────────────────────────────────────────┐
│    │                                                  │
│ ⚡ │                                                  │
│ 🔗 │               <Outlet />                        │
│ 📋 │                                                  │
│ 📊 │                                                  │
│ 👤 │                                                  │
│    │                                                  │
│ ⚙ │                                                  │
└────┴──────────────────────────────────────────────────┘
 w:56px            flex: 1
```

**折叠行为规范**

| 属性 | 值 |
|---|---|
| 折叠宽度 | `56px` |
| 图标居中 | `justify-content: center`，隐藏文字 |
| 文字隐藏方式 | `opacity: 0` + `width: 0` + `overflow: hidden`（配合 transition） |
| Tooltip | 鼠标悬停图标时，右侧浮出 `surface-overlay` 背景的 label |
| Logo | 仅保留图标，隐藏文字 |
| 分组标题 | 折叠时完全隐藏（`display: none` 或 `opacity: 0`） |
| 展开按钮 | 底部 `chevron-right`，点击恢复 `expanded` |

**过渡动画**

```css
.side-nav {
  transition: width 240ms cubic-bezier(0.4, 0, 0.2, 1);
}

.side-nav.collapsed {
  width: 56px;
}

.side-nav.collapsed .nav-label {
  opacity: 0;
  width: 0;
  overflow: hidden;
  transition: opacity 160ms, width 240ms cubic-bezier(0.4, 0, 0.2, 1);
}

.side-nav .nav-label {
  opacity: 1;
  width: auto;
  white-space: nowrap;
  transition: opacity 200ms 40ms, width 240ms cubic-bezier(0.4, 0, 0.2, 1);
}
```

> **注意**：文字淡出比宽度收缩略早结束（`160ms` vs `240ms`），避免文字被截断时的生硬感。展开时文字延迟 `40ms` 出现，等宽度先打开再显示文字。

---

## 四、形态切换

### 4.1 切换触发点

| 触发位置 | 行为 |
|---|---|
| 顶部工具栏 — 布局切换按钮 | 在 `horizontal` / `vertical` 之间切换 |
| 侧边栏底部 — 折叠按钮 | 在 `expanded` / `collapsed` 之间切换（仅 vertical 模式） |

切换按钮使用 `grid-view`（水平）和 `layout-sidebar`（垂直）图标，当前激活态用 `primary` 色高亮。

### 4.2 切换逻辑伪代码

```jsx
// useLayoutStore.js（Zustand / Pinia 均可，以下为 Zustand 风格）

const useLayoutStore = create(
  persist(
    (set) => ({
      navMode: 'horizontal',       // 'horizontal' | 'vertical'
      sidebarState: 'expanded',    // 'expanded' | 'collapsed'

      setNavMode: (mode) => set({ navMode: mode }),

      toggleSidebar: () =>
        set((s) => ({
          sidebarState: s.sidebarState === 'expanded' ? 'collapsed' : 'expanded',
        })),
    }),
    { name: 'layout-prefs' }       // 持久化到 localStorage
  )
)
```

```jsx
// AppLayout.jsx

function AppLayout() {
  const { navMode, sidebarState, setNavMode, toggleSidebar } = useLayoutStore()

  return (
    <div className={`app-shell ${navMode}`}>

      {navMode === 'horizontal' && (
        <TopNav onSwitchLayout={() => setNavMode('vertical')} />
      )}

      {navMode === 'vertical' && (
        <SideNav
          state={sidebarState}
          onToggle={toggleSidebar}
          onSwitchLayout={() => setNavMode('horizontal')}
        />
      )}

      <main className="page-content">
        <Outlet />
      </main>

    </div>
  )
}
```

```jsx
// SideNav.jsx（简化）

function SideNav({ state, onToggle, onSwitchLayout }) {
  const collapsed = state === 'collapsed'

  return (
    <nav className={`side-nav ${collapsed ? 'collapsed' : ''}`}>

      <div className="nav-logo">
        <Logo iconOnly={collapsed} />
      </div>

      <div className="nav-items">
        {NAV_ITEMS.map(item => (
          <NavItem key={item.path} item={item} collapsed={collapsed} />
        ))}
      </div>

      <div className="nav-footer">
        {/* 布局模式切换 */}
        <NavFooterAction
          icon="layout-navbar"
          label="切换为顶部导航"
          collapsed={collapsed}
          onClick={onSwitchLayout}
        />
        {/* 折叠/展开 */}
        <NavFooterAction
          icon={collapsed ? 'chevron-right' : 'chevron-left'}
          label={collapsed ? '展开侧边栏' : '折叠侧边栏'}
          collapsed={collapsed}
          onClick={onToggle}
        />
      </div>

    </nav>
  )
}
```

---

## 五、Tooltip（折叠态专用）

折叠时所有 label 不可见，悬停图标需弹出 Tooltip 提示导航项名称。

```jsx
// 仅在 collapsed 时挂载 Tooltip
function NavItem({ item, collapsed }) {
  const content = (
    <div className={`nav-item ${isActive ? 'active' : ''}`}>
      <Icon name={item.icon} size={16} />
      {!collapsed && <span className="nav-label">{item.label}</span>}
    </div>
  )

  return collapsed
    ? <Tooltip content={item.label} placement="right">{content}</Tooltip>
    : content
}
```

**Tooltip 样式规范**

| 属性 | 值 |
|---|---|
| 背景 | `surface-overlay` `#1c1c1f` |
| 文字 | `ink-primary` `#ededef`，`body-md` 14px |
| 圆角 | `rounded.sm` `6px` |
| padding | `6px 10px` |
| 出现延迟 | `300ms`（防误触） |
| 动画 | `opacity 0→1`，`translateX(-4px→0)`，`120ms ease` |
| 阴影 | `shadow-level-2` |

---

## 六、响应式行为

| 断点 | 行为 |
|---|---|
| `≥ 1440px` | 任意模式均正常显示 |
| `1024–1440px` | vertical 默认 `expanded`，horizontal 正常 |
| `768–1024px` | vertical 自动切换为 `collapsed`；horizontal 导航项可省略文字只保留图标 |
| `< 768px` | 强制 horizontal 模式；顶部导航收入 hamburger 菜单 |

```js
// 响应式自动折叠（在 store 初始化时执行）
const mql = window.matchMedia('(max-width: 1024px)')
mql.addEventListener('change', (e) => {
  if (useLayoutStore.getState().navMode === 'vertical' && e.matches) {
    useLayoutStore.getState().set({ sidebarState: 'collapsed' })
  }
})
```

---

## 七、设计 Token 对照

以下为本方案直接使用的 token，均来自 `DESIGN.md`：

| Token | 值 | 用途 |
|---|---|---|
| `surface` | `#000` | 导航背景 |
| `surface-hover` | `#181818` | nav item 悬停 |
| `surface-overlay` | `#1c1c1f` | Tooltip 背景 |
| `edge` | `#222` | 导航分隔线 |
| `edge-subtle` | `#1a1a1a` | Logo 区底部线 |
| `primary` | `#22c55e` | 激活指示条 |
| `primary-soft` | `#4ade80` | 激活文字色 |
| `primary-bg-subdued` | `rgba(34,197,94,0.12)` | 激活背景 |
| `ink-primary` | `#ededef` | 主文字 |
| `ink-secondary` | `#a1a1aa` | 默认导航文字 |
| `ink-mute` | `#8a8a97` | 分组标题 |
| `rounded.sm` | `6px` | nav item 圆角 |
| `rounded.pill` | `9999px` | horizontal 激活态 |
| `shadow-level-2` | `0 4px 12px rgba(0,0,0,.4)` | Tooltip 阴影 |

---

## 八、文件结构建议

```
src/
├── layouts/
│   ├── AppLayout.jsx          ← 唯一入口，切换逻辑
│   ├── TopNav.jsx             ← 水平导航
│   ├── SideNav.jsx            ← 垂直导航（含折叠态）
│   ├── NavItem.jsx            ← 单个导航项（含 Tooltip）
│   └── layout.css             ← 布局 CSS 变量与过渡
├── stores/
│   └── useLayoutStore.js      ← navMode + sidebarState + persist
└── router.jsx                 ← <Route element={<AppLayout />}>包裹所有页面
```

---

> **核心原则**：一个 `AppLayout`，一套状态，两种形态。页面组件永远只渲染内容，导航是 Layout 的职责，不下放到页面层。
