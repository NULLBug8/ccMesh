# Tauri v2 项目目录组织最佳实践

> 更新日期：2026-05-27

---

## 一、整体项目结构

```
my-tauri-app/
├── index.html                  # 前端入口
├── package.json                # Node.js 依赖
├── vite.config.ts              # Vite 构建配置
├── tsconfig.json               # TypeScript 配置
│
├── src/                        # 前端源码（React/Vue/Svelte）
│   ├── main.tsx                # 前端入口
│   ├── App.tsx                 # 根组件
│   ├── components/             # UI 组件
│   ├── hooks/                  # 自定义 Hook
│   ├── services/               # API 调用封装
│   │   └── tauri.ts            # Tauri 命令调用封装
│   ├── stores/                 # 状态管理
│   └── types/                  # TypeScript 类型定义
│
└── src-tauri/                  # Rust 后端
    ├── Cargo.toml              # Rust 依赖配置
    ├── tauri.conf.json         # Tauri 应用配置
    ├── build.rs                # 构建脚本
    ├── capabilities/           # 权限声明
    │   └── default.json
    ├── icons/                  # 应用图标
    │
    └── src/                    # Rust 源码
        ├── main.rs             # 入口
        ├── lib.rs              # 模块注册和命令注册
        ├── error.rs            # 统一错误类型
        ├── state.rs            # 全局状态定义
        │
        ├── commands/           # 命令处理器（按功能分模块）
        │   ├── mod.rs
        │   ├── user.rs
        │   ├── file.rs
        │   ├── settings.rs
        │   └── auth.rs
        │
        ├── models/             # 数据模型
        │   ├── mod.rs
        │   ├── user.rs
        │   └── config.rs
        │
        ├── services/           # 业务逻辑层
        │   ├── mod.rs
        │   ├── user_service.rs
        │   └── file_service.rs
        │
        └── utils/              # 工具函数
            ├── mod.rs
            └── path.rs
```

---

## 二、各层职责

| 层 | 目录 | 职责 |
|----|------|------|
| 入口层 | `main.rs`, `lib.rs` | 启动应用，注册模块和命令 |
| 命令层 | `commands/` | 接收前端调用，解析参数，调用 service |
| 业务层 | `services/` | 核心业务逻辑，复杂计算和处理 |
| 数据层 | `models/` | 数据结构定义，序列化/反序列化 |
| 工具层 | `utils/` | 通用工具函数 |
| 错误层 | `error.rs` | 统一错误类型定义 |
| 状态层 | `state.rs` | 全局共享状态定义 |

---

## 三、核心文件详解

### 3.1 main.rs — 应用入口

```rust
// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    ccmesh_lib::run()
}
```

**要点：**
- 保持最小化，只调用 `lib.rs` 中的 `run()` 函数
- `windows_subsystem = "windows"` 防止 release 模式弹出控制台

### 3.2 lib.rs — 模块注册中心

```rust
// 声明所有模块
mod commands;
mod error;
mod models;
mod services;
mod state;

use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // 注册全局状态
        .manage(AppState::new())
        // 注册所有命令
        .invoke_handler(tauri::generate_handler![
            // 用户模块
            commands::user::get_user,
            commands::user::create_user,
            commands::user::update_user,
            commands::user::delete_user,
            // 文件模块
            commands::file::read_file,
            commands::file::write_file,
            commands::file::list_files,
            // 设置模块
            commands::settings::get_settings,
            commands::settings::save_settings,
            // 认证模块
            commands::auth::login,
            commands::auth::logout,
            commands::auth::check_auth,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

**要点：**
- 集中注册所有模块和命令
- 使用 `manage()` 注册全局状态
- 命令按模块分组，便于维护

### 3.3 error.rs — 统一错误处理

```rust
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("未找到: {0}")]
    NotFound(String),

    #[error("权限不足")]
    PermissionDenied,

    #[error("参数无效: {0}")]
    InvalidArgument(String),

    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON 错误: {0}")]
    Json(#[from] serde_json::Error),

    #[error("网络错误: {0}")]
    Network(#[from] reqwest::Error),

    #[error("未知错误: {0}")]
    Unknown(String),
}

// 必须实现 Serialize，前端才能收到错误信息
impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.to_string().as_ref())
    }
}
```

**要点：**
- 使用 `thiserror` 简化错误定义
- 实现 `Serialize` 是必须的，否则前端收不到错误信息
- 使用 `#[from]` 自动转换错误类型

### 3.4 state.rs — 全局状态管理

