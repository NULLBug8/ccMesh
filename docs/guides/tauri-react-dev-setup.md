# Tauri + React 开发环境搭建指南

> 基于 Tauri v2.11.2 + React + TypeScript + Vite
> 适用系统：Windows 10/11
> 更新日期：2026-05-27

---

## 版本信息

| 组件 | 版本 | 说明 |
|------|------|------|
| Tauri | v2.11.2 | 2026-05-16 发布，当前最新稳定版 |
| @tauri-apps/cli | 2.11.2 | npm CLI 工具 |
| @tauri-apps/api | 2.11.0 | 前端 JS API 绑定 |
| Rust 工具链 | stable-msvc | 需保持最新 stable |
| Node.js | >= 18 LTS | 推荐 22.x LTS |

---

## 一、环境依赖总览

| 依赖 | 用途 | 是否必须 |
|------|------|---------|
| Microsoft C++ Build Tools | Rust 编译所需的 C++ 工具链 | 是 |
| WebView2 | Tauri 渲染 Web 内容的运行时 | 是 |
| Rust (rustup) | Tauri 后端语言 | 是 |
| Node.js (LTS) | 前端包管理与构建工具 | 是 (使用 JS 框架时) |

---

## 二、安装 Microsoft C++ Build Tools

Tauri 的 Rust 后端需要 MSVC (Microsoft Visual C++) 编译工具链。

**步骤：**

1. 下载 [Microsoft C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/)
2. 运行安装程序
3. 在工作负载页面勾选 **"Desktop development with C++"**
4. 点击安装，等待完成

**验证：**

安装完成后，打开新的 PowerShell 终端，执行：

```powershell
cl
```

如果输出 Microsoft (R) C/C++ 优化编译器 的版本信息，说明安装成功。

---

## 三、确认 WebView2

Tauri 使用 Microsoft Edge WebView2 在 Windows 上渲染 Web 内容。

> Windows 10 (1803 及以上) 和 Windows 11 已预装 WebView2，通常无需额外操作。

**验证是否已安装：**

```powershell
Get-ItemProperty -Path "HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BEE-13A6279B2E44}" -ErrorAction SilentlyContinue | Select-Object pv
```

