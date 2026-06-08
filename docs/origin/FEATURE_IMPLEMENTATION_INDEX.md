# ccNexus 功能与实现索引（旧版参考）

## 文档说明

本文档记录旧版 ccNexus 项目（`E:\myCode\localAway\ccNexus`）各功能模块与具体代码实现的对应关系，作为重构的参考依据。

**旧版项目信息**
- 项目路径：`E:\myCode\localAway\ccNexus`
- 技术栈：Go + Wails v2 + Vue.js + SQLite（旧版技术栈，重构时待定）
- 代码结构：详见项目目录

**重构参考用途**
- 了解旧版功能实现方式
- 识别可复用的代码和设计
- 确定重构重点和优先级
- 避免重复已知的问题

**重构注意事项**
- 旧版代码仅供参考，不直接复用
- 重构时应重新设计架构
- 关注代码质量和可维护性
- 移除不需要的功能（Token Pool 管理等）

---

## 1. 核心代理功能

### 1.1 多端点轮换与故障转移

**功能描述**
- 自动端点轮换（按顺序循环）
- 故障转移（端点失败自动切换）
- 重试策略（同一端点最多连续失败 2 次后切换）
- 网络瞬时错误重试（300ms 延迟）
- 手动端点切换

**代码实现**

| 模块 | 文件路径 | 关键函数/方法 |
|------|----------|---------------|
| 代理核心 | `internal/proxy/proxy.go` | `rotateEndpoint()`, `getCurrentEndpoint()`, `SetCurrentEndpoint()` |
| 请求处理 | `internal/proxy/proxy_request.go` | `handleProxyRequest()`, `runEndpointAttempt()`, `handleSendError()` |
| 端点解析 | `internal/proxy/endpoint_resolver.go` | `ResolveEndpoint()`, `parseEndpointFromHeader()`, `parseEndpointFromModel()` |
| 配置管理 | `internal/config/config.go` | `Endpoint` 结构体, `GetEndpoints()` |

**关键逻辑**
```go
// 轮换逻辑（proxy.go:249）
p.currentIndex = (oldIndex + 1) % len(endpoints)

// 重试策略（proxy_request.go:88）
if endpointAttempts >= 2 && !reqCtx.useSpecificEndpoint {
    p.rotateEndpoint()
    endpointAttempts = 0
}
```

### 1.2 API 格式转换

**功能描述**
- Claude ↔ OpenAI Chat（核心功能）
- 支持流式和非流式响应
- 支持工具调用转换
- 支持思考/推理内容转换
- 后续可扩展其他格式（OpenAI Responses、Gemini 等）

**代码实现**

| 模块 | 文件路径 | 关键函数/方法 |
|------|----------|---------------|
| 转换器接口 | `internal/transformer/transformer.go` | `Transformer` 接口 |
| 类型定义 | `internal/transformer/types.go` | `OpenAIRequest`, `ClaudeRequest` |
| Claude → OpenAI | `internal/transformer/convert/claude_openai.go` | `ClaudeReqToOpenAI()`, `OpenAIToClaudeResp()` |
| 转换器注册 | `internal/transformer/registry.go` | 转换器注册和管理 |
| 流式处理 | `internal/proxy/streaming.go` | `handleStreamingResponse()` |
| 请求准备 | `internal/proxy/request.go` | `prepareTransformerForClient()` |

**关键逻辑**
```go
// 转换器选择（request.go:34）
func prepareTransformerForClient(clientFormat ClientFormat, endpoint config.Endpoint, effectiveModel string) {
    switch clientFormat {
    case ClientFormatClaude:
        return prepareCCTransformer(endpoint, endpointTransformer, effectiveModel)
    case ClientFormatOpenAIChat:
        return prepareCxChatTransformer(endpoint, endpointTransformer, effectiveModel)
    }
}
```

**扩展性设计**
- 转换器接口抽象，便于后续添加新格式
- 插件式架构，支持动态注册转换器
- 配置驱动，通过端点配置选择转换器

