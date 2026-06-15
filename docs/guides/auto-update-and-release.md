# 自动更新与发布

> 应用内更新（updater）与 GitHub Actions 发布流程说明。

---

## 自动更新（updater）

应用内更新已接入，更新源指向 GitHub Releases 的 `latest.json`。

> 更新源：`src-tauri/tauri.conf.json` 中 `plugins.updater.endpoints` 已指向
> `https://github.com/VkRainB/ccMesh/releases/latest/download/latest.json`。

工作机制：CI 构建时生成并上传 `latest.json` 与各平台更新包签名；客户端启动后比对版本拉取更新。
注意 `latest/download` 仅对**已正式发布**（非草稿、非预发布）的 Release 生效，故需在 GitHub 上手动 Publish 发布草稿后更新才会生效。

### 本地构建签名

本项目已开启 `createUpdaterArtifacts`（自动更新签名产物）。本地执行 `pnpm tauri build` 时
必须设置签名私钥环境变量，否则构建会报错：

```bash
export TAURI_SIGNING_PRIVATE_KEY="$(cat ~/.tauri/ccmesh_updater.key)"
export TAURI_SIGNING_PRIVATE_KEY_PASSWORD=""   # 本项目密钥无密码
```

CI 构建通过仓库 Secret 注入，无需本地操作（见下文「发布」）。

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
