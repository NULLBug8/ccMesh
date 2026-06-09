# Tauri 签名密钥与自动更新配置

> 更新日期：2026-06-09

---

## 一、工作原理

Tauri 的应用内自动更新（updater）依赖一对 **Ed25519 签名密钥**：

- **私钥**：构建时用于给更新包签名，必须保密，绝不入库。
- **公钥**：写入应用配置，运行时用于校验下载的更新包是否可信。

更新流程：

1. 构建时（`createUpdaterArtifacts: true`）为每个平台产出更新包及其 `.sig` 签名，并汇总生成 `latest.json`。
2. `latest.json` 上传到更新源（本项目用 GitHub Releases）。
3. 客户端启动后请求 `latest.json`，比对版本号；有新版则下载更新包，用内置公钥校验签名后安装。

**关键文件：**

| 文件 | 作用 |
|------|------|
| `~/.tauri/ccmesh_updater.key` | 私钥（本机生成，勿提交） |
| `~/.tauri/ccmesh_updater.key.pub` | 公钥（其内容已写入配置） |
| `src-tauri/tauri.conf.json` | `bundle.createUpdaterArtifacts` + `plugins.updater` |
| `.github/workflows/release.yml` | CI 构建签名并上传 `latest.json` |
| `.env.local` | 本地构建注入签名变量（已忽略） |

---

## 二、创建签名密钥

**命令：** `tauri signer generate`

```bash
# 写入仓库外目录，避免误提交；-p "" 设空密码，--ci 跳过交互提示
pnpm tauri signer generate --ci -p "" -w "$HOME/.tauri/ccmesh_updater.key" -f
```

| 参数 | 说明 |
|------|------|
| `-w, --write-keys <PATH>` | 私钥写入文件，同时生成同名 `.pub` 公钥 |
| `-p, --password <PWD>` | 私钥密码；`""` 表示无密码 |
| `--ci` | 跳过交互式输入（CI 或脚本场景） |
| `-f, --force` | 覆盖已存在的密钥文件 |

执行后输出：

```
Your keypair was generated successfully:
Private: C:\Users\Administrator\.tauri\ccmesh_updater.key (Keep it secret!)
Public:  C:\Users\Administrator\.tauri\ccmesh_updater.key.pub
```

> **注意：** 私钥与密码一旦丢失，将无法再签名出能被旧客户端校验的更新包，意味着已发布用户的自动更新链路断裂。务必备份私钥，且不要放进版本库。

---

## 三、tauri.conf.json 配置

### 3.1 开启更新产物

```json
"bundle": {
  "createUpdaterArtifacts": true
}
```

`true` 时 `tauri build` 才会产出 `.sig` 签名与 `latest.json`；此时构建**强制要求**提供签名私钥（见第四章），否则报错中断。

### 3.2 updater 插件配置

```json
"plugins": {
  "updater": {
    "endpoints": [
      "https://github.com/VkRainB/ccMesh/releases/latest/download/latest.json"
    ],
    "pubkey": "dW50cnVzdGVkIGNvbW1lbnQ6..."
  }
}
```

| 字段 | 说明 |
|------|------|
| `endpoints` | 更新源地址列表；支持 `{{target}}`/`{{arch}}`/`{{current_version}}` 占位 |
| `pubkey` | 公钥内容（`*.key.pub` 文件内容，直接粘贴） |

> **注意：** `endpoints` 已指向真实仓库 `VkRainB/ccMesh`。`/releases/latest/download/` 仅对**正式发布**（非草稿、非预发布）的 Release 生效。

---

## 四、本地构建签名

`createUpdaterArtifacts: true` 后，本地 `tauri build` 需提供以下环境变量：

| 变量 | 说明 |
|------|------|
| `TAURI_SIGNING_PRIVATE_KEY` | 私钥**内容**字符串 |
| `TAURI_SIGNING_PRIVATE_KEY_PATH` | 或私钥**文件路径**（与上者二选一） |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | 私钥密码（本项目为空字符串） |

### 4.1 推荐方案：`.env.local` + dotenv-cli

**安装依赖：**

```bash
pnpm add -D dotenv-cli
```

**`package.json` 脚本：**

```json
"scripts": {
  "tauri:build": "dotenv -e .env.local -- tauri build"
}
```

**`.env.local`（已被 `.gitignore` 忽略，勿提交）：**