**旧版参考（可扩展时参考）**
- Claude ↔ OpenAI Responses：`internal/transformer/convert/claude_openai2.go`
- Claude ↔ Gemini：`internal/transformer/convert/claude_gemini.go`
- OpenAI Chat ↔ OpenAI Responses：`internal/transformer/convert/openai_openai2.go`
- OpenAI Chat ↔ Gemini：`internal/transformer/convert/openai_gemini.go`
- OpenAI Responses ↔ Gemini：`internal/transformer/convert/openai2_gemini.go`

---

## 2. 实时统计

### 2.1 四周期统计

**功能描述**
- 今日、昨日、本周、本月统计
- 事件驱动零延迟更新
- 趋势对比分析
- 按端点统计

**代码实现**

| 模块 | 文件路径 | 关键函数/方法 |
|------|----------|---------------|
| 统计核心 | `internal/proxy/stats.go` | `RecordRequest()`, `RecordError()`, `RecordTokens()`, `emitStatsUpdate()` |
| 存储层 | `internal/storage/sqlite.go` | `RecordDailyStat()`, `GetDailyStats()`, `GetPeriodStatsAggregated()` |
| 前端统计 | `cmd/desktop/frontend/src/modules/stats.js` | `loadStatsByPeriod()`, `loadTrend()`, `updateEndpointStatsCache()` |

**关键逻辑**
```go
// 事件驱动更新（stats.go:162）
func (s *Stats) emitStatsUpdate(endpointName string) {
    today, yesterday, weekStart, monthStart := getPeriodDates()
    
    dailyStats, _ := s.storage.GetDailyStats(endpointName, today, today)
    yesterdayStats, _ := s.storage.GetDailyStats(endpointName, yesterday, yesterday)
    weeklyStats, _ := s.storage.GetDailyStats(endpointName, weekStart, today)
    monthlyStats, _ := s.storage.GetDailyStats(endpointName, monthStart, today)
    
    s.onStatsUpdated(endpointName, endpointPeriods, totalPeriods)
}
```

---

## 3. 数据存储

### 3.1 SQLite 数据库

**功能描述**
- WAL 模式支持并发读写
- 自动数据库迁移
- 设备 ID 唯一标识
- 配置和统计数据分开存储

**代码实现**

| 模块 | 文件路径 | 关键函数/方法 |
|------|----------|---------------|
| 存储核心 | `internal/storage/sqlite.go` | `NewSQLiteStorage()`, `initSchema()`, `RecordDailyStat()`, `GetDailyStats()` |
| 数据库迁移 | `internal/storage/sqlite.go` | `migrateSortOrder()`, `migrateAuthMode()` |
| 设备 ID | `internal/storage/sqlite.go` | `GetOrCreateDeviceID()`, `generateDeviceID()` |
| 配置管理 | `internal/storage/sqlite.go` | `GetConfig()`, `SetConfig()` |

**数据表结构**
```sql
-- 端点配置表
CREATE TABLE IF NOT EXISTS endpoints (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT UNIQUE NOT NULL,
    api_url TEXT NOT NULL,
    api_key TEXT NOT NULL,
    auth_mode TEXT NOT NULL DEFAULT 'api_key',
    enabled BOOLEAN DEFAULT TRUE,
    transformer TEXT DEFAULT 'claude',
    model TEXT,
    remark TEXT,
    sort_order INTEGER DEFAULT 0,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- 每日统计表
CREATE TABLE IF NOT EXISTS daily_stats (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    endpoint_name TEXT NOT NULL,
    date TEXT NOT NULL,
    requests INTEGER DEFAULT 0,
    errors INTEGER DEFAULT 0,
    input_tokens INTEGER DEFAULT 0,
    output_tokens INTEGER DEFAULT 0,
    device_id TEXT DEFAULT 'default',
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(endpoint_name, date, device_id)
);

-- 应用配置表
CREATE TABLE IF NOT EXISTS app_config (
    key TEXT PRIMARY KEY,
    value TEXT,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
```

