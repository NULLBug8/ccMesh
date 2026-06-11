# gpt-5.3-codex 模型参数配置文档

> 数据来源：AI/ML API 官方文档 https://docs.aimlapi.com/api-references/text-models-llm/openai/gpt-5.3-codex
> （在线 WebFetch + 本地存档 HTML Grep 交叉校验）
> 用途：作为本网关实现 `/v1/responses` 路由与参数透传的依据。

---

## 1. 模型概述

| 项 | 值 |
| --- | --- |
| 模型 ID（请求体 `model` 字段） | `openai/gpt-5-3-codex` |
| 响应体回显的 `model` | `gpt-5.3-codex` |
| 定位 | 面向代码的 LLM，针对高精度编程任务与生产开发环境优化，用于代码生成、重构、调试。 |
| 端点 | `POST /v1/responses`（Responses API） |
| Base URL | `https://api.aimlapi.com` |
| 鉴权 | `Authorization: Bearer <YOUR_AIMLAPI_KEY>` |
| 上下文长度 / 最大上下文窗口 | **文档未明确给出具体数值**（仅在 `truncation` 说明中提到"模型上下文窗口大小"概念）。 |

**端点说明**：文档明确将 Chat Completions 定位为"较旧的、面向对话的接口"（legacy 遗留接口），将 Responses API 定位为"更新的统一接口"，意在"完全取代 chat completions"。**本模型在 AI/ML API 文档中仅提供 Responses API 的 schema**，不提供 Chat Completions 的入参定义。因此本网关 codex 端点应基于 Responses API 实现。

---

## 2. 完整请求参数表（Responses API）

> 必填字段：`["model", "input"]`

| 参数名 | 类型 | 必填 | 默认值 | 取值范围 / 可选值 | 说明 |
| --- | --- | --- | --- | --- | --- |
| `model` | string | 是 | — | `openai/gpt-5-3-codex` | 用于生成响应的模型 ID。 |
| `input` | string \| array | 是 | — | 文本字符串，或输入项数组 | 提供给模型的文本/图像/文件输入，用于生成响应。数组形式支持 message 对象（role: user/assistant/system/developer）、function call、reasoning item、MCP item 等。 |
| `background` | boolean | 否 | `false` | true / false | 是否在后台运行模型响应。 |
| `instructions` | string | 否 | `null` | — | 插入模型上下文的 system（或 developer）消息。使用 `previous_response_id` 续接对话时**不会被自动带入**。 |
| `include` | array (nullable) | 否 | `null` | `message.input_image.image_url`、`computer_call_output.output.image_url`、`reasoning.encrypted_content`、`code_interpreter_call.outputs` | 指定要在响应中额外包含的输出数据。 |
| `max_output_tokens` | integer | 否 | `null` | 最小 `16` | 生成响应（含可见输出 token 与 reasoning token）的 token 数量上限。 |
| `previous_response_id` | string (nullable) | 否 | `null` | — | 上一条响应的唯一 ID，用于构建多轮对话。 |
| `prompt` | object (nullable) | 否 | `null` | 需含 `id` | 引用提示词模板及其变量。子字段：`id`、`variables`、`version`。 |
| `store` | boolean (nullable) | 否 | `false` | true / false | 是否存储生成的响应以便后续通过 API 检索。（注：响应示例中实际回显为 `true`，请以实际接口行为为准。） |
| `stream` | boolean (nullable) | 否 | `false` | true / false | 为 true 时，响应数据通过 SSE（server-sent events）边生成边流式返回客户端。 |
| `text` | object | 否 | — | `format.type`: `text` / `json_object` / `json_schema`；`verbosity`: `low`/`medium`/`high` | 文本响应的配置项，可为纯文本或结构化 JSON。 |
| `truncation` | string | 否 | `disabled` | `auto` / `disabled` | 截断策略。`auto`：超出上下文窗口时丢弃中间输入项以适配；`disabled`：超出则返回 400 错误。 |
| `tools` | array | 否 | `[]` | function、web_search_preview、mcp、local_shell | 模型在生成响应时可调用的工具数组。 |
| `tool_choice` | string \| object | 否 | `auto` | `none` / `auto` / `required` / 对象 | 模型如何选择调用哪个（些）工具。`none`=不调用工具仅生成消息；`auto`=自行决定生成消息或调用工具。 |
| `parallel_tool_calls` | boolean (nullable) | 否 | `true` | true / false | 是否允许模型并行执行工具调用。 |
| `reasoning` | object (nullable) | 否 | `null` | 见子字段 | reasoning 模型的配置项（o 系列 / 推理模型）。 |
| `reasoning.effort` | string (nullable) | 否 | `null` | `low` / `medium` / `high`（响应中亦出现 `none`） | 约束推理模型的推理投入。越低响应越快、token 越少。 |
| `reasoning.summary` | string (nullable) | 否 | `null` | `auto` / `concise` / `detailed` | 模型推理过程的摘要，便于调试。 |
| `metadata` | object | 否 | `{}` | — | 附加到响应对象的键值元数据。 |