```
TAURI_SIGNING_PRIVATE_KEY_PATH=C:/Users/Administrator/.tauri/ccmesh_updater.key
TAURI_SIGNING_PRIVATE_KEY_PASSWORD=
```

**构建：**

```bash
pnpm tauri:build
```

`dotenv-cli` 先把 `.env.local` 注入进程环境，再执行 `tauri build` 完成签名打包。

### 4.2 备选方案：临时设环境变量

```bash
# git-bash
export TAURI_SIGNING_PRIVATE_KEY="$(cat ~/.tauri/ccmesh_updater.key)"
export TAURI_SIGNING_PRIVATE_KEY_PASSWORD=""
pnpm tauri build
```

```powershell
# PowerShell
$env:TAURI_SIGNING_PRIVATE_KEY = Get-Content -Raw "$HOME\.tauri\ccmesh_updater.key"
$env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = ""
pnpm tauri build
```

> **注意：** `.env.local` 是 Vite 的约定文件，但 **Vite 不会把它喂给签名步骤**（Vite 仅向前端暴露 `VITE_` 前缀变量）。必须经 `dotenv-cli` 注入到进程环境，签名才生效。

---

## 五、CI 构建签名（GitHub Actions）

`.github/workflows/release.yml` 通过 GitHub Secrets 注入签名变量，与本地 `.env.local` 完全独立。

### 5.1 配置 Secrets

GitHub 仓库 → **Settings → Secrets and variables → Actions → New repository secret**：

| Secret | 值 | 必需 |
|--------|----|----|
| `TAURI_SIGNING_PRIVATE_KEY` | `ccmesh_updater.key` 文件完整内容 | 是 |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | 留空（本项目无密码） | 否 |

### 5.2 workflow 接线

```yaml
- uses: tauri-apps/tauri-action@v0
  env:
    TAURI_SIGNING_PRIVATE_KEY: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY }}
    TAURI_SIGNING_PRIVATE_KEY_PASSWORD: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY_PASSWORD }}
  with:
    tauriScript: pnpm tauri
    includeUpdaterJson: true   # 生成并上传 latest.json
```

> **注意：** CI 执行的是 `pnpm tauri build`（非 `pnpm tauri:build`），不读取 `.env.local`；`.env.local` 被 gitignore 忽略，也不会进入 CI 工作区。两套机制互不影响。

---

## 六、前端 / 插件接线（本项目现状）

| 位置 | 内容 |
|------|------|
| `Cargo.toml` | 依赖 `tauri-plugin-updater` |
| `src/lib.rs` | `.plugin(tauri_plugin_updater::Builder::new().build())` |
| `capabilities/default.json` | 权限 `updater:default` |
| `src/commands/update.rs` | `check_for_updates` / `download_and_install` / `get_update_settings` / `set_update_settings` / `skip_version` |

应用层调用上述命令即可触发「检查更新 → 下载 → 校验签名 → 安装」。

---

## 七、关闭 updater 的影响

把 `createUpdaterArtifacts` 设回 `false`：

- **好处**：本地 `tauri build` 不再需要签名私钥，构建无障碍；仍正常产出全部安装包。
- **代价**：不再生成 `.sig` 与 `latest.json`，应用内自动更新失效；`pubkey`/`endpoints` 变为惰性配置（留存无害）。

适用：尚未搭建发布源、仅本地自用或只靠 CI 出包的阶段。

---

## 八、完整流程总结

```
1. 生成密钥   tauri signer generate -w ~/.tauri/ccmesh_updater.key
                ├── 私钥 → 本地保管 / CI Secret
                └── 公钥 → tauri.conf.json plugins.updater.pubkey

2. 配置       createUpdaterArtifacts: true
              plugins.updater.endpoints → 真实 GitHub Releases latest.json

3. 构建签名   本地 pnpm tauri:build（dotenv 注入）
              CI   tauri-action（Secrets 注入）+ includeUpdaterJson

4. 发布       推送 v*.*.* tag → CI 出包 + latest.json → 草稿
              GitHub 上手动 Publish（latest/download 才生效）

5. 客户端     启动请求 latest.json → 比对版本 → 下载 → 公钥校验 → 安装
```

**上线前检查清单：**

- [x] `endpoints` 已把 `OWNER/REPO` 换成真实仓库（`VkRainB/ccMesh`）
- [ ] GitHub Secret `TAURI_SIGNING_PRIVATE_KEY` 已配置
- [ ] 私钥已备份（丢失不可恢复）
- [ ] Release 已正式 Publish（非草稿）