**索引优化**
```sql
CREATE INDEX IF NOT EXISTS idx_daily_stats_date ON daily_stats(date);
CREATE INDEX IF NOT EXISTS idx_daily_stats_endpoint ON daily_stats(endpoint_name);
CREATE INDEX IF NOT EXISTS idx_daily_stats_device ON daily_stats(device_id);
```

---

## 4. WebDAV 同步

### 4.1 云同步功能

**功能描述**
- 多设备间同步配置和数据
- 数据库备份和恢复
- 备份文件管理
- 连接测试

**代码实现**

| 模块 | 文件路径 | 关键函数/方法 |
|------|----------|---------------|
| WebDAV 客户端 | `internal/webdav/client.go` | `NewClient()`, `TestConnection()`, `UploadBackup()`, `DownloadBackup()`, `ListBackups()`, `DeleteBackups()` |
| 同步管理器 | `internal/webdav/sync.go` | `BackupDatabase()`, `RestoreDatabase()`, `ListConfigBackups()`, `DeleteConfigBackups()` |
| 类型定义 | `internal/webdav/types.go` | `BackupFile`, `BackupData`, `ConflictInfo`, `TestResult` |
| 前端模块 | `cmd/desktop/frontend/src/modules/webdav.js` | WebDAV 配置和操作 |

**关键逻辑**
```go
// 备份数据库（sync.go:42-89）
func (m *Manager) BackupDatabase(dbPath string, version string, filename string) error {
    dbData, err := os.ReadFile(dbPath)
    metadata := &DatabaseBackupData{
        BackupTime: time.Now(),
        Version:    version,
    }
    m.client.UploadBackup(dbFilename, dbData, true)
    m.client.UploadBackup(metaFilename, metadataJSON, true)
}
```

**安全配置过滤**
```go
// 安全配置键（sqlite.go:22-39）
var safeConfigKeys = []string{
    "port", "logLevel", "language",
    "theme", "themeAuto", "autoLightTheme", "autoDarkTheme",
    "closeWindowBehavior",
    "webdav_url", "webdav_username", "webdav_password", "webdav_configPath", "webdav_statsPath",
    "backup_provider",
    "backup_s3_endpoint", "backup_s3_region", "backup_s3_bucket", "backup_s3_prefix",
    "backup_s3_accessKey", "backup_s3_secretKey", "backup_s3_sessionToken",
    "backup_s3_useSSL", "backup_s3_forcePathStyle",
    "update_autoCheck", "update_checkInterval",
}
```

---

## 5. 系统托盘

### 5.1 托盘功能

**功能描述**
- 跨平台支持（Windows/macOS/Linux）
- 多语言菜单
- 窗口行为配置

**代码实现**

| 模块 | 文件路径 | 关键函数/方法 |
|------|----------|---------------|
| Windows 托盘 | `internal/tray/tray_windows.go` | `Setup()`, `onReady()`, `onExit()` |
| macOS 托盘 | `internal/tray/tray_darwin.go` | `Setup()`, `onReady()`, `onExit()` |
| 其他平台 | `internal/tray/tray_other.go` | `Setup()` |
| 图标 | `internal/tray/icon.go` | 托盘图标数据 |

**关键逻辑**
```go
// Windows 托盘（tray_windows.go:46-60）
func Setup(icon []byte, showFunc func(), hideFunc func(), quitFunc func(), language string) {
    go func() {
        runtime.LockOSThread()
        systray.Run(func() {
            onReady(icon)
        }, onExit)
    }()
}
```

---

## 6. 主题系统

### 6.1 主题管理

**功能描述**
- 12种主题可选
- 自动主题切换（根据时间）
- 浅色/深色主题时间配置

**代码实现**

| 模块 | 文件路径 | 关键函数/方法 |
|------|----------|---------------|
| 主题配置 | `internal/storage/sqlite.go` | `safeConfigKeys` 中的主题相关配置 |
| 前端主题 | `cmd/desktop/frontend/src/themes/` | 主题样式文件 |
| 主题切换 | `cmd/desktop/frontend/src/modules/settings.js` | `setTheme()`, `setAutoTheme()` |

---

