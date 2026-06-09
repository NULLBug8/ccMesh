# Tauri v2 开发技巧与最佳实践

> 更新日期：2026-05-27

---

## 一、图标生成

用一个高分辨率源图标自动生成所有平台所需尺寸：

```bash
pnpm tauri icon <源图片路径>   # 建议 1024x1024 PNG
```

生成到 `src-tauri/icons/`：

```
icons/
├── 32x32.png
├── 128x128.png
├── 128x128@2x.png
├── icon.icns      # macOS
├── icon.ico       # Windows
└── icon.png       # Linux
```

---

## 二、开发调试技巧

### 2.1 前端调试

在应用窗口右键选择 "Inspect"，或按 `Ctrl + Shift + I` 打开 Web Inspector，支持完整的 Chrome DevTools。

### 2.2 热重载

- 前端修改：Vite 自动热更新，无需手动刷新
- Rust 修改：`tauri dev` 自动检测变化并重新编译

### 2.3 禁用文件监听

```bash
pnpm tauri dev -- --no-watch
```

### 2.4 Release 模式运行

```bash
pnpm tauri dev -- --release
```

### 2.5 传递参数

```bash
# 传递给 cargo 的参数
pnpm tauri dev -- [runnerArgs]

# 传递给应用的参数
pnpm tauri dev -- -- [appArgs]
```

---

## 三、命令行工具速查

| 命令 | 说明 |
|------|------|
| `tauri dev` | 开发模式运行（热重载） |
| `tauri build` | 构建发布版本并生成安装包 |
| `tauri init` | 在现有目录初始化 Tauri |
| `tauri info` | 显示环境信息 |
| `tauri add <plugin>` | 添加 Tauri 插件 |
| `tauri icon <path>` | 生成各平台图标 |
| `tauri permission ls` | 列出可用权限 |
| `tauri permission new` | 创建新权限文件 |
| `tauri capability new` | 创建新 capability |
| `tauri migrate` | 从 v1 迁移到 v2 |
| `tauri completions` | 生成 shell 补全 |

### build 命令选项

```bash
pnpm tauri build -- --no-bundle          # 仅编译，不打包
pnpm tauri build -- --bundles nsis,msi   # 指定打包格式
pnpm tauri build -- --debug              # debug 模式构建
pnpm tauri build -- --no-sign            # 跳过代码签名
```

---

## 四、窗口管理技巧

### 4.1 三种配置方式

1. `tauri.conf.json` — 静态配置
2. JavaScript API — 前端动态控制
3. Rust 代码 — 后端高级控制

### 4.2 自定义标题栏

**tauri.conf.json：**
```json
{
  "windows": [{ "decorations": false }]
}
```

**HTML：**
```html
<div class="titlebar">
  <div data-tauri-drag-region></div>
  <div class="controls">
    <button id="titlebar-minimize">_</button>
    <button id="titlebar-maximize">□</button>
    <button id="titlebar-close">×</button>
  </div>
</div>
```

**JavaScript 控制：**
```javascript
import { getCurrentWindow } from '@tauri-apps/api/window';

const appWindow = getCurrentWindow();

document.getElementById('titlebar-minimize')?.addEventListener('click', () => appWindow.minimize());
document.getElementById('titlebar-maximize')?.addEventListener('click', () => appWindow.toggleMaximize());
document.getElementById('titlebar-close')?.addEventListener('click', () => appWindow.close());
```

### 4.3 多窗口管理

```rust
use tauri::WebviewWindowBuilder;

#[tauri::command]
async fn open_settings(app: tauri::AppHandle) {
    WebviewWindowBuilder::new(&app, "settings", tauri::WebviewUrl::App("/settings".into()))
        .title("设置")
        .inner_size(400.0, 300.0)
        .build()
        .unwrap();
}
```

---

## 五、权限管理（Capabilities）

### 5.1 基本结构

```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "default",
  "description": "Capability for the main window",
  "windows": ["main"],
  "platforms": ["linux", "macOS", "windows"],
  "permissions": [
    "core:default",
    "opener:default",
    "clipboard-manager:allow-write-text"
  ]
}
```

### 5.2 平台特定权限

