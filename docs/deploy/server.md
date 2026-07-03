# ccMesh 服务器部署教程

本文档面向只部署 Web 控制台 + 后端代理服务的场景。部署后访问同一个端口即可使用网页端，也可以把该端口作为 OpenAI / Claude / Codex 客户端的 API Base URL。

## 1. 部署结构

ccMesh 现在是一个 Rust HTTP 服务，启动后同时提供：

- Web 控制台：`http://服务器IP:端口/`
- 管理接口：`/__admin/api/invoke`
- 代理接口：`/v1/responses`、`/v1/chat/completions`、`/v1/messages` 等
- 健康检查：`/health`

默认建议端口：`3999`。

## 2. 环境要求

### Linux 服务器

推荐 Ubuntu 22.04 / Debian 12 及以上。

需要安装：

```bash
sudo apt update
sudo apt install -y curl git build-essential pkg-config libssl-dev
```

安装 Node.js 与 pnpm：

```bash
curl -fsSL https://deb.nodesource.com/setup_22.x | sudo -E bash -
sudo apt install -y nodejs
corepack enable
corepack prepare pnpm@latest --activate
```

安装 Rust：

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
rustup default stable
```

### Windows 服务器

需要：

- Node.js 22+
- pnpm
- Rust stable MSVC toolchain
- Git

本项目本地开发也可以直接在 Windows 上运行。

## 3. 拉取代码

```bash
git clone https://github.com/NULLBug8/ccMesh.git
cd ccMesh
```

如果你是从当前仓库重新部署第一版，只需要拉取 `main` 分支即可。

## 4. 构建

```bash
pnpm install
pnpm build
pnpm server:build
```

构建产物：

- 前端静态资源：`dist/`
- 后端二进制：`src-tauri/target/release/ccmesh`
  - Windows 为 `src-tauri/target/release/ccmesh.exe`

注意：后端启动时会从当前工作目录或资源目录查找 `dist/`，所以生产部署时建议在项目根目录运行二进制，或把 `dist/` 与二进制按服务可识别的目录结构一起保留。

## 5. 配置数据目录和端口

ccMesh 使用 SQLite 保存端点、设置、日志和统计数据。

推荐生产数据目录：

```bash
/var/lib/ccmesh
```

环境变量：

| 变量 | 作用 | 示例 |
| --- | --- | --- |
| `CCMESH_PORT` | Web + 代理监听端口 | `3999` |
| `CCMESH_DATA_DIR` | 数据目录 | `/var/lib/ccmesh` |
| `CCMESH_ADMIN_TOKEN` | 管理后台和代理接口访问令牌，公网部署必须配置 | `一段足够长的随机字符串` |
| `RUST_LOG` | 日志级别 | `info` |

安全规则：

- 设置 `CCMESH_ADMIN_TOKEN` 后，Web 控制台、管理接口、静态资源、`/v1/*` 代理接口、`/health` 和 `/stats` 都需要认证。
- 未设置 `CCMESH_ADMIN_TOKEN` 时，服务会进入锁定状态，不展示控制台和任何业务数据，避免公网误暴露。
- 登录网页后后端会写入 HttpOnly Cookie；API 客户端需要把该 token 作为 `Authorization: Bearer ...` 传入。

Linux 示例：

```bash
export CCMESH_PORT=3999
export CCMESH_DATA_DIR=/var/lib/ccmesh
export CCMESH_ADMIN_TOKEN='替换为足够长的随机字符串'
export RUST_LOG=info
./src-tauri/target/release/ccmesh
```

Windows PowerShell 示例：

```powershell
$env:CCMESH_PORT="3999"
$env:CCMESH_DATA_DIR="D:\ccmesh-data"
$env:CCMESH_ADMIN_TOKEN="替换为足够长的随机字符串"
$env:RUST_LOG="info"
.\src-tauri\target\release\ccmesh.exe
```

访问：

```text
http://服务器IP:3999/
```

## 6. systemd 后台运行（Linux 推荐）

创建用户和目录：

```bash
sudo useradd --system --home /var/lib/ccmesh --shell /usr/sbin/nologin ccmesh || true
sudo mkdir -p /opt/ccmesh /var/lib/ccmesh
sudo chown -R ccmesh:ccmesh /opt/ccmesh /var/lib/ccmesh
```

复制项目文件或构建产物到 `/opt/ccmesh`。最简单方式是直接把完整项目目录放到 `/opt/ccmesh`，保证里面存在：

```text
/opt/ccmesh/dist/
/opt/ccmesh/src-tauri/target/release/ccmesh
```

创建服务文件：

```bash
sudo tee /etc/systemd/system/ccmesh.service >/dev/null <<'EOF'
[Unit]
Description=ccMesh Web AI Gateway
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=ccmesh
Group=ccmesh
WorkingDirectory=/opt/ccmesh
Environment=CCMESH_PORT=3999
Environment=CCMESH_DATA_DIR=/var/lib/ccmesh
Environment=CCMESH_ADMIN_TOKEN=change-me-to-a-long-random-secret
Environment=RUST_LOG=info
ExecStart=/opt/ccmesh/src-tauri/target/release/ccmesh
Restart=always
RestartSec=3
LimitNOFILE=65535

[Install]
WantedBy=multi-user.target
EOF
```

启动：

```bash
sudo systemctl daemon-reload
sudo systemctl enable --now ccmesh
sudo systemctl status ccmesh
```

查看日志：

```bash
journalctl -u ccmesh -f
```

## 7. Nginx 反向代理（可选）

如果需要域名和 HTTPS，可以使用 Nginx 转发到本机 `3999`。

```nginx
server {
    listen 80;
    server_name your-domain.com;

    client_max_body_size 64m;

    location / {
        proxy_pass http://127.0.0.1:3999;
        proxy_http_version 1.1;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;

        # 流式响应必须关闭缓冲，否则 Codex / OpenAI stream 可能变慢或中断
        proxy_buffering off;
        proxy_cache off;
        proxy_read_timeout 3600s;
        proxy_send_timeout 3600s;
    }
}
```

HTTPS 建议用 certbot：

```bash
sudo apt install -y certbot python3-certbot-nginx
sudo certbot --nginx -d your-domain.com
```

访问：

```text
https://your-domain.com/
```

## 8. 客户端如何配置

假设服务地址是：

```text
http://服务器IP:3999
```

或：

```text
https://your-domain.com
```

### OpenAI 兼容客户端

Base URL：

```text
http://服务器IP:3999/v1
```

API Key：

```text
填写 CCMESH_ADMIN_TOKEN 的值
```

常见接口：

```text
POST /v1/chat/completions
POST /v1/responses
GET  /v1/models
```

### Codex / Responses

Base URL 使用根域名或 `/v1` 取决于客户端要求。请求最终会进入：

```text
/v1/responses
```

API Key / Token 填 `CCMESH_ADMIN_TOKEN` 的值。

如果某些中转站要求特定 User-Agent，请在 ccMesh 设置页配置 UA。ccMesh 会在真实转发和端点测试中使用该配置。

### Claude 兼容客户端

Base URL 使用根域名：

```text
http://服务器IP:3999
```

API Key / Auth Token 填 `CCMESH_ADMIN_TOKEN` 的值。

常见接口：

```text
POST /v1/messages
POST /v1/messages/count_tokens
```

## 9. 首次网页配置流程

1. 打开 `http://服务器IP:3999/`。
2. 输入 `CCMESH_ADMIN_TOKEN` 登录。
3. 进入「设置」：确认端口、代理、UA 等全局配置。
4. 进入「端点管理」：新增上游中转站点。
   - API URL 填中转站根地址，例如 `https://example.com` 或站点要求的根路径。
   - API Key 填该站点密钥。
   - 类型选择 `openai`、`claude` 或 `codex`。
   - 模型列表填写站点支持的真实模型。
   - 如需模型映射，配置入站模型到出站模型的映射。
5. 点击端点卡片上的「测试连通性」，选择本次测试模型。
6. 进入「余额查询」或端点编辑的余额页签，配置/探测余额模板。
7. 进入「日志」查看真实请求四阶段详情：接收请求、转发请求、接收上游响应、响应客户端。

## 10. 数据备份与迁移

需要备份整个数据目录：

```bash
sudo systemctl stop ccmesh
sudo tar -czf ccmesh-data-$(date +%F).tar.gz -C /var/lib ccmesh
sudo systemctl start ccmesh
```

恢复：

```bash
sudo systemctl stop ccmesh
sudo rm -rf /var/lib/ccmesh
sudo tar -xzf ccmesh-data-YYYY-MM-DD.tar.gz -C /var/lib
sudo chown -R ccmesh:ccmesh /var/lib/ccmesh
sudo systemctl start ccmesh
```

## 11. 升级流程

```bash
cd /opt/ccmesh
git pull
pnpm install
pnpm build
pnpm server:build
sudo systemctl restart ccmesh
```

升级前建议先备份 `/var/lib/ccmesh`。

## 12. 常见问题

### 访问网页返回 404 或空白

确认已执行：

```bash
pnpm build
```

并确认服务的 `WorkingDirectory` 下存在 `dist/index.html`。

### 访问网页只显示锁定提示

说明服务未配置 `CCMESH_ADMIN_TOKEN`。公网部署必须设置：

```bash
export CCMESH_ADMIN_TOKEN='替换为足够长的随机字符串'
```

如果使用 systemd，修改 `/etc/systemd/system/ccmesh.service` 后执行：

```bash
sudo systemctl daemon-reload
sudo systemctl restart ccmesh
```

### API 客户端返回 401 Unauthorized

客户端没有携带访问令牌，或访问令牌不正确。OpenAI/Codex/Claude 客户端的 API Key 填 `CCMESH_ADMIN_TOKEN` 的值即可。

### 端口被占用

```bash
sudo lsof -i :3999
```

修改 `CCMESH_PORT` 后重启服务。

### 流式响应容易断开

如果前面套了 Nginx，确认：

```nginx
proxy_buffering off;
proxy_read_timeout 3600s;
proxy_send_timeout 3600s;
```

同时检查上游站点是否对 UA、模型、工具调用、图片、长上下文有限制。

### 请求体过大返回 413

当前后端请求体限制为 64 MB。正常文本请求不应触发；如果频繁触发，优先检查客户端是否上传了超大图片、文件或异常日志内容。

### 余额重启后不显示

网页会在浏览器本地缓存最近一次余额查询结果。换浏览器、清缓存或换设备后需要重新查询余额。生产环境如果需要多用户共享余额快照，应后续增加后端持久化余额结果。