## 7. 模型列表 API

### 7.1 模型管理

**功能描述**
- 带缓存的模型列表
- 支持按需刷新
- 多端点类型支持

**代码实现**

| 模块 | 文件路径 | 关键函数/方法 |
|------|----------|---------------|
| 模型缓存 | `internal/proxy/models.go` | `ModelsCache`, `Get()`, `Set()`, `Clear()` |
| 模型获取 | `internal/proxy/models.go` | `fetchModelsFromEndpoint()`, `getDefaultModels()` |
| 模型 API | `internal/proxy/handler.go` | `handleModels()` |

**关键逻辑**
```go
// 模型缓存（models.go:25-31）
type ModelsCache struct {
    data      []ModelInfo
    updatedAt time.Time
    ttl       time.Duration
    mu        sync.RWMutex
}

// 默认 TTL：30分钟
func NewModelsCache(ttlMinutes int) *ModelsCache {
    if ttlMinutes <= 0 {
        ttlMinutes = 30
    }
}
```

---

## 8. 健康检查

### 8.1 健康状态

**功能描述**
- 端点状态检查
- API Key 脱敏显示
- 状态监控

**代码实现**

| 模块 | 文件路径 | 关键函数/方法 |
|------|----------|---------------|
| 健康检查 | `internal/proxy/handler.go` | `handleHealth()` |
| API Key 脱敏 | `internal/proxy/handler.go` | `maskAPIKey()` |

**关键逻辑**
```go
// 健康检查（handler.go:14-34）
func (p *Proxy) handleHealth(w http.ResponseWriter, r *http.Request) {
    response := map[string]interface{}{
        "status":            "healthy",
        "enabled_endpoints": len(endpoints),
        "endpoints":         maskedEndpoints,
    }
}

// API Key 脱敏（handler.go:37-45）
func maskAPIKey(key string) string {
    if len(key) <= 8 {
        return "****"
    }
    return key[:4] + strings.Repeat("*", len(key)-8) + key[len(key)-4:]
}
```

---

## 9. Token 计数 API

### 9.1 Token 估算

**功能描述**
- 请求 Token 估算
- 系统提示 Token 计数
- 消息内容 Token 计数

**代码实现**

| 模块 | 文件路径 | 关键函数/方法 |
|------|----------|---------------|
| Token 计数 | `internal/proxy/handler.go` | `handleCountTokens()` |
| Token 估算 | `internal/tokencount/` | `EstimateInputTokens()`, `EstimateOutputTokens()` |

---

## 10. 多语言支持

### 10.1 国际化

**功能描述**
- 中文/英文支持
- 动态语言切换
- 翻译键值对

**代码实现**

| 模块 | 文件路径 | 关键函数/方法 |
|------|----------|---------------|
| i18n 核心 | `cmd/desktop/frontend/src/i18n/index.js` | `setLanguage()`, `getLanguage()`, `t()` |
| 英文翻译 | `cmd/desktop/frontend/src/i18n/en.js` | 英文翻译文件 |
| 中文翻译 | `cmd/desktop/frontend/src/i18n/zh-CN.js` | 中文翻译文件 |

**关键逻辑**
```javascript
// 翻译函数（i18n/index.js:24-50）
export function t(key) {
    const keys = key.split('.');
    let value = translations[currentLanguage] || translations[defaultLanguage];
    
    for (const k of keys) {
        if (value && typeof value === 'object') {
            value = value[k];
        } else {
            value = undefined;
            break;
        }
    }
    
    return value || key;
}
```

---

## 11. 端点管理

### 11.1 端点筛选

**功能描述**
- 按类型筛选
- 按可用性筛选
- 按启用状态筛选
- 筛选状态持久化

**代码实现**

| 模块 | 文件路径 | 关键函数/方法 |
|------|----------|---------------|
| 筛选逻辑 | `cmd/desktop/frontend/src/modules/filters.js` | `getFilterState()`, `isFilterActive()`, `clearAllFilters()`, `applyFilters()` |
| 筛选 UI | `cmd/desktop/frontend/src/modules/filters.js` | `initFilterDropdowns()`, `togglePanel()` |