```json
{
  "identifier": "mobile-capability",
  "windows": ["main"],
  "platforms": ["iOS", "android"],
  "permissions": ["nfc:allow-scan", "biometric:allow-authenticate"]
}
```

### 5.3 远程 URL 访问权限

```json
{
  "identifier": "remote-capability",
  "windows": ["main"],
  "remote": {
    "urls": ["https://*.example.com"]
  },
  "permissions": ["http:default"]
}
```

---

## 六、前后端通信进阶

### 6.1 异步命令（不阻塞 UI）

```rust
#[tauri::command]
async fn heavy_task() -> Result<String, String> {
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    Ok("完成".to_string())
}
```

### 6.2 大数据传输优化

```rust
use tauri::ipc::Response;

#[tauri::command]
fn read_file() -> Response {
    let data = std::fs::read("/path/to/file").unwrap();
    tauri::ipc::Response::new(data)
}
```

### 6.3 Channel 流式传输

```rust
#[tauri::command]
async fn load_image(path: PathBuf, reader: tauri::ipc::Channel<&[u8]>) {
    let mut file = tokio::fs::File::open(path).await.unwrap();
    let mut chunk = vec![0; 4096];
    loop {
        let len = file.read(&mut chunk).await.unwrap();
        if len == 0 { break; }
        reader.send(&chunk).unwrap();
    }
}
```

### 6.4 状态管理

```rust
use std::sync::Mutex;

struct AppState {
    db: Mutex<Database>,
}

pub fn run() {
    tauri::Builder::default()
        .manage(AppState { db: Mutex::new(Database::new()) })
        .invoke_handler(tauri::generate_handler![my_command])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[tauri::command]
fn my_command(state: tauri::State<AppState>) -> String {
    let db = state.db.lock().unwrap();
    "result".to_string()
}
```

---

## 七、错误处理最佳实践

```rust
use thiserror::Error;

#[derive(Debug, Error)]
enum AppError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("数据库错误: {0}")]
    Database(String),
}

impl serde::Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.to_string().as_ref())
    }
}

#[tauri::command]
fn my_command() -> Result<(), AppError> {
    std::fs::File::open("path/to/file")?;
    Ok(())
}
```

---

## 八、打包与自动更新

### 8.1 生成签名密钥

```bash
pnpm tauri signer generate -- -w ~/.tauri/myapp.key
```

### 8.2 配置自动更新

**tauri.conf.json：**
```json
{
  "bundle": {
    "createUpdaterArtifacts": true
  },
  "plugins": {
    "updater": {
      "pubkey": "CONTENT FROM PUBLICKEY.PEM",
      "endpoints": [
        "https://releases.myapp.com/{{target}}/{{arch}}/{{current_version}}"
      ]
    }
  }
}
```

**环境变量（构建时）：**
```powershell
$env:TAURI_SIGNING_PRIVATE_KEY="Path or content of your private key"
$env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD=""
```

**前端检查更新：**
```typescript
import { check } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';

const update = await check();
if (update) {
  await update.downloadAndInstall();
  await relaunch();
}
```

---

## 九、常见问题

### 9.1 异步命令不能使用借用参数

```rust
// 错误
#[tauri::command]
async fn my_command(value: &str) -> String { ... }

// 正确：使用 String
#[tauri::command]
async fn my_command(value: String) -> String { ... }
```

### 9.2 lib.rs 中命令不能标记为 pub

使用单独模块：

```rust
// src-tauri/src/commands.rs
#[tauri::command]
pub fn my_command() -> String { "Hello".to_string() }

// src-tauri/src/lib.rs
mod commands;
.invoke_handler(tauri::generate_handler![commands::my_command])
```

### 9.3 源码控制建议

- **提交**: `Cargo.lock`、`Cargo.toml`、`tauri.conf.json`
- **不提交**: `target/`、`node_modules/`

---

## 十、跨平台条件编译

```rust
#[cfg(desktop)]
app.handle().plugin(tauri_plugin_updater::Builder::new().build());

#[cfg(mobile)]
app.handle().plugin(tauri_plugin_biometric::init());
```

---

## 参考资料

- [Tauri v2 官方文档](https://tauri.app)
- [Tauri v2 中文文档](https://v2.tauri.org.cn)
- [Tauri GitHub](https://github.com/tauri-apps/tauri)
