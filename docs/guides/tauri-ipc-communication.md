# Tauri v2 前后端通信详解

> 更新日期：2026-05-27

---

## 通信方式总览

Tauri v2 提供 4 种前后端通信方式：

| 方式 | 方向 | 模式 | 适用场景 |
|------|------|------|----------|
| `invoke` | 前端 → Rust | 请求-响应 | 调用函数、获取数据 |
| `Event` | 双向 | 发布-订阅 | 状态通知、广播消息 |
| `Channel` | Rust → 前端 | 单向流 | 进度更新、大数据传输 |
| `Custom Protocol` | 前端 → Rust | HTTP 风格 | 静态资源、REST API |

---

## 一、invoke — 命令调用（请求-响应）

最常用的通信方式。前端主动调用 Rust 函数，等待返回结果。

### 1.1 基本用法

**Rust 端定义命令：**

```rust
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

// 注册命令
.invoke_handler(tauri::generate_handler![greet])
```

**前端调用：**

```typescript
import { invoke } from '@tauri-apps/api/core';

const result = await invoke('greet', { name: '张三' });
console.log(result);  // "Hello, 张三! You've been greeted from Rust!"
```

### 1.2 传递复杂参数

**Rust 端：**

```rust
use serde::Deserialize;

#[derive(Deserialize)]
struct User {
    name: String,
    age: u32,
    email: Option<String>,
}

#[tauri::command]
fn create_user(user: User) -> String {
    format!("创建用户: {}, {}岁", user.name, user.age)
}
```

**前端调用：**

```typescript
await invoke('create_user', {
    user: {
        name: '张三',
        age: 25,
        email: 'zhangsan@example.com'  // 可选字段可省略
    }
});
```

### 1.3 错误处理

**Rust 端：**

```rust
use serde::Serialize;

#[derive(Debug, Serialize)]
enum AppError {
    NotFound(String),
    PermissionDenied,
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::NotFound(msg) => write!(f, "未找到: {}", msg),
            AppError::PermissionDenied => write!(f, "权限不足"),
        }
    }
}

#[tauri::command]
fn get_data(id: u32) -> Result<String, AppError> {
    if id == 0 {
        return Err(AppError::NotFound("ID 不能为 0".to_string()));
    }
    Ok(format!("数据: {}", id))
}
```

**前端捕获错误：**

```typescript
try {
    const data = await invoke('get_data', { id: 0 });
} catch (error) {
    console.error('调用失败:', error);  // "未找到: ID 不能为 0"
}
```

### 1.4 异步命令

耗时操作应使用异步命令，避免阻塞 UI：

```rust
#[tauri::command]
async fn fetch_data(url: String) -> Result<String, String> {
    let response = reqwest::get(&url).await.map_err(|e| e.to_string())?;
    let body = response.text().await.map_err(|e| e.to_string())?;
    Ok(body)
}
```

### 1.5 状态管理

通过 `tauri::State` 共享状态：

```rust
use std::sync::Mutex;

struct AppState {
    counter: Mutex<u32>,
}

#[tauri::command]
fn increment(state: tauri::State<AppState>) -> u32 {
    let mut count = state.counter.lock().unwrap();
    *count += 1;
    *count
}

// 注册状态
tauri::Builder::default()
    .manage(AppState { counter: Mutex::new(0) })
    .invoke_handler(tauri::generate_handler![increment])
```

---

## 二、Event System — 事件系统（发布-订阅）

双向通信，前端和 Rust 都可以主动发消息，不需要等待响应。

### 2.1 前端监听 Rust 事件

```typescript
import { listen, emit } from '@tauri-apps/api/core';

// 监听 Rust 发来的事件
const unlisten = await listen('download-progress', (event) => {
    console.log('进度:', event.payload);  // { percent: 50 }
});

// 不再监听时取消
unlisten();
```

### 2.2 前端发送事件给 Rust

```typescript
await emit('frontend-ready', { status: 'ok' });
```

### 2.3 Rust 监听前端事件

```rust
use tauri::Listener;

app.listen("frontend-ready", |event| {
    println!("前端就绪: {:?}", event.payload());
});
```

### 2.4 Rust 发送事件给前端

```rust
use tauri::Emitter;

// 发送给所有窗口
app.emit("download-progress", serde_json::json!({ "percent": 50 }))?;

// 发送给特定窗口
app.emit_to("main", "download-progress", serde_json::json!({ "percent": 50 }))?;
```

### 2.5 事件命名规范

```typescript
// 建议使用 kebab-case 或 命名空间
await listen('download-progress', handler);
await listen('user:login', handler);
await listen('file:created', handler);
```

---

## 三、Channel API — 流式传输

用于持续传输大量数据（如文件流、进度条），比事件系统更高效。

### 3.1 Rust 端发送流数据

```rust
use tauri::ipc::Channel;
use serde::Serialize;

#[derive(Serialize, Clone)]
struct Progress {
    percent: u32,
    message: String,
}

#[tauri::command]
async fn download_file(url: String, on_progress: Channel<Progress>) -> Result<String, String> {
    for i in 0..=100 {
        // 持续推送进度
        on_progress.send(Progress {
            percent: i,
            message: format!("下载中... {}%", i),
        }).map_err(|e| e.to_string())?;
    }
    Ok("下载完成".to_string())
}
```