**关键逻辑**
```javascript
// 筛选状态（filters.js:6-10）
let filterState = {
    types: [],            // ['claude', 'gemini', 'openai', 'openai2']
    availabilities: [],   // ['available', 'unknown', 'unavailable']
    enabledStates: []     // ['enabled', 'disabled']
};
```

### 11.2 端点克隆

**功能描述**
- 一键克隆端点
- 自动命名（添加副本后缀）

**代码实现**

| 模块 | 文件路径 | 关键函数/方法 |
|------|----------|---------------|
| 克隆逻辑 | `cmd/desktop/frontend/src/modules/endpoints.js` | `extractBaseName()` |

**关键逻辑**
```javascript
// 克隆命名（endpoints.js:8-18）
function extractBaseName(name) {
    const copyPattern = /\(Copy\)(?:\s+\d+)?$/;
    const chineseCopyPattern = /\(副本\)(?:\s+\d+)?$/;
    
    let baseName = name.replace(copyPattern, '').trim();
    baseName = baseName.replace(chineseCopyPattern, '').trim();
    
    return baseName;
}
```

### 11.3 端点测试

**功能描述**
- 测试端点可用性
- 测试状态管理（成功/失败/未测试）
- 测试结果持久化

**代码实现**

| 模块 | 文件路径 | 关键函数/方法 |
|------|----------|---------------|
| 测试状态 | `cmd/desktop/frontend/src/modules/endpoints.js` | `getEndpointTestStatus()`, `saveEndpointTestStatus()` |
| 测试 API | `cmd/server/webui/api/testing.go` | 端点测试 API |

**关键逻辑**
```javascript
// 测试状态（endpoints.js:24-42）
export function getEndpointTestStatus(endpointName) {
    try {
        const statusMap = JSON.parse(localStorage.getItem(ENDPOINT_TEST_STATUS_KEY) || '{}');
        return statusMap[endpointName]; // true=成功, false=失败, undefined=未测试
    } catch {
        return undefined;
    }
}
```

---

## 12. 自动更新

### 12.1 更新检查

**功能描述**
- 自动检查更新
- 配置检查间隔
- 更新红点提示
- 跳过特定版本
- 下载进度显示

**代码实现**

| 模块 | 文件路径 | 关键函数/方法 |
|------|----------|---------------|
| 更新逻辑 | `cmd/desktop/frontend/src/modules/updater.js` | `checkUpdatesOnStartup()`, `checkForUpdates()`, `startAutoCheck()`, `stopAutoCheck()` |
| 更新 API | `cmd/desktop/app.go` | `CheckForUpdates()`, `GetUpdateSettings()`, `SetUpdateSettings()`, `SkipVersion()`, `DownloadUpdate()` |

**关键逻辑**
```javascript
// 更新检查（updater.js:29-60）
export async function checkUpdatesOnStartup() {
    const settingsStr = await GetUpdateSettings();
    const settings = JSON.parse(settingsStr);
    
    if (settings.checkInterval === 0) {
        stopAutoCheck();
        return;
    }
    
    if (settings.lastCheckTime) {
        const lastCheck = new Date(settings.lastCheckTime);
        const now = new Date();
        const hoursSinceCheck = (now - lastCheck) / (1000 * 60 * 60);
        
        if (hoursSinceCheck < settings.checkInterval) {
            startAutoCheck(settings.checkInterval);
            return;
        }
    }
    
    await checkForUpdates(true);
    startAutoCheck(settings.checkInterval);
}
```

---

## 13. 其他功能

### 13.1 日志系统

**功能描述**
- 实时日志查看
- 日志级别配置
- 调试日志文件

**代码实现**

| 模块 | 文件路径 | 关键函数/方法 |
|------|----------|---------------|
| 日志核心 | `internal/logger/logger.go` | `Info()`, `Warn()`, `Error()`, `Debug()`, `EnableDebugFile()` |
| 前端日志 | `cmd/desktop/frontend/src/modules/logs.js` | 日志显示模块 |

