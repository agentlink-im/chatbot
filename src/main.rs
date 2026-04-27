use std::collections::HashMap;
use std::env;
use std::sync::Arc;

use anyhow::{Context, Result};
use agentlink_protocol::message::SendMessageRequest;
use agentlink_protocol::MessageType;
use agentlink_rust_sdk::event_handler::{CONNECTION_READY, ERROR, MESSAGE_CREATED};
use agentlink_rust_sdk::{AgentLinkClient, SdkConfig};
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, RwLock};
use tracing::{error, info, warn};
use uuid::Uuid;

// ===================================================================
// DeepSeek API Types
// ===================================================================

#[derive(Serialize, Deserialize, Debug, Clone)]
struct DeepSeekMessage {
    role: String,
    content: String,
}

#[derive(Serialize, Debug)]
struct DeepSeekRequest {
    model: String,
    messages: Vec<DeepSeekMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
}

#[derive(Deserialize, Debug)]
struct DeepSeekResponse {
    choices: Vec<DeepSeekChoice>,
}

#[derive(Deserialize, Debug)]
struct DeepSeekChoice {
    message: DeepSeekMessage,
}

// ===================================================================
// Conversation Memory (per-conversation locking)
// ===================================================================

#[derive(Clone, Debug)]
struct ChatMemory {
    history: Vec<DeepSeekMessage>,
    max_messages: usize,
}

impl ChatMemory {
    fn new(max_messages: usize) -> Self {
        Self {
            history: Vec::with_capacity(max_messages),
            max_messages,
        }
    }

    fn push(&mut self, role: &str, content: String) {
        self.history.push(DeepSeekMessage {
            role: role.to_string(),
            content,
        });
        if self.history.len() > self.max_messages {
            let excess = self.history.len() - self.max_messages;
            self.history.drain(0..excess);
        }
    }

    fn to_vec(&self) -> Vec<DeepSeekMessage> {
        self.history.clone()
    }
}

type MemoryStore = Arc<RwLock<HashMap<Uuid, Arc<Mutex<ChatMemory>>>>>;

async fn get_or_create_memory(
    store: &MemoryStore,
    conversation_id: Uuid,
    max_messages: usize,
) -> Arc<Mutex<ChatMemory>> {
    {
        let read_guard = store.read().await;
        if let Some(mem) = read_guard.get(&conversation_id) {
            return mem.clone();
        }
    }

    let mut write_guard = store.write().await;
    write_guard
        .entry(conversation_id)
        .or_insert_with(|| Arc::new(Mutex::new(ChatMemory::new(max_messages))))
        .clone()
}

// ===================================================================
// DeepSeek API Call
// ===================================================================

async fn call_deepseek(
    api_key: &str,
    memory_store: &MemoryStore,
    conversation_id: Uuid,
) -> Result<String> {
    let history = {
        let read_guard = memory_store.read().await;
        if let Some(mem) = read_guard.get(&conversation_id) {
            mem.lock().await.to_vec()
        } else {
            Vec::new()
        }
    };

    let mut messages = vec![DeepSeekMessage {
        role: "system".to_string(),
        content: "You are a helpful assistant on the AgentLink platform. Keep replies concise and friendly.".to_string(),
    }];
    messages.extend(history);

    let request_body = DeepSeekRequest {
        model: "deepseek-chat".to_string(),
        messages,
        max_tokens: Some(1024),
        temperature: Some(0.7),
    };

    let client = reqwest::Client::new();
    let response = client
        .post("https://api.deepseek.com/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await
        .context("Failed to send request to DeepSeek")?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response
            .text()
            .await
            .unwrap_or_else(|_| "<unknown>".to_string());
        anyhow::bail!("DeepSeek API returned {}: {}", status, text);
    }

    let deepseek_resp: DeepSeekResponse = response
        .json()
        .await
        .context("Failed to parse DeepSeek response")?;

    let reply = deepseek_resp
        .choices
        .into_iter()
        .next()
        .map(|c| c.message.content)
        .unwrap_or_else(|| "Sorry, I couldn't generate a response.".to_string());

    Ok(reply)
}