### 2.1 工具（tools）子参数

| 工具类型 | 必填子字段 | 可选子字段 |
| --- | --- | --- |
| `function` | `name`、`parameters`、`type` | `description`、`strict` |
| `web_search_preview` | `type` | `search_context_size`（`low`/`medium`/`high`，默认 `medium`）、`user_location` |
| `mcp` | `server_label`、`server_url`、`type` | `allowed_tools`、`headers`、`require_approval` |
| `local_shell` | `type` | — |

### 2.2 仅出现在响应体（非请求入参）的字段

以下字段在响应对象中回显，但**不属于本模型 Responses API 文档化的请求入参**，透传时需注意：
`temperature`（number, `0`–`2`, 响应默认 `1`）、`top_p`（number, 响应默认 `0.98`）、`frequency_penalty`（默认 `0`）、`presence_penalty`（默认 `0`）、`top_logprobs`（默认 `0`）、`service_tier`（`default`）、`prompt_cache_key`、`prompt_cache_retention`、`safety_identifier`、`max_tool_calls`、`usage`、`meta.usage`（`credits_used`、`usd_spent`）。

---

## 3. 完整请求体 JSON（全字段标注）

> 下方示例把所有**可选请求参数**都列出，并对每个字段标注类型与含义。
> 实际最小可用请求仅需 `model` + `input`（见 3.1）。

```jsonc
{
  // 【必填】string，模型 ID
  "model": "openai/gpt-5-3-codex",

  // 【必填】string | array，模型输入（文本，或输入项数组）
  "input": "Write a Python function to reverse a string.",

  // boolean，是否后台运行，默认 false
  "background": false,

  // string，系统/开发者指令，注入模型上下文；默认 null
  "instructions": "You are a senior software engineer. Reply with code only.",

  // array | null，额外包含的输出数据；默认 null
  "include": ["reasoning.encrypted_content"],

  // integer，输出 token 上限（含 reasoning token），最小 16；默认 null（不限）
  "max_output_tokens": 2048,

  // string | null，上一条响应 ID，用于多轮对话；默认 null
  "previous_response_id": null,

  // object | null，引用提示词模板；默认 null
  "prompt": {
    "id": "pmpt_xxx",            // string，模板 ID（必填于该对象内）
    "version": "1",             // string，模板版本
    "variables": {}             // object，模板变量
  },

  // boolean | null，是否存储响应以便检索；默认 false
  "store": false,

  // boolean | null，是否 SSE 流式返回；默认 false
  "stream": false,

  // object，文本响应配置
  "text": {
    "format": {
      // string，输出格式：text | json_object | json_schema
      "type": "text"
    },
    // string，输出详尽程度：low | medium | high
    "verbosity": "medium"
  },

  // string，截断策略：auto | disabled；默认 disabled
  "truncation": "disabled",

  // array，可调用工具列表；默认 []
  "tools": [
    {
      "type": "function",        // string，工具类型（必填）
      "name": "get_weather",     // string，函数名（必填）
      "description": "Get weather", // string，函数描述（可选）
      "parameters": {},          // object，JSON Schema 参数（必填）
      "strict": true             // boolean，严格模式（可选）
    }
  ],

  // string | object，工具选择策略：none | auto | required | {对象}；默认 auto
  "tool_choice": "auto",

  // boolean | null，是否允许并行工具调用；默认 true
  "parallel_tool_calls": true,

  // object | null，推理配置（推理/o 系列模型）；默认 null
  "reasoning": {
    // string，推理投入：low | medium | high；默认 null
    "effort": "medium",
    // string，推理摘要：auto | concise | detailed；默认 null
    "summary": "auto"
  },

  // object，附加元数据键值对；默认 {}
  "metadata": {}
}
```