### 13.2 历史记录

**功能描述**
- 端点使用历史
- 月度归档数据
- 数据清理

**代码实现**

| 模块 | 文件路径 | 关键函数/方法 |
|------|----------|---------------|
| 历史查询 | `internal/storage/sqlite.go` | `GetAllStats()`, `GetArchiveMonths()`, `GetMonthlyArchiveData()` |
| 数据清理 | `internal/storage/sqlite.go` | `DeleteMonthlyStats()` |
| 前端历史 | `cmd/desktop/frontend/src/modules/history.js` | 历史显示模块 |

---

## 14. 服务器模式（不支持）

**说明**
- 本重构仅支持桌面模式，不支持服务器模式
- 服务器模式相关代码不纳入重构范围
- 旧版服务器模式代码仅供参考

**旧版参考（仅供参考）**
- 服务器入口：`cmd/server/main.go`
- Web UI：`cmd/server/webui/`
- API 端点：`cmd/server/webui/api/`
- Docker：`cmd/server/Dockerfile`

---

## 15. 配置管理

### 15.1 应用配置

**功能描述**
- 端口配置
- 日志级别配置
- 语言配置
- 主题配置
- 窗口行为配置

**代码实现**

| 模块 | 文件路径 | 关键函数/方法 |
|------|----------|---------------|
| 配置管理 | `internal/config/config.go` | `Config` 结构体, `GetPort()`, `GetLogLevel()`, `Validate()` |
| 配置存储 | `internal/storage/sqlite.go` | `GetConfig()`, `SetConfig()` |
| 环境变量 | `cmd/server/main.go` | `applyEnvOverrides()` |

**环境变量**
```go
// 环境变量（cmd/server/main.go）
CCNEXUS_PORT          // 覆盖默认端口
CCNEXUS_LOG_LEVEL     // 日志级别
CCNEXUS_DB_PATH       // 数据库路径
CCNEXUS_DATA_DIR      // 数据目录
CCNEXUS_BASIC_AUTH_USERNAME  // Basic Auth 用户名
CCNEXUS_BASIC_AUTH_PASSWORD  // Basic Auth 密码
```

---

## 16. API 端点

### 16.1 代理端点

**功能描述**
- 主代理路由
- Token 计数
- 模型列表
- 健康检查
- 统计数据

**代码实现**

| 端点 | 文件路径 | 处理函数 |
|------|----------|----------|
| `/` | `internal/proxy/proxy.go` | `handleProxy()` |
| `/v1/messages/count_tokens` | `internal/proxy/handler.go` | `handleCountTokens()` |
| `/v1/models` | `internal/proxy/handler.go` | `handleModels()` |
| `/health` | `internal/proxy/handler.go` | `handleHealth()` |
| `/stats` | `internal/proxy/handler.go` | `handleStats()` |

**路由注册**
```go
// 路由注册（proxy.go:108-114）
mux.HandleFunc("/", p.handleProxy)
mux.HandleFunc("/v1/messages/count_tokens", p.handleCountTokens)
mux.HandleFunc("/v1/models", p.handleModels)
mux.HandleFunc("/health", p.handleHealth)
mux.HandleFunc("/stats", p.handleStats)
```

---

## 17. 性能优化

### 17.1 缓存策略

**功能描述**
- 模型列表缓存（30分钟）
- 统计数据防抖保存（2秒）
- 连接池优化

**代码实现**

| 模块 | 文件路径 | 关键函数/方法 |
|------|----------|---------------|
| 模型缓存 | `internal/proxy/models.go` | `ModelsCache` |
| 统计防抖 | `internal/proxy/stats.go` | `scheduleSave()` |
| 连接池 | `internal/proxy/proxy.go` | `httpClient` 配置 |

