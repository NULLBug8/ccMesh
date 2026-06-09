# ccMesh

基于 Tauri 2 + React + TypeScript 的跨平台桌面代理网关，支持 Windows / macOS / Linux。

技术栈：Tauri 2、React 19、TypeScript、Vite、SQLite（bundled）、axum/reqwest（rustls）。

---

## 开发

环境要求：Node.js LTS、pnpm 10+、Rust stable。

```bash
pnpm install
pnpm tauri dev      # 启动桌面开发环境
pnpm test           # 运行前端单测
```

---

## 构建

```bash
pnpm tauri build
```

各平台构建依赖：

- **Windows**：MSVC 工具链（Visual Studio Build Tools）+ WebView2（Win10/11 一般已内置）。
- **macOS**：Xcode Command Line Tools。产物为通用二进制（同时支持 Intel 与 Apple Silicon）。
- **Linux**（Ubuntu/Debian 系，构建机需安装）：

  ```bash
  sudo apt-get install -y \
    libwebkit2gtk-4.1-dev \
    libayatana-appindicator3-dev \
    librsvg2-dev \
    patchelf
  ```

> 注意：本项目已开启 `createUpdaterArtifacts`（自动更新签名产物）。本地执行 `pnpm tauri build` 时
> 必须设置签名私钥环境变量，否则构建会报错：
>
> ```bash
> export TAURI_SIGNING_PRIVATE_KEY="$(cat ~/.tauri/ccmesh_updater.key)"
> export TAURI_SIGNING_PRIVATE_KEY_PASSWORD=""   # 本项目密钥无密码
> ```
>
> CI 构建通过仓库 Secret 注入，无需本地操作（见下文「发布」）。

---

## 安装说明

### Windows

下载 `*-setup.exe`（NSIS）或 `*.msi`，双击安装即可。

### macOS（重要：当前为未签名版本）

由于暂未配置 Apple 开发者签名与公证证书，下载的 `.dmg` / `.app` **未签名**，
首次打开会被 Gatekeeper 拦截（提示「已损坏」或「无法验证开发者」）。请用以下任一方法放行：

方法一（推荐，右键打开）：

1. 将 ccMesh.app 拖入「应用程序」。
2. 在「应用程序」中 **右键点击** ccMesh → 选择「打开」→ 在弹窗再次点「打开」。
   之后即可正常双击启动。

方法二（终端移除隔离属性）：

```bash
# 若提示「已损坏，无法打开」，执行：
xattr -dr com.apple.quarantine /Applications/ccMesh.app
```

> 正式签名/公证接入后此步骤将不再需要。

### Linux

提供三种包，按发行版选择：

- **AppImage（推荐，免安装、自带依赖）**：

  ```bash
  chmod +x ccMesh_*.AppImage
  ./ccMesh_*.AppImage
  ```

- **deb（Debian/Ubuntu 系）**：

  ```bash
  sudo apt install ./ccMesh_*_amd64.deb
  ```

- **rpm（Fedora/RHEL 系）**：

  ```bash
  sudo dnf install ./ccMesh-*.x86_64.rpm
  ```

> **系统托盘说明**：本应用使用系统托盘（AppIndicator）。在部分桌面环境（尤其原生 GNOME）下，
> 托盘图标默认不显示，需安装并启用 AppIndicator 扩展：
> - GNOME：安装 [AppIndicator and KStatusNotifierItem Support](https://extensions.gnome.org/extension/615/appindicator-support/) 扩展。
> - 运行时依赖 `libayatana-appindicator3-1`（deb）/ `libayatana-appindicator-gtk3`（rpm），包管理器会自动安装。

---

## 自动更新（updater）

应用内更新已接入，更新源指向 GitHub Releases 的 `latest.json`。

> **上线前必改**：`src-tauri/tauri.conf.json` 中 `plugins.updater.endpoints` 当前为占位符
> `https://github.com/OWNER/REPO/releases/latest/download/latest.json`，
> 请将 `OWNER/REPO` 替换为真实的 GitHub 仓库路径。

工作机制：CI 构建时生成并上传 `latest.json` 与各平台更新包签名；客户端启动后比对版本拉取更新。
注意 `latest/download` 仅对**已正式发布**（非草稿、非预发布）的 Release 生效，故需在 GitHub 上手动 Publish 发布草稿后更新才会生效。

---

## 发布（GitHub Actions）

发布流程见 `.github/workflows/release.yml`：

- 触发：推送 `v*.*.*` 形式的 tag（如 `v0.1.0`）自动三平台构建并创建 **Release 草稿**；
  也可在 Actions 页面手动 `workflow_dispatch` 仅验证构建。
- 平台矩阵：macOS 通用二进制（dmg）、Linux（deb/rpm/AppImage）、Windows（msi/nsis）。
- 构建完成后在 GitHub Releases 审核草稿并手动 Publish。

> 仓库托管在 Gitee，但 release workflow 为 GitHub Actions，仅在 GitHub 上运行。
> 需将代码推送/镜像到 GitHub 才能触发构建。

### 需要在 GitHub 仓库配置的 Secrets

| Secret | 用途 | 是否必需 |
| --- | --- | --- |
| `TAURI_SIGNING_PRIVATE_KEY` | updater 更新包签名私钥（`~/.tauri/ccmesh_updater.key` 文件内容） | 必需（已开启 updater） |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | 私钥密码（本项目为空，可不配或留空） | 可选 |
| `APPLE_CERTIFICATE` 等 `APPLE_*` | macOS 签名与公证 | 可选（暂无证书，缺失则产出未签名包） |

> updater 私钥已生成在本机 `~/.tauri/ccmesh_updater.key`（**切勿提交到仓库**），
> 其公钥已写入 `tauri.conf.json` 的 `plugins.updater.pubkey`。
> 将该私钥文件内容粘贴为 GitHub Secret `TAURI_SIGNING_PRIVATE_KEY` 即可。

---

更多跨平台兼容性细节见 [`docs/跨平台兼容性分析报告.md`](docs/跨平台兼容性分析报告.md)。
