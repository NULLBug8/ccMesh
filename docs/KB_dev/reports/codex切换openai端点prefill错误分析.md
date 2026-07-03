# codex 端点切换到 openai chat 端点后 prefill 错误分析

> 分析对象：tauri-gateway（产品名 ccMesh）
> 技术栈：Tauri 2 + Rust（axum 网关）/ SQLite
> 文档日期：2026-06-26
> 涉及模块：`src-tauri/src/modules/transform/responses_chat.rs`、`src-tauri/src/modules/proxy/forward.rs`
> 当前状态：最优先级根因已处理（防御性修复），中/低优先级待跟进

---

## 1. 问题现象

客户端持续用 `/v1/responses` 向 codex 端点发送请求，多轮对话后中途切换到 openai chat 端点继续发 `/v1/responses`，上游返回 500 错误（SSE 流式行）：

```
data:{"error":{"code":"500","message":"{\"error\":\"prefill failed: unexpected end of data: line 1 column 104 (char 103)\"}","param":"","type":"Internal Server Error"}}
```

关键特征：
- 错误以 `data:{...}` 的 SSE 行格式返回，说明是**上游服务**在流式阶段抛出，不是网关自身逻辑。
- `prefill` 是上游推理引擎术语（KV-cache 预填充阶段），网关代码全文检索无 `prefill` 关键字。
- `unexpected end of data: line 1 column 104 (char 103)` 是典型的 **JSON 解析到第 103 字符遇到 EOF** 错误——某段单行 JSON 字符串被截断了。

---

## 2. 原因分析（为什么会发生）

### 2.1 网关是无状态代理，切换端点不迁移会话

全库检索 `previous_response_id` / `conversation_id` / `prefill` 均无匹配。网关不存储任何会话状态，每一请求的上下文完全来自客户端当次请求体。端点切换（`server.rs:switch_endpoint`）只改轮换指针 + 取消旧端点在途请求，**不迁移 codex 积累的上下文**。

### 2.2 codex 端点透传，openai 端点转换

入站识别与三态转发决策见 `forward.rs:296-353`、`forward.rs:542-608`：

| 场景 | 上游路径 | 请求体处理 |
| --- | --- | --- |
| codex 端点 + `/v1/responses` | `/v1/responses` 透传 | 原样转发（仅可能改 model） |
| openai 端点 + `/v1/responses` | 转成 `/v1/chat/completions` | `responses_request_to_chat()` 重建整个 body |

### 2.3 转换时 `function_call.arguments` 原样透传，不校验完整性

`responses_chat.rs:178-196`（修复前）把 Responses `input` 里的 `function_call.arguments` 字符串**直接塞进** Chat 的 `tool_calls[].function.arguments`，不做任何 JSON 完整性校验。

codex 多轮对话几乎必然累积大量 `function_call` / `function_call_output` input item。一旦客户端回传的 `arguments` 是流式累积未闭合的 JSON（例如 `{"path":"foo","content":"bar` 缺尾部 `}`），网关原样透传给 Chat 上游，上游在 prefill 阶段解析 `tool_calls` 的 arguments 字符串时，就会在第 103 字符处 EOF，报 `unexpected end of data: line 1 column 104 (char 103)`。

**错误信息 "line 1 column 104" 精确匹配"单行 JSON 字符串在某个字符处截断"的特征，与该路径高度吻合。**

### 2.4 为什么"先 codex 对话很多轮"是关键诱因

- codex 端点是透传路径，任何 codex 特有结构（reasoning、function_call、previous_response_id）都不会出问题，因为 codex 上游本来就认这个格式。
- 多轮对话让 `input` 历史里**累积了大量 function_call / reasoning / output item**。
- 一旦切换到 openai chat 端点，转换函数要处理这些累积的复杂 item，问题同时暴露。
- 第一轮就发 openai chat 时 input 很简单（可能就一个 user string），不会触发。

**问题不是"切换"这个动作本身，而是切换时客户端携带的 codex 风格历史 input 在转换函数里没有得到正确处理。**

### 2.5 其它可疑点（尚未处理）

| 优先级 | 根因 | 说明 |
| --- | --- | --- |
| 中 | `reasoning` 类型 input item 被静默丢弃 | `responses_chat.rs:207-208` 的 `_ => {}` 跳过 reasoning item，多轮 reasoning 上下文丢失，可能导致 messages 结构与上游预期不一致 |
| 中 | `previous_response_id` 被丢弃 + 客户端只发增量 | 转换函数白名单式写入，不读 `previous_response_id`/`store`/`conversation`；客户端若依赖服务端续接只发增量，切到 openai 后会断上下文 |
| 低 | 上游本身对转换后的 messages 序列格式挑剔 | 需抓包确认 |