**关键逻辑**
```go
// 连接池配置（proxy.go:58-71）
httpClient := &http.Client{
    Timeout: 300 * time.Second,
    Transport: &http.Transport{
        MaxIdleConns:           100,
        MaxIdleConnsPerHost:    10,
        IdleConnTimeout:        90 * time.Second,
        TLSHandshakeTimeout:    10 * time.Second,
        ExpectContinueTimeout:  1 * time.Second,
        ResponseHeaderTimeout:  90 * time.Second,
        WriteBufferSize:        128 * 1024,
        ReadBufferSize:         128 * 1024,
        MaxResponseHeaderBytes: 64 * 1024,
    },
}
```

---

## 18. 安全考虑

### 18.1 安全措施

**功能描述**
- API Key 脱敏显示
- 安全配置过滤
- 输入验证
- SQL 注入防护

**代码实现**

| 模块 | 文件路径 | 关键函数/方法 |
|------|----------|---------------|
| API Key 脱敏 | `internal/proxy/handler.go` | `maskAPIKey()` |
| SQL 注入防护 | `internal/storage/sqlite.go` | `escapeSQLString()` |
| 安全配置 | `internal/storage/sqlite.go` | `safeConfigKeys` |
| 输入验证 | `internal/config/config.go` | `Validate()` |

**关键逻辑**
```go
// SQL 注入防护：使用参数化查询（? 占位符），不要手动转义
// 正确做法
row := db.QueryRow("SELECT * FROM endpoints WHERE name = ?", endpointName)

// 错误做法（不安全）
// query := fmt.Sprintf("SELECT * FROM endpoints WHERE name = '%s'", name)
```

---

## 19. 测试覆盖

### 19.1 测试文件

**功能描述**
- 单元测试
- 集成测试
- 边界情况测试

**代码实现**

| 测试类型 | 文件路径 | 测试内容 |
|----------|----------|----------|
| 转换器测试 | `internal/transformer/convert/*_test.go` | 各种格式转换 |
| 代理测试 | `internal/proxy/*_test.go` | 代理逻辑 |
| 流式测试 | `internal/proxy/streaming_usage_test.go` | 流式响应处理 |
| Token 提取测试 | `internal/proxy/token_extraction_test.go` | Token 提取逻辑 |

---

## 20. 部署配置

### 20.1 构建和部署

**功能描述**
- 桌面应用构建
- 服务器构建
- Docker 部署

**代码实现**

| 构建类型 | 命令 | 输出 |
|----------|------|------|
| 桌面应用 | `cd cmd/desktop && wails build` | 跨平台 GUI 应用 |
| 服务器 | `cd cmd/server && go build` | 无头 HTTP 代理 |
| Docker | `docker build -f cmd/server/Dockerfile` | Docker 镜像 |

**环境变量**
```bash
# 桌面模式环境变量（可选）
CCNEXUS_PORT=3000
CCNEXUS_LOG_LEVEL=1
CCNEXUS_DB_PATH=~/.ccNexus/ccnexus.db
CCNEXUS_DATA_DIR=~/.ccNexus
```

---

## 21. 依赖管理

### 21.1 主要依赖

**后端依赖**
- Go 1.24+
- SQLite（modernc.org/sqlite）
- WebDAV 客户端（studio-b12/gowebdav）
- 系统托盘（energye/systray）

**前端依赖**
- 具体框架待定
- 国际化（i18n）
- 主题系统

**桌面框架**
- 具体技术栈待定（如：Wails、Electron、Tauri 等）

---

## 22. 文档参考

### 22.1 项目文档

| 文档 | 路径 | 内容 |
|------|------|------|
| README | `README.md` | 项目介绍和快速开始 |
| CLAUDE.md | `CLAUDE.md` | 开发指南和架构说明 |
| 配置文档 | `docs/configuration.md` | 详细配置说明 |
| 开发文档 | `docs/development.md` | 开发指南 |
| FAQ | `docs/FAQ.md` | 常见问题 |
| Docker 文档 | `docs/README_DOCKER.md` | Docker 部署 |
| 英文文档 | `docs/README_EN.md` | 英文 README |

---

## 23. 功能与实现对照表

