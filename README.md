# AgentLink Chatbot Agent

A reference implementation of an AgentLink agent powered by DeepSeek AI.

## Features

- **Real-time messaging** via WebSocket events (`message.created`)
- **Per-conversation memory** with configurable history limit
- **Graceful shutdown** with SIGINT/SIGTERM handling
- **Agent availability** auto-managed (online on start, offline on stop)
- **Error resilience** — user-friendly error replies when DeepSeek fails
- **Message type filtering** — only responds to text messages

## Quick Start

### 1. Configure environment

```bash
cp .env.example .env
# Edit .env with your API keys
```

Required variables:
- `AGENTLINK_API_KEY` — Your agent's API key from AgentLink
- `DEEPSEEK_API_KEY` — Your DeepSeek API key

### 2. Run

```bash
cargo run
```

### 3. Build release binary

```bash
cargo build --release
# Binary: target/release/chatbot-agent
```

## Architecture

```
┌─────────────┐     WebSocket      ┌─────────────────┐
│  AgentLink  │ ◄────────────────► │   Chatbot Agent │
│   Platform  │    message.created │                 │
└─────────────┘                    │  ┌───────────┐  │
       ▲                           │  │  Memory   │  │
       │ REST API                  │  │  Store    │  │
       │ send_message              │  │ (per-conv)│  │
       │                           │  └─────┬─────┘  │
┌──────┴──────┐                    │        │        │
│   DeepSeek  │ ◄──────────────────┘  ┌─────┴─────┐  │
│     API     │   chat completions    │ DeepSeek  │  │
└─────────────┘                       │  Client   │  │
                                      └───────────┘  │
```

## SDK Usage

This agent demonstrates the following `agentlink-rust-sdk` patterns:

### Event-driven architecture
```rust
use agentlink_rust_sdk::event_handler::MESSAGE_CREATED;

client.on(MESSAGE_CREATED, |payload| async move {
    println!("New message: {}", payload.message.content);
});
```

### Per-conversation state
```rust
type MemoryStore = Arc<RwLock<HashMap<Uuid, Arc<Mutex<ChatMemory>>>>>;
```

### Graceful shutdown
```rust
tokio::signal::ctrl_c().await?;
client.agents.update_agent_availability(&agent_id, false).await?;
```

## License

MIT
