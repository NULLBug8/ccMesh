# Tauri 依赖安装指南

> 更新日期：2026-05-27

---

## 一、两种依赖类型

| 类型 | 安装位置 | 安装命令 | 用途 |
|------|----------|----------|------|
| Rust 依赖 | `src-tauri/Cargo.toml` | `cargo add` | Rust 后端功能库 |
| Tauri 插件 | `Cargo.toml` + 前端 | `tauri add` | Tauri 官方插件（自动配置权限和前端 API） |
| 前端依赖 | `package.json` | `pnpm add` / `npm install` | 前端 JS/TS 库 |

---

## 二、安装 Tauri 插件（推荐方式）

Tauri 官方插件使用 `tauri add` 命令，会自动完成：
- 安装 Rust crate
- 安装前端 JS 绑定（`@tauri-apps/plugin-*`）
- 配置权限声明（capabilities）

**命令格式：**

```bash
pnpm tauri add <plugin-name>
```

**示例 — 安装剪贴板插件：**

```bash
pnpm tauri add clipboard-manager
```

安装完成后需要重启 `tauri dev`。

**前端使用：**

```typescript
import { writeText, readText } from '@tauri-apps/plugin-clipboard-manager';

// 写入剪贴板
await writeText('Hello Tauri');

// 读取剪贴板
const content = await readText();
```

**常用 Tauri 插件：**

| 插件名 | 功能 |
|--------|------|
| `clipboard-manager` | 剪贴板读写 |
| `dialog` | 文件选择、确认框、消息框 |
| `fs` | 文件系统读写 |
| `shell` | 执行系统命令 |
| `http` | HTTP 请求 |
| `notification` | 系统通知 |
| `global-shortcut` | 全局快捷键 |
| `autostart` | 开机自启 |

---

## 三、安装普通 Rust 依赖

非 Tauri 插件的 Rust 库，在 `src-tauri` 目录下用 `cargo add` 安装。

**命令格式：**

```bash
cd src-tauri
cargo add <crate-name>
```

**示例 — 安装 HTTP 客户端：**

```bash
cd src-tauri
cargo add reqwest
```

**带特性安装：**

```bash
cargo add tokio --features full
```

**指定版本：**

```bash
cargo add serde@1.0
```

---

## 四、安装前端依赖

前端 JS/TS 库在项目根目录用包管理器安装。

```bash
pnpm add <package-name>

# 示例
pnpm add axios
pnpm add zustand
```

---

## 五、查看已安装依赖

```bash
# 查看 Rust 依赖
cd src-tauri
cargo tree

# 查看前端依赖
pnpm list
```