```rust
use std::collections::HashMap;
use std::sync::Mutex;
use crate::models::user::User;
use crate::models::config::AppConfig;

pub struct AppState {
    pub users: Mutex<HashMap<u32, User>>,
    pub config: Mutex<AppConfig>,
    pub auth_token: Mutex<Option<String>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            users: Mutex::new(HashMap::new()),
            config: Mutex::new(AppConfig::default()),
            auth_token: Mutex::new(None),
        }
    }
}
```

**要点：**
- 使用 `Mutex` 保证线程安全
- 集中管理所有共享状态
- 提供 `new()` 构造函数设置默认值

### 3.5 commands/user.rs — 命令处理器示例

```rust
use serde::{Deserialize, Serialize};
use tauri::State;
use crate::error::AppError;
use crate::state::AppState;
use crate::services::user_service;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct User {
    pub id: u32,
    pub name: String,
    pub email: String,
}

#[derive(Deserialize)]
pub struct CreateUserRequest {
    pub name: String,
    pub email: String,
}

/// 获取用户
#[tauri::command]
pub fn get_user(state: State<AppState>, id: u32) -> Result<User, AppError> {
    let users = state.users.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
    users.get(&id)
        .cloned()
        .ok_or_else(|| AppError::NotFound(format!("用户 {} 不存在", id)))
}

/// 创建用户
#[tauri::command]
pub fn create_user(
    state: State<AppState>,
    request: CreateUserRequest,
) -> Result<User, AppError> {
    // 参数验证
    user_service::validate_user(&request.name, &request.email)?;

    // 创建用户
    let mut users = state.users.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
    let id = users.len() as u32 + 1;
    let user = User {
        id,
        name: request.name,
        email: request.email,
    };
    users.insert(id, user.clone());

    Ok(user)
}

/// 更新用户
#[tauri::command]
pub fn update_user(
    state: State<AppState>,
    id: u32,
    name: Option<String>,
    email: Option<String>,
) -> Result<User, AppError> {
    let mut users = state.users.lock().map_err(|e| AppError::Unknown(e.to_string()))?;

    let user = users.get_mut(&id)
        .ok_or_else(|| AppError::NotFound(format!("用户 {} 不存在", id)))?;

    if let Some(name) = name {
        user.name = name;
    }
    if let Some(email) = email {
        user.email = email;
    }

    Ok(user.clone())
}

/// 删除用户
#[tauri::command]
pub fn delete_user(state: State<AppState>, id: u32) -> Result<(), AppError> {
    let mut users = state.users.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
    users.remove(&id)
        .ok_or_else(|| AppError::NotFound(format!("用户 {} 不存在", id)))?;
    Ok(())
}
```

**要点：**
- 命令函数保持薄，只做参数解析和调用 service
- 复杂验证逻辑放在 `services` 层
- 返回 `Result<T, AppError>` 统一错误处理
- 使用 `State<AppState>` 访问共享状态

### 3.6 services/user_service.rs — 业务逻辑层

```rust
use crate::error::AppError;

/// 验证用户数据
pub fn validate_user(name: &str, email: &str) -> Result<(), AppError> {
    if name.trim().is_empty() {
        return Err(AppError::InvalidArgument("用户名不能为空".to_string()));
    }

    if name.len() > 50 {
        return Err(AppError::InvalidArgument("用户名长度不能超过 50".to_string()));
    }

    if !email.contains('@') || !email.contains('.') {
        return Err(AppError::InvalidArgument("邮箱格式无效".to_string()));
    }

    Ok(())
}

/// 生成用户显示名称
pub fn display_name(name: &str) -> String {
    if name.len() > 20 {
        format!("{}...", &name[..20])
    } else {
        name.to_string()
    }
}
```

**要点：**
- 复杂业务逻辑放在这里
- 命令层调用 service，service 不直接访问状态
- 便于单元测试

### 3.7 models/user.rs — 数据模型

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: u32,
    pub name: String,
    pub email: String,
    #[serde(default)]
    pub role: UserRole,
    #[serde(default)]
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum UserRole {
    #[default]
    User,
    Admin,
    Moderator,
}

#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
    pub name: String,
    pub email: String,
    #[serde(default)]
    pub role: UserRole,
}

#[derive(Debug, Deserialize)]
pub struct UpdateUserRequest {
    pub name: Option<String>,
    pub email: Option<String>,
    pub role: Option<UserRole>,
}
```

**要点：**
- 使用 `serde` 进行序列化/反序列化
- 请求和响应模型分开定义
- 使用 `#[serde(default)]` 处理可选字段