---

## 3. 解决方法（已处理最优先级根因）

针对 `function_call.arguments` 残缺 JSON 被原样透传这一最高优先级根因，已在转换层加**完整性校验 + 降级保护**。

### 修改的文件

`src-tauri/src/modules/transform/responses_chat.rs`

### 核心改动

在 `convert_input_items` 的 `function_call` 分支，`arguments` 不再无条件原样透传，而是先做一次 JSON 解析：

- **合法 JSON** → 用 `canonical_json_string` 规范化（递归排序对象键）后输出，顺带提升上游 prefix-cache 命中率。
- **残缺/非法 JSON**（如流式累积未闭合的 `{"path":"foo","content":"bar`）→ 降级为 `{}` 并打 `tracing::warn!` 日志（含 `call_id`、`name`、解析错误、原串长度），避免上游 prefill 解析失败。

```rust
// responses_chat.rs:178-208（修复后）
Some("function_call") => {
    let call_id = item.get("call_id").or_else(|item.get("id"))...;
    let name = item.get("name")...;
    let args = item.get("arguments").and_then(|v| v.as_str()).unwrap_or("{}");
    let safe_args = match serde_json::from_str::<Value>(args) {
        Ok(parsed) => canonical_json_string(&parsed),
        Err(err) => {
            tracing::warn!(
                call_id = %call_id, name = %name, error = %err, args_len = args.len(),
                "function_call.arguments 非合法 JSON，已降级为 {{}} 以避免上游 prefill 解析失败"
            );
            "{}".to_string()
        }
    };
    if !call_id.is_empty() && !name.is_empty() {
        pending_calls.push(json!({
            "id": call_id, "type": "function",
            "function": { "name": name, "arguments": safe_args }
        }));
    }
}
```

### 测试

新增两个单测，全部 24 个模块测试通过：

- `input_array_function_call_malformed_arguments_downgraded_to_empty`：未闭合 JSON → 降级为 `{}`
- `input_array_function_call_valid_arguments_canonicalized`：`{"b":2,"a":1}` → 规范化为 `{"a":1,"b":2}`

---

## 4. 现状与后续需留意的不妥之处

### 4.1 本质是防御性修复，根因可能在客户端

本次修复能避免网关把残缺 arguments 透传给上游，但**`arguments` 残缺的源头可能在客户端**——客户端回传的 codex 历史里 `function_call.arguments` 本身就是流式累积未闭合的。后续需通过 warn 日志确认：

- 如果 warn 日志频繁出现 → 客户端组包有问题，需去客户端侧修。
- 如果修复后错误消失 → 确认就是这条路径。
- warn 日志字段 `args_len` 和 `error` 可定位是哪条 call、原串多长、断在哪。

### 4.2 降级为 `{}` 会丢失工具调用的真实参数语义

残缺 arguments 降级为 `{}` 后，上游能正常 prefill，但**该次工具调用的参数语义丢失了**——相当于发了个空参数的 function_call。对于"回放历史对话"场景影响较小（历史已发生过），但对于"客户端期望上游基于完整 arguments 继续推理"的场景会导致行为偏差。需留意这是否符合预期，是否应改为"残缺时直接拒绝该条 call 并告警"而非"降级为空"。

### 4.3 中优先级根因尚未处理，仍可能引发同类问题

- `reasoning` item 丢弃：多轮 codex 对话含 reasoning 时，转 Chat 后 messages 结构可能不完整。
- `previous_response_id` 丢弃：客户端若依赖服务端续接只发增量 input，切到 openai 后会断上下文，可能再次引发上游解析异常（不一定是 prefill，但属同类"上下文残缺"问题）。

建议后续在切换端点（codex → openai）时检测到不兼容上下文，给客户端明确提示，而非静默转换。

### 4.4 缺少出错请求的现场抓取能力

当前定位该类错误只能靠日志推断。建议后续在网关转发层加一个"请求体快照"调试开关（默认关），出错时把转换前后的 body（脱敏 api_key）落盘或写入 request_logs，便于复现上游解析类错误。

### 4.5 历史数据与本修复无关，但相关统计口径需注意

本次 prefill 修复不涉及 token 统计。但同日另有一处 usage 字段归一化修复（OpenAI `input_tokens` 含 cache_read 导致双重计算），见 `docs/archiving/用量对比.txt`。两者的共同点是"OpenAI 风格与 Claude 风格语义差异"，后续处理 OpenAI 相关字段时需留意这种"包含 vs 独立"的语义分歧。
