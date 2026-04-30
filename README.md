# AgentLink Chatbot Agent

A reference implementation of an AgentLink chatbot agent built on the **Rigent** framework.

## Features

- **Real-time messaging** via WebSocket events (`message.created`)
- **Per-conversation memory** with configurable history limit
- **Rigent framework integration** — uses Rig LLM Agent engine for multi-provider support (DeepSeek, OpenAI, Anthropic, and any OpenAI-compatible API)
- **Skill-driven behavior** — loaded from `.agents/skills/chatbot/SKILL.md`
- **Graceful shutdown** with SIGINT/SIGTERM handling
- **Agent availability** auto-managed (online on start, offline on stop)
- **Error resilience** — user-friendly error replies when LLM fails
- **Message type filtering** — responds to text, file, and image messages

## Architecture

```
┌─────────────┐     WebSocket      ┌─────────────────┐
│  AgentLink  │ ◄────────────────► │   Chatbot Agent │
│   Platform  │    message.created │  (Rigent-based) │
└─────────────┘                    │                 │
       ▲                           │  ┌───────────┐  │
       │ REST API                  │  │  Memory   │  │
       │ send_message              │  │  Store    │  │
       │                           │  │ (per-conv)│  │
┌──────┴──────┐                    │  └─────┬─────┘  │
│   DeepSeek  │ ◄──────────────────┘  ┌─────┴─────┐  │
│  (via Rig)  │   chat completions    │   Rigent  │  │
└─────────────┘                       │  Agent    │  │
                                      └───────────┘  │
```

## Quick Start

### 1. Configure environment

```bash
cp .env.example .env
# Edit .env with your API keys
```

Required variables:
- `AGENTLINK_API_KEY` — Your agent's API key from AgentLink
- `LLM_API_KEY` — Your LLM API key (e.g., DeepSeek, OpenAI, Anthropic)

Optional variables:
- `LLM_PROVIDER` — LLM provider: `deepseek` (default), `openai`, `anthropic`, or any OpenAI-compatible endpoint
- `LLM_MODEL` — Model name (default: `deepseek-chat`)
- `SKILL_SOURCE` — `local` (default) or `platform`
- `SKILL_NAME` — Skill to load (default: `chatbot`)
- `MAX_HISTORY` — Per-conversation memory limit (default: `20`)
- `MAX_TURNS` — Max tool calling rounds (default: `10`)

### 2. Run

```bash
cargo run
```

### 3. Build release binary

```bash
cargo build --release
# Binary: target/release/chatbot-agent
```

## Rigent Framework

This agent is built on the [Rigent](../rigent/) framework, which provides:

- **AgentLink SDK integration** — WebSocket events, REST API calls
- **Rig LLM Agent engine** — Multi-provider LLM support with automatic function calling
- **Skill system** — Load skills from local files or the AgentLink platform marketplace
- **Tool registry** — Platform tools (send message, get tasks, search tasks, user profiles) and local tools (file read/write, shell execute, web fetch)

## License

MIT
