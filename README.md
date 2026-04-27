# Chatbot Agent

基于 DeepSeek 的简单对话聊天机器人 Agent，运行在 AgentLink 平台上。

## 功能

- 通过 WebSocket 实时接收 AgentLink 消息事件
- 调用 DeepSeek API 生成 AI 回复（携带多轮对话上下文）
- 通过 AgentLink REST API 将回复发送回对话

## 快速开始

### 1. 配置环境变量

```bash
cp .env.example .env
# 编辑 .env，填入你的 API Key
```

### 2. 运行

```bash
cargo run
```

## 环境变量

| 变量 | 必填 | 说明 |
|------|------|------|
| `AGENTLINK_BASE_URL` | 否 | AgentLink API 地址，默认 `https://beta-api.agentlink.chat/` |
| `AGENTLINK_API_KEY` | 是 | Agent API Key（以 `sk_` 开头） |
| `DEEPSEEK_API_KEY` | 是 | DeepSeek API Key |
| `RUST_LOG` | 否 | 日志级别，默认 `chatbot_agent=info` |
| `MAX_HISTORY` | 否 | 每个对话保留的最近消息数，默认 `20` |

## 工作原理

1. 启动后通过 SDK 回调机制注册 `MESSAGE_CREATED` 事件处理器
2. 调用 `poll()` 进入事件循环
3. 收到消息后检查 `sender_id`，忽略自己发送的消息（防循环）
4. 将用户消息存入该 `conversation_id` 的短期记忆缓存
5. 调用 DeepSeek API (`deepseek-chat` 模型)，携带历史上下文
6. 收到 AI 回复后存入记忆，并通过 SDK 发送回复

## 构建发布版本

```bash
cargo build --release
# 可执行文件位于 target/release/chatbot-agent
```
