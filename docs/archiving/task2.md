问题：新建端点的时候，有测试模型连通性的步骤，试模型连通性的功能，前提条件是选择一个模型来测试连通性，一般是选中一个模型测试连通性，看看如何设计实现这个功能，在点击的检测的时候，弹出浮层，选择模型这样合适么，另外浮层查找相关组件radix-ui 或者  shadcn/ui 来实现，另外如果有其他的使用到了浮层也替换一个合适的。



关于本地代理出去的路由端点

/v1/models存在问题：

Anthropic端点的/v1/models拿不到数据

open ai 端点的/v1/models



读取下面的项目，选取一个最优的方案，解决Anthropic端点的/v1/models拿不到数据问题

### 其他两个项目如何处理 Anthropic `/v1/models`

#### 一、new-api（QuantumNous）

**核心逻辑（`controller/model.go`）：** 根据请求路由到的 API 类型（Anthropic / OpenAI / Gemini）分别返回不同格式。当客户端走 Anthropic 兼容端点时，返回 Anthropic 格式：



json

```json
// Anthropic 格式（走 /v1/models 且鉴权为 Anthropic 模式）
{
  "data": [
    {
      "id":           "claude-opus-4-6",
      "type":         "model",
      "display_name": "claude-opus-4-6",
      "created_at":   "2021-07-20T00:00:00Z"
    }
  ],
  "first_id": "claude-opus-4-6",
  "has_more":  false,
  "last_id":  "claude-haiku-4-5-20251001"
}
```

对应到 OpenAI 格式时，则是：



json

```json
{
  "success": true,
  "object":  "list",
  "data": [
    {
      "id":       "gpt-4o",
      "object":   "model",
      "created":  1626777600,
      "owned_by": "openai"
    }
  ]
}
```

**关键区别：**

- Anthropic 格式：字段用 `id` / `type` / `display_name` / `created_at`（字符串），顶层有 `first_id` / `last_id` / `has_more`
- OpenAI 格式：字段用 `id` / `object` / `created`（Unix 时间戳整数）/ `owned_by`，顶层是 `object: "list"` + `data`
- new-api 还额外加了 `success: true`（OpenAI 格式）

------

#### 二、sub2api（Wei-Shaw）

**核心逻辑（`backend/internal/handler/gateway_handler.go`）：** 根据 `platform`（`claude` / `openai`）自动判断返回哪种格式。

走 Claude 端点时：



json

```json
{
  "object": "list",
  "data": [
    {
      "id":           "claude-sonnet-4-5",
      "type":         "model",
      "display_name": "claude-sonnet-4-5",
      "created_at":   "2024-01-01T00:00:00Z"
    }
  ]
}
```

走 OpenAI 端点时：



json

```json
{
  "object": "list",
  "data": [
    {
      "id":           "gpt-4o",
      "object":       "model",
      "created":      1704067200,
      "owned_by":     "openai",
      "type":         "model",
      "display_name": "gpt-4o"
    }
  ]
}
```

------

#### 三、与官方 Anthropic `/v1/models` 的区别总结

| 字段               | 官方 Anthropic                               | new-api（Anthropic 模式）     | sub2api（Claude 模式）                    |
| ------------------ | -------------------------------------------- | ----------------------------- | ----------------------------------------- |
| 顶层结构           | `data` + `first_id` + `last_id` + `has_more` | 同官方                        | `object: "list"` + `data`（少了分页字段） |
| `created_at`       | RFC3339 实际时间                             | 固定 `"2021-07-20T00:00:00Z"` | 固定 `"2024-01-01T00:00:00Z"`             |
| `capabilities`     | 有（含 vision/tool_use 等）                  | **没有**                      | **没有**                                  |
| `max_input_tokens` | 有                                           | **没有**                      | **没有**                                  |
| `max_tokens`       | 有                                           | **没有**                      | **没有**                                  |
| 分页支持           | 有（query params）                           | 有字段，无真实分页            | 无分页字段                                |

两个项目都是**伪造的静态数据**，不会实际查询上游 Anthropic API 的模型列表，返回的是网关本身配置/启用的模型，功能字段（capabilities、token 限制）一律缺失。对于只需要知道"有哪些模型可用"的客户端（如 Claude Code）完全够用，但如果你的代码依赖 `max_input_tokens` 或 `capabilities` 字段就会拿不到数据。