### 3.8 commands/mod.rs — 模块导出

```rust
pub mod user;
pub mod file;
pub mod settings;
pub mod auth;
```

### 3.9 models/mod.rs

```rust
pub mod user;
pub mod config;
```

### 3.10 services/mod.rs

```rust
pub mod user_service;
pub mod file_service;
```

---

## 四、前端调用封装

### 4.1 services/tauri.ts — Tauri 命令调用封装

```typescript
import { invoke } from '@tauri-apps/api/core';

// 类型定义
interface User {
    id: number;
    name: string;
    email: string;
    role: 'User' | 'Admin' | 'Moderator';
}

interface CreateUserRequest {
    name: string;
    email: string;
    role?: 'User' | 'Admin' | 'Moderator';
}

// 用户相关命令
export const userApi = {
    getUser: (id: number) => invoke<User>('get_user', { id }),

    createUser: (request: CreateUserRequest) =>
        invoke<User>('create_user', { request }),

    updateUser: (id: number, name?: string, email?: string) =>
        invoke<User>('update_user', { id, name, email }),

    deleteUser: (id: number) => invoke<void>('delete_user', { id }),
};

// 文件相关命令
export const fileApi = {
    readFile: (path: string) => invoke<string>('read_file', { path }),

    writeFile: (path: string, content: string) =>
        invoke<void>('write_file', { path, content }),

    listFiles: (dir: string) => invoke<string[]>('list_files', { dir }),
};

// 设置相关命令
export const settingsApi = {
    getSettings: () => invoke<Record<string, unknown>>('get_settings'),

    saveSettings: (settings: Record<string, unknown>) =>
        invoke<void>('save_settings', { settings }),
};
```

### 4.2 前端使用示例

```typescript
import { userApi } from './services/tauri';

// 获取用户
const user = await userApi.getUser(1);
console.log(user.name);

// 创建用户
const newUser = await userApi.createUser({
    name: '张三',
    email: 'zhangsan@example.com',
});

// 更新用户
await userApi.updateUser(1, '李四');
```

---

## 五、Cargo.toml 推荐依赖

```toml
[package]
name = "ccmesh"
version = "0.1.0"
edition = "2021"

[dependencies]
tauri = { version = "2", features = [] }
tauri-plugin-opener = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "1"          # 错误处理
tokio = { version = "1", features = ["full"] }  # 异步运行时（如需要）
reqwest = { version = "0.12", features = ["json"] }  # HTTP 客户端（如需要）
chrono = { version = "0.4", features = ["serde"] }   # 时间处理（如需要）
```

---

## 六、扩展指南

### 6.1 添加新模块

1. 在 `commands/` 下创建新文件，如 `commands/notification.rs`
2. 在 `commands/mod.rs` 中添加 `pub mod notification;`
3. 在 `lib.rs` 的 `generate_handler![]` 中注册新命令

### 6.2 添加新插件

```bash
pnpm tauri add clipboard-manager
```

然后在 `capabilities/default.json` 中添加权限：

```json
{
    "permissions": [
        "core:default",
        "clipboard-manager:allow-read-text",
        "clipboard-manager:allow-write-text"
    ]
}
```

### 6.3 多窗口支持

```rust
// 在 lib.rs 中
.setup(|app| {
    // 创建第二个窗口
    tauri::WebviewWindowBuilder::new(
        app,
        "settings",
        tauri::WebviewUrl::App("/settings".into())
    )
    .title("设置")
    .inner_size(400.0, 300.0)
    .build()?;

    Ok(())
})
```

---

## 七、项目组织原则总结

| 原则 | 说明 |
|------|------|
| 单一职责 | 每个文件只负责一个功能 |
| 分层架构 | commands → services → models |
| 命令层薄 | commands 只做参数解析和调用 service |
| 统一错误 | 一个 `AppError` 处理所有错误类型 |
| 状态集中 | `AppState` 统一管理共享状态 |
| 模块化 | 按功能模块拆分，便于维护和扩展 |
| 类型安全 | 使用 serde 保证前后端类型一致 |

---

## 参考资料

- [Tauri v2 官方文档](https://v2.tauri.app)
- [Rust 项目结构最佳实践](https://doc.rust-lang.org/cargo/guide/project-structure.html)
- [serde 官方文档](https://serde.rs)
- [thiserror 官方文档](https://github.com/dtolnay/thiserror)