// ===================================================================
// Main
// ===================================================================

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            env::var("RUST_LOG")
                .unwrap_or_else(|_| "chatbot_agent=info,agentlink_rust_sdk=warn".into()),
        )
        .init();

    let base_url = env::var("AGENTLINK_BASE_URL")
        .unwrap_or_else(|_| "https://beta-api.agentlink.chat/".to_string());
    let api_key = env::var("AGENTLINK_API_KEY")
        .context("AGENTLINK_API_KEY environment variable is required")?;
    let deepseek_api_key = env::var("DEEPSEEK_API_KEY")
        .context("DEEPSEEK_API_KEY environment variable is required")?;

    let max_history: usize = env::var("MAX_HISTORY")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(20);

    info!(base_url = %base_url, max_history, "Starting chatbot agent");

    let client = AgentLinkClient::new(
        SdkConfig::new(&base_url).with_token(&api_key),
    )
    .context("Failed to create AgentLink client")?;

    // Authenticate and identify ourselves
    let me = client
        .users
        .get_current_user()
        .await
        .context("Failed to get current user")?;
    let my_user_id = me.id;
    info!(
        user_id = %my_user_id,
        linkid = %me.linkid,
        display_name = %me.display_name.unwrap_or_default(),
        "Agent authenticated"
    );

    // Set agent as available
    match client
        .agents
        .update_agent_availability(&my_user_id.to_string(), true)
        .await
    {
        Ok(_) => info!("Agent marked as available"),
        Err(e) => warn!(error = %e, "Failed to set agent availability"),
    }

    let memory_store: MemoryStore = Arc::new(RwLock::new(HashMap::new()));

    // Clone client for use inside callbacks (on() now takes &self, safe to clone after)
    let reply_client = client.clone();
    let ds_key = deepseek_api_key.clone();
    let mem_store = memory_store.clone();

    // Register message handler
    client.on(MESSAGE_CREATED, move |payload| {
        let client = reply_client.clone();
        let ds_key = ds_key.clone();
        let mem_store = mem_store.clone();
        let my_user_id = my_user_id;
        let max_history = max_history;

        async move {
            let msg = payload.message;
            let conversation_id = msg.conversation_id;

            // Ignore our own messages
            if msg.sender_id == my_user_id {
                return;
            }

            // Only respond to text messages
            if msg.kind != MessageType::Text {
                info!(
                    conversation_id = %conversation_id,
                    kind = ?msg.kind,
                    "Ignoring non-text message"
                );
                return;
            }

            info!(
                conversation_id = %conversation_id,
                sender_id = %msg.sender_id,
                sender_name = %msg.sender_name,
                content = %msg.content,
                "Received message"
            );

            // Store user message in memory
            let mem = get_or_create_memory(&mem_store, conversation_id, max_history).await;
            mem.lock().await.push("user", msg.content.clone());
            drop(mem);

            // Call DeepSeek and reply
            match call_deepseek(&ds_key, &mem_store, conversation_id).await {
                Ok(reply) => {
                    info!(reply = %reply, "DeepSeek replied");

                    // Store assistant reply in memory
                    let mem = get_or_create_memory(&mem_store, conversation_id, max_history).await;
                    mem.lock().await.push("assistant", reply.clone());
                    drop(mem);

                    let send_req = SendMessageRequest {
                        content: reply,
                        kind: Some(MessageType::Text),
                        metadata: None,
                        reply_to: Some(msg.id),
                    };

                    match client
                        .messages
                        .send_message(&conversation_id.to_string(), send_req)
                        .await
                    {
                        Ok(sent) => {
                            info!(message_id = %sent.id, "Reply sent successfully");
                        }
                        Err(e) => {
                            error!(error = %e, "Failed to send reply");
                        }
                    }
                }
                Err(e) => {
                    error!(error = %e, "DeepSeek API call failed");

                    // Inform the user that something went wrong
                    let error_reply = SendMessageRequest {
                        content: "Sorry, I'm having trouble responding right now. Please try again later.".to_string(),
                        kind: Some(MessageType::Text),
                        metadata: None,
                        reply_to: Some(msg.id),
                    };

                    if let Err(send_err) = client
                        .messages
                        .send_message(&conversation_id.to_string(), error_reply)
                        .await
                    {
                        error!(error = %send_err, "Failed to send error reply");
                    }
                }
            }
        }
    });

    // Register connection event handlers for observability
    client.on(CONNECTION_READY, |payload| async move {
        info!(
            user_id = %payload.user_id,
            linkid = %payload.linkid,
            "WebSocket connected and ready"
        );
    });

    client.on(ERROR, |payload| async move {
        error!(
            code = %payload.code,
            message = %payload.message,
            "WebSocket error received"
        );
    });

    // Run WebSocket event loop in a background task
    let mut poll_client = client.clone();
    let poll_handle = tokio::spawn(async move {
        info!("Entering WebSocket event poll loop...");
        if let Err(e) = poll_client.poll().await {
            error!(error = %e, "Event poll loop ended with error");
        }
    });

    // Wait for shutdown signal
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            info!("Received SIGINT, shutting down gracefully...");
        }
        _ = async {
            #[cfg(unix)]
            {
                let mut sigterm = tokio::signal::unix::signal(
                    tokio::signal::unix::SignalKind::terminate()
                ).expect("Failed to create SIGTERM handler");
                sigterm.recv().await;
            }
            #[cfg(not(unix))]
            {
                std::future::pending::<()>().await;
            }
        } => {
            info!("Received SIGTERM, shutting down gracefully...");
        }
        result = poll_handle => {
            if let Err(e) = result {
                error!(error = %e, "Poll task panicked");
            }
            info!("Poll loop ended, shutting down...");
        }
    }

    // Mark agent as unavailable before exiting
    match client
        .agents
        .update_agent_availability(&my_user_id.to_string(), false)
        .await
    {
        Ok(_) => info!("Agent marked as unavailable"),
        Err(e) => warn!(error = %e, "Failed to set agent unavailability"),
    }

    info!("Chatbot agent stopped");
    Ok(())
}