### 3.2 前端接收流数据

```typescript
import { Channel, invoke } from '@tauri-apps/api/core';

interface Progress {
    percent: number;
    message: string;
}

const onProgress = new Channel<Progress>();
onProgress.onmessage = (progress) => {
    console.log('进度:', progress.percent, progress.message);
};

const result = await invoke('download_file', {
    url: 'https://example.com/file.zip',
    onProgress
});
console.log(result);  // "下载完成"
```

### 3.3 Channel vs Event 对比

| 特性 | Channel | Event |
|------|---------|-------|
| 方向 | 单向 (Rust → 前端) | 双向 |
| 性能 | 更高（直接 IPC） | 稍低（经过事件循环） |
| 适用场景 | 大数据流、进度更新 | 通知、广播 |
| 一对多 | 否 | 是 |

---

## 四、Custom Protocol — 自定义协议

注册自定义 URL scheme，前端通过 `fetch` 访问 Rust 资源。

### 4.1 注册自定义协议

```rust
use tauri::http::Response;

app.register_uri_scheme("myapp", |_app, request| {
    let path = request.uri().path();

    match path {
        "/api/config" => {
            let data = serde_json::json!({ "theme": "dark", "lang": "zh" });
            Response::builder()
                .header("Content-Type", "application/json")
                .body(data.to_string().into_bytes())
                .unwrap()
        }
        _ => Response::builder()
            .status(404)
            .body(b"Not Found".to_vec())
            .unwrap()
    }
});
```

### 4.2 前端访问

```typescript
// 像普通 HTTP 请求一样使用
const response = await fetch('myapp://api/config');
const config = await response.json();
console.log(config);  // { theme: "dark", lang: "zh" }
```

### 4.3 适用场景

- 提供静态资源（图片、配置文件）
- RESTful 风格的 API
- 与现有前端 HTTP 库集成（如 axios）

---

## 五、通信方式选择指南

```
需要调用 Rust 函数并获取返回值？
├── 是 → 使用 invoke
│   ├── 简单请求 → invoke
│   └── 大量数据流 → Channel
└── 否 → 使用 Event
    ├── 通知/广播 → Event
    └── 资源访问 → Custom Protocol
```

### 常见场景推荐

| 场景 | 推荐方式 | 原因 |
|------|----------|------|
| 读取文件内容 | `invoke` | 一次性请求 |
| 文件上传进度 | `Channel` | 持续推送进度 |
| 用户登录通知 | `Event` | 广播给多个监听者 |
| 获取配置信息 | `invoke` 或 `Custom Protocol` | 看是否需要 HTTP 语义 |
| 实时日志流 | `Channel` | 高频数据流 |
| 窗口间通信 | `Event` | 跨窗口广播 |

---

## 六、完整示例：带进度的文件下载

### Rust 端

```rust
use serde::Serialize;
use tauri::ipc::Channel;

#[derive(Serialize, Clone)]
struct DownloadProgress {
    bytes_downloaded: u64,
    total_bytes: u64,
    percent: f64,
}

#[tauri::command]
async fn download_with_progress(
    url: String,
    save_path: String,
    on_progress: Channel<DownloadProgress>,
) -> Result<String, String> {
    let total_bytes = 1024 * 1024;  // 示例值

    for i in 0..=100 {
        let downloaded = (total_bytes * i) / 100;
        on_progress.send(DownloadProgress {
            bytes_downloaded: downloaded,
            total_bytes,
            percent: i as f64,
        }).map_err(|e| e.to_string())?;
    }

    Ok(format!("已保存到: {}", save_path))
}
```

### 前端

```typescript
import { invoke, Channel } from '@tauri-apps/api/core';

interface DownloadProgress {
    bytes_downloaded: number;
    total_bytes: number;
    percent: number;
}

async function startDownload() {
    const progressBar = document.getElementById('progress');
    const statusText = document.getElementById('status');

    const onProgress = new Channel<DownloadProgress>();
    onProgress.onmessage = (progress) => {
        progressBar.style.width = `${progress.percent}%`;
        statusText.textContent = `${progress.percent.toFixed(1)}% - ${(progress.bytes_downloaded / 1024 / 1024).toFixed(2)} MB`;
    };

    try {
        const result = await invoke('download_with_progress', {
            url: 'https://example.com/file.zip',
            savePath: 'C:/Downloads/file.zip',
            onProgress
        });
        statusText.textContent = result;
    } catch (error) {
        statusText.textContent = `下载失败: ${error}`;
    }
}
```

---

## 参考资料

- [Tauri v2 IPC 文档](https://v2.tauri.app/develop/calling-rust/)
- [Tauri Event System](https://v2.tauri.app/develop/calling-rust/#event-system)
- [Tauri Channel API](https://v2.tauri.app/develop/calling-rust/#channels)