如果返回版本号，说明已安装。否则前往 [WebView2 下载页](https://developer.microsoft.com/en-us/microsoft-edge/webview2/) 下载 "Evergreen Bootstrapper" 并安装。

---

## 四、安装 Rust

Tauri 后端使用 Rust 编写。

**安装方式一：使用 winget（推荐）**

```powershell
winget install --id Rustlang.Rustup
```

**安装方式二：手动安装**

前往 https://rustup.rs/ 下载安装程序并运行。

**关键步骤 - 设置 MSVC 工具链为默认：**

安装过程中，确保选择 MSVC 工具链。如果已安装 Rust，手动确认：

```powershell
rustup default stable-msvc
```

**验证安装（重启终端后）：**

```powershell
rustc --version
# 示例输出：rustc 1.87.0 (17067e9ac 2025-05-09)

cargo --version
# 示例输出：cargo 1.87.0 (a511c3b42 2025-05-06)
```

---

## 四（补充）、Cargo 国内镜像源配置

国内访问 crates.io 较慢，建议配置镜像源加速依赖下载。

**配置文件位置：** `C:\Users\<用户名>\.cargo\config.toml`

**推荐配置（稀疏索引协议，速度最快）：**

```toml
[source.crates-io]
replace-with = 'tuna'

# 清华大学（推荐）
[source.tuna]
registry = "sparse+https://mirrors.tuna.tsinghua.edu.cn/crates.io-index/"
```

**其他可用镜像源：**

| 源名称 | 机构 | registry 值 |
|--------|------|-------------|
| tuna | 清华大学 | `sparse+https://mirrors.tuna.tsinghua.edu.cn/crates.io-index/` |
| ustc | 中国科学技术大学 | `sparse+https://mirrors.ustc.edu.cn/crates.io-index/` |
| sjtu | 上海交通大学 | `sparse+https://mirrors.sjtug.sjtu.edu.cn/crates.io-index/` |

> **注意：** `sparse+` 前缀表示使用稀疏索引协议，比传统 git 协议快很多。首次编译时按需下载，无需 clone 整个索引仓库。

---

## 五、安装 Node.js

前端 React 项目需要 Node.js 环境。

**安装方式：**

1. 前往 https://nodejs.org/ 下载 **LTS** 版本并安装
2. 或使用 winget：

```powershell
winget install OpenJS.NodeJS.LTS
```

**验证安装（重启终端后）：**

```powershell
node -v
# 示例输出：v22.15.0

npm -v
# 示例输出：10.9.2
```

> 如果你想使用 pnpm 或 yarn 作为包管理器，可运行 `corepack enable` 启用。

---

## 六、创建 Tauri + React 项目

环境就绪后，有两种方式创建项目。

### 方式一：使用 create-tauri-app（推荐，一步到位）

```powershell
npm create tauri-app@latest
```

交互式向导会依次询问：

```
? Project name › my-tauri-app
? Identifier (com.my-tauri-app.app) › com.example.myapp
? Choose which language to use for your frontend › TypeScript / JavaScript
? Choose your package manager › npm
? Choose your UI template › React
? Choose your UI flavor › TypeScript
```

选择完毕后进入项目目录：

```powershell
cd my-tauri-app
npm install
npm run tauri dev
```

首次运行会编译 Rust 依赖，耗时较长（约 1-5 分钟），后续热更新会快很多。

编译完成后会弹出一个桌面窗口，展示你的 React 应用。

### 方式二：手动搭建（已有 React 项目或需要更多控制）

**第一步：创建 Vite + React 前端项目**

```powershell
mkdir my-tauri-app
cd my-tauri-app
npm create vite@latest . -- --template react-ts
npm install
```

**第二步：安装 Tauri CLI**

```powershell
npm install -D @tauri-apps/cli@^2.11.2
```

**第三步：初始化 Tauri 后端**

```powershell
npx tauri init
```

按提示输入：

```
? What is your app name? › my-tauri-app
? What should the window title be? › My Tauri App
? Where are your web assets located? › ../dist
? What is the url of your dev server? › http://localhost:5173
? What is your frontend dev command? › npm run dev
? What is your frontend build command? › npm run build
```

**第四步：安装 Tauri 前端 API（可选但推荐）**

```powershell
npm install @tauri-apps/api@^2.11.0
```

**第五步：启动开发**

```powershell
npx tauri dev
```

---

## 七、项目结构说明

```
my-tauri-app/
├── src/                    # React 前端源码
│   ├── App.tsx
│   ├── main.tsx
│   └── ...
├── src-tauri/              # Tauri (Rust) 后端
│   ├── Cargo.toml          # Rust 依赖配置
│   ├── tauri.conf.json     # Tauri 应用配置
│   ├── src/
│   │   └── main.rs         # Rust 入口
│   └── icons/              # 应用图标
├── index.html              # 前端入口 HTML
├── package.json            # Node.js 依赖配置
├── tsconfig.json           # TypeScript 配置
└── vite.config.ts          # Vite 构建配置
```

---

## 八、常用命令

| 命令 | 说明 |
|------|------|
| `npm run tauri dev` | 启动开发模式（前端热更新 + Rust 编译） |
| `npm run tauri build` | 构建生产版本（生成安装包） |
| `npm run tauri icon <path>` | 根据图片自动生成各尺寸应用图标 |
| `npm run tauri info` | 查看环境与依赖信息 |

---

## 九、常见问题

### 1. 首次 `tauri dev` 编译很慢

正常现象。Rust 首次编译需要下载并编译所有依赖（约 200+ crate）。后续编译会利用缓存，速度快很多。

### 2. 报错 `link.exe not found`

说明 MSVC 工具链未正确安装或未加入 PATH。解决方案：

- 确认已安装 "Desktop development with C++" 工作负载
- 重启终端或重启电脑
- 运行 `rustup default stable-msvc`

### 3. 报错 `WebView2Loader.dll not found`

WebView2 未安装或版本过旧。重新安装 WebView2 Evergreen Bootstrapper。

### 4. MSI 构建失败 `failed to run light.exe`

需要启用 VBSCRIPT Windows 可选功能：

1. 打开 设置 -> 应用 -> 可选功能 -> 更多 Windows 功能
2. 找到 VBSCRIPT，确保勾选
3. 重启电脑

### 5. 前端修改后窗口未刷新

确认 Vite 开发服务器正在运行。`tauri dev` 会自动启动前端 dev server 并在 Rust 编译完成后加载。

---

## 十、参考资料

- [Tauri v2 官方文档 - 环境准备](https://v2.tauri.app/start/prerequisites/)
- [Tauri v2 官方文档 - 创建项目](https://v2.tauri.app/start/create-project/)
- [Tauri v2 官方文档 - 项目结构](https://v2.tauri.app/start/project-structure/)
- [Tauri GitHub Releases](https://github.com/tauri-apps/tauri/releases)
- [Rust 官方安装](https://www.rust-lang.org/tools/install)
- [Node.js 官网](https://nodejs.org/)