### 3.1 文档官方最小请求示例（cURL，来自本地存档）

```bash
curl --request POST \
  --url 'https://api.aimlapi.com/v1/responses' \
  --header 'Authorization: Bearer <YOUR_AIMLAPI_KEY>' \
  --header 'Content-Type: application/json' \
  --data '{
      "model": "openai/gpt-5-3-codex",
      "input": "Hello"
  }'
```

### 3.2 响应体形态（节选，来自本地存档，含默认值回显）

```jsonc
{
  "id": "hWzBc3WV-TVdl26mHW94Q",
  "object": "response",
  "status": "completed",
  "model": "gpt-5.3-codex",
  "output": [
    {
      "type": "message",
      "role": "assistant",
      "content": [
        { "type": "output_text", "text": "Hello! How can I help you today?", "annotations": [], "logprobs": [] }
      ],
      "phase": "final_answer"
    }
  ],
  "instructions": null,
  "max_output_tokens": null,
  "max_tool_calls": null,
  "parallel_tool_calls": true,
  "frequency_penalty": 0,
  "presence_penalty": 0,
  "reasoning": { "effort": "none", "summary": null },
  "service_tier": "default",
  "store": true,
  "temperature": 1,
  "text": { "format": { "type": "text" }, "verbosity": "medium" },
  "tool_choice": "auto",
  "tools": [],
  "top_logprobs": 0,
  "top_p": 0.98,
  "truncation": "disabled",
  "metadata": {},
  "usage": {
    "input_tokens": 7,
    "input_tokens_details": { "cached_tokens": 0 },
    "output_tokens": 15,
    "output_tokens_details": { "reasoning_tokens": 0 }
  }
}
```

---

## 4. Responses API 与 Chat Completions API 的参数差异

AI/ML API 文档对本模型**只提供 Responses API 的 schema**，未给出 Chat Completions 的字段定义。文档对两者的定位区分如下：

- **Chat Completions（legacy）**：发送 `messages` 列表，返回单条响应，面向对话工作流；为遗留接口。
- **Responses API（推荐）**：统一接口，支持多种输入类型（文本/图像/音频/工具）与多种输出模态，意在完全取代 Chat Completions。

关键入参映射差异：

| 语义 | Chat Completions（旧） | Responses API（本模型使用） |
| --- | --- | --- |
| 输入内容 | `messages`（数组） | `input`（string 或数组） |
| 系统/开发者指令 | system role 消息混在 `messages` 中 | 独立的 `instructions` 字段 |
| 输出长度上限 | `max_tokens` / `max_completion_tokens` | `max_output_tokens` |
| 输出格式 | `response_format` | `text.format`（`text`/`json_object`/`json_schema`） |
| 推理强度 | `reasoning_effort`（顶层） | `reasoning.effort`（嵌套于 `reasoning` 对象） |
| 多轮续接 | 客户端自行拼接 `messages` | `previous_response_id` 服务端续接 |
| 流式 | `stream` | `stream`（一致） |

> 透传实现提示：以下 Chat Completions 参数**未出现**在本模型 Responses 文档中，网关侧不应假定可直接透传：`messages`、`max_tokens`、`max_completion_tokens`、`n`、`stop`、`presence_penalty`、`frequency_penalty`、`logit_bias`、`seed`、`response_format`、`user`、`logprobs`/`top_logprobs`（后两者仅作为响应内的输出结构出现，非请求入参）。`temperature`、`top_p` 仅在响应对象中回显，文档未将其列为 Responses 请求入参。

---

## 附：数据完整度说明

- 在线 WebFetch 完整返回了 Responses API 的请求参数 schema（第 2 节）。
- 在线返回中**缺失**完整请求体示例与上下文窗口数值；已用本地存档 HTML 的 Grep 结果补全官方 cURL 最小示例（3.1）与响应体默认值（3.2）。
- **上下文长度具体数值**两个来源均未给出，需后续从 OpenAI 官方或 AI/ML API 模型列表另行确认。
- 第 3 节"全字段标注 JSON"为依据参数表手工组装（用户明确要求列全可选参数），官方原文未提供该完整示例。