| 功能模块 | PRD 用户故事 | 实现文件 | 关键函数 |
|----------|--------------|----------|----------|
| 多端点轮换 | 1-10 | `internal/proxy/proxy.go`, `internal/proxy/proxy_request.go` | `rotateEndpoint()`, `handleProxyRequest()` |
| API 格式转换 | 11-15 | `internal/transformer/convert/` | 各种转换函数 |
| 实时统计 | 16-25 | `internal/proxy/stats.go`, `internal/storage/sqlite.go` | `RecordRequest()`, `emitStatsUpdate()` |
| 数据存储 | 26-30 | `internal/storage/sqlite.go` | `NewSQLiteStorage()`, `initSchema()` |
| WebDAV 同步 | 31-39 | `internal/webdav/` | `BackupDatabase()`, `RestoreDatabase()` |
| 系统托盘 | 40-44 | `internal/tray/` | `Setup()` |
| 主题系统 | 45-48 | `cmd/desktop/frontend/src/themes/` | `setTheme()`, `setAutoTheme()` |
| 模型列表 API | 49-52 | `internal/proxy/models.go` | `ModelsCache`, `fetchModelsFromEndpoint()` |
| 健康检查 | 53-55 | `internal/proxy/handler.go` | `handleHealth()` |
| Token 计数 | 56-58 | `internal/proxy/handler.go` | `handleCountTokens()` |
| 多语言支持 | 59-61 | `cmd/desktop/frontend/src/i18n/` | `setLanguage()`, `t()` |
| 端点筛选 | 62-64 | `cmd/desktop/frontend/src/modules/filters.js` | `getFilterState()`, `applyFilters()` |
| 端点克隆 | 65-66 | `cmd/desktop/frontend/src/modules/endpoints.js` | `extractBaseName()` |
| 端点测试 | 67-70 | `cmd/desktop/frontend/src/modules/endpoints.js` | `getEndpointTestStatus()` |
| 自动更新 | 71-75 | `cmd/desktop/frontend/src/modules/updater.js` | `checkUpdatesOnStartup()` |
| 其他功能 | 76-80 | 各相关模块 | 各相关函数 |

---

## 24. 开发指南

### 24.1 开发环境

**桌面应用开发**
```bash
cd cmd/desktop && wails dev
```

**服务器开发**
```bash
cd cmd/server && go run main.go
```

**测试**
```bash
go test ./... -count=1
```

### 24.2 代码规范

**命名规范**
```go
// 包名：小写单词，不使用下划线
package proxy

// 结构体：大写驼峰
type EndpointConfig struct {
    Name    string
    APIURL  string
    APIKey  string
}

// 函数/方法：大写驼峰（导出），小写驼峰（未导出）
func (p *Proxy) RotateEndpoint() { ... }
func (p *Proxy) getCurrentEndpoint() Endpoint { ... }

// 接口：大写驼峰，通常以 er 结尾
type Transformer interface {
    Transform(req *Request) (*Response, error)
}
```

**错误处理**
```go
// 显式错误检查，不忽略错误
result, err := doSomething()
if err != nil {
    return fmt.Errorf("doSomething failed: %w", err)
}
```

**变量声明**
```go
// 靠近使用处声明，不在函数开头堆砌
func handleRequest(r *http.Request) error {
    endpoint := resolveEndpoint(r)
    if !endpoint.Enabled {
        return ErrEndpointDisabled
    }

    resp, err := sendRequest(endpoint, r)
    if err != nil {
        return err
    }
    return writeResponse(resp)
}
```

---

## 25. 总结

ccNexus 是一个功能完善的 API 端点轮换代理，具有以下特点：

1. **模块化设计**：各功能模块独立，便于维护和扩展
2. **完整实现**：所有 PRD 功能都有对应的代码实现
3. **性能优化**：缓存、防抖、连接池等优化措施
4. **安全考虑**：API Key 脱敏、SQL 注入防护、输入验证
5. **用户体验**：多语言、主题、拖拽、筛选等便捷功能
6. **跨平台支持**：Windows、macOS、Linux 全平台支持
7. **文档完善**：详细的开发文档和用户文档

通过本索引文档，可以快速定位各功能的实现位置，便于开发和维护。
