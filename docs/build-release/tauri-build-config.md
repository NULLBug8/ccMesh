# Tauri 构建配置详解

> 更新日期：2026-05-27

---

## 一、build.rs — Rust 构建脚本

**文件位置：** `src-tauri/build.rs`

**作用：** Cargo 在编译前自动执行此脚本，用于生成代码或执行预编译任务。

**默认内容：**

```rust
fn main() {
    tauri_build::build()
}
```

`tauri_build::build()` 会自动完成：
- 生成 Windows 资源文件（.res）
- 处理 `tauri.conf.json` 配置
- 生成能力（capabilities）相关的代码

**何时需要修改：**

通常不需要修改。除非需要：
- 链接额外的系统库
- 生成自定义代码
- 执行编译前的文件处理

---

## 二、tauri.conf.json — 应用配置

**文件位置：** `src-tauri/tauri.conf.json`

这是 Tauri 应用的核心配置文件，控制应用的构建、窗口、安全等行为。

### 2.1 完整结构

```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "ccMesh",
  "version": "0.1.0",
  "identifier": "com.ex.xyz",
  "build": { ... },
  "app": { ... },
  "bundle": { ... }
}
```

### 2.2 顶层字段

| 字段 | 类型 | 说明 |
|------|------|------|
| `$schema` | string | JSON Schema 地址，提供编辑器自动补全 |
| `productName` | string | 应用名称，用于显示和打包 |
| `version` | string | 应用版本号 |
| `identifier` | string | 应用唯一标识符（反向域名格式），用于打包和系统集成 |

---

### 2.3 build — 构建配置

控制开发和构建时的行为。

```json
"build": {
  "beforeDevCommand": "pnpm dev",
  "devUrl": "http://localhost:1420",
  "beforeBuildCommand": "pnpm build",
  "frontendDist": "../dist"
}
```

| 字段 | 说明 |
|------|------|
| `beforeDevCommand` | 执行 `tauri dev` 前运行的命令，通常启动前端开发服务器 |
| `devUrl` | 开发模式下前端服务器的地址，Tauri 窗口会加载此 URL |
| `beforeBuildCommand` | 执行 `tauri build` 前运行的命令，通常构建前端 |
| `frontendDist` | 前端构建产物目录（相对于 `src-tauri`），打包时会嵌入应用 |

---

### 2.4 app — 应用配置

控制窗口、安全等运行时行为。

```json
"app": {
  "windows": [ ... ],
  "security": { ... }
}
```

#### 2.4.1 windows — 窗口配置

```json
"windows": [
  {
    "title": "ccMesh",
    "width": 800,
    "height": 600
  }
]
```

常用窗口属性：

| 属性 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `title` | string | — | 窗口标题 |
| `width` | number | 800 | 窗口宽度（px） |
| `height` | number | 600 | 窗口高度（px） |
| `minWidth` | number | — | 最小宽度 |
| `minHeight` | number | — | 最小高度 |
| `maxWidth` | number | — | 最大宽度 |
| `maxHeight` | number | — | 最大高度 |
| `resizable` | bool | true | 是否可调整大小 |
| `fullscreen` | bool | false | 是否全屏 |
| `center` | bool | false | 是否居中显示 |
| `decorations` | bool | true | 是否显示标题栏和边框 |
| `transparent` | bool | false | 背景是否透明 |
| `alwaysOnTop` | bool | false | 是否置顶 |
| `visible` | bool | true | 是否默认可见 |
| `url` | string | "index" | 加载的页面路径 |

#### 2.4.2 security — 安全配置

```json
"security": {
  "csp": null
}
```

| 属性 | 说明 |
|------|------|
| `csp` | Content Security Policy（内容安全策略），`null` 表示不限制 |

> **注意：** 生产环境建议设置合理的 CSP，限制资源加载来源。

---

### 2.5 bundle — 打包配置

控制应用打包行为。

```json
"bundle": {
  "active": true,
  "targets": "all",
  "icon": [
    "icons/32x32.png",
    "icons/128x128.png",
    "icons/128x128@2x.png",
    "icons/icon.icns",
    "icons/icon.ico"
  ]
}
```

| 属性 | 类型 | 说明 |
|------|------|------|
| `active` | bool | 是否启用打包 |
| `targets` | string/array | 打包目标格式：`"all"`、`"msi"`、`"nsis"`、`"app"`、`"dmg"` 等 |
| `icon` | array | 应用图标路径列表 |
| `productName` | string | 打包后的文件名（可覆盖顶层 productName） |
| `category` | string | 应用分类（Utility、DeveloperTool 等） |
| `shortDescription` | string | 简短描述 |
| `longDescription` | string | 详细描述 |

**Windows 打包格式：**
- `msi` — Windows Installer 包
- `nsis` — NSIS 安装程序（推荐，更灵活）

---

## 三、capabilities — 权限控制

**目录位置：** `src-tauri/capabilities/`

Tauri v2 使用 capabilities 机制控制前端可访问的 API 权限。

**默认配置：** `capabilities/default.json`

```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "default",
  "description": "Capability for the main window",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "opener:default"
  ]
}
```

| 字段 | 说明 |
|------|------|
| `identifier` | 能力标识符 |
| `description` | 描述信息 |
| `windows` | 应用此权限的窗口列表，`["main"]` 表示主窗口 |
| `permissions` | 允许的权限列表 |

**添加插件权限：**

安装插件后需要在 `permissions` 中添加对应权限：

```json
"permissions": [
  "core:default",
  "opener:default",
  "clipboard-manager:allow-write-text",
  "clipboard-manager:allow-read-text"
]
```

**权限格式：** `<plugin>:<permission>` 或 `<plugin>:allow-<action>`

---

## 四、文件关系总结

```
src-tauri/
├── build.rs              # 编译前自动执行，调用 tauri_build
├── tauri.conf.json       # 应用配置（窗口、构建、安全、打包）
├── capabilities/         # 前端 API 权限声明
│   └── default.json      # 默认权限配置
└── src/
    ├── main.rs           # Rust 入口
    └── lib.rs            # Tauri 命令定义
```

**执行流程：**

1. `tauri dev` → 执行 `beforeDevCommand`（启动前端）→ 编译 Rust（`build.rs`）→ 启动窗口加载 `devUrl`
2. `tauri build` → 执行 `beforeBuildCommand`（构建前端）→ 编译 Rust → 打包应用
