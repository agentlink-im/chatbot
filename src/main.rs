use std::env;

use anyhow::Result;
use rigent::agentlink_protocol::file::FileAttachment;
use rigent::agentlink_protocol::MessageType;
use rigent::agentlink_rust_sdk::event_handler::{CONNECTION_READY, ERROR, MESSAGE_CREATED};
use rigent::{config::FrameworkConfig, framework::AgentFramework};
use tracing::{error, info};

// ===================================================================
// Message Handlers
// ===================================================================

async fn handle_text_message(
    framework: &AgentFramework,
    msg: rigent::agentlink_protocol::message::MessageResponse,
) {
    let conversation_id = msg.conversation_id.to_string();

    info!(
        conversation_id = %conversation_id,
        sender_id = %msg.sender_id,
        sender_name = %msg.sender_name,
        content = %msg.content,
        "Received text message"
    );

    // Use Rigent's native conversation memory via framework.chat()
    match framework.chat(&conversation_id, &msg.content).await {
        Ok(reply) => {
            info!(reply = %reply, "Agent replied");

            if let Err(e) = framework
                .send_reply(&conversation_id, reply, Some(msg.id))
                .await
            {
                error!(error = %e, "Failed to send reply");
            }
        }
        Err(e) => {
            error!(error = %e, "Agent chat failed");

            let error_reply =
                "Sorry, I'm having trouble responding right now. Please try again later."
                    .to_string();
            if let Err(send_err) = framework
                .send_reply(&conversation_id, error_reply, Some(msg.id))
                .await
            {
                error!(error = %send_err, "Failed to send error reply");
            }
        }
    }
}

async fn handle_file_message(
    framework: &AgentFramework,
    msg: rigent::agentlink_protocol::message::MessageResponse,
) {
    let conversation_id = msg.conversation_id.to_string();

    info!(
        conversation_id = %conversation_id,
        sender_id = %msg.sender_id,
        sender_name = %msg.sender_name,
        "Received file message"
    );

    // Parse file attachment from metadata
    let attachment: Option<FileAttachment> = msg
        .metadata
        .as_ref()
        .and_then(|m| serde_json::from_value(m.clone()).ok());

    let file_desc = match &attachment {
        Some(att) => format!("file '{}' ({} bytes)", att.filename, att.size),
        None => "a file".to_string(),
    };

    info!(file_desc = %file_desc, "File details parsed");

    let ack = format!(
        "I've received {}. If you'd like me to analyze it or have questions about it, just let me know!",
        file_desc
    );

    if let Err(e) = framework
        .send_reply(&conversation_id, ack, Some(msg.id))
        .await
    {
        error!(error = %e, "Failed to send file acknowledgment");
    }
}

async fn handle_image_message(
    framework: &AgentFramework,
    msg: rigent::agentlink_protocol::message::MessageResponse,
) {
    let conversation_id = msg.conversation_id.to_string();

    info!(
        conversation_id = %conversation_id,
        sender_id = %msg.sender_id,
        sender_name = %msg.sender_name,
        "Received image message"
    );

    // Parse image attachment from metadata
    let attachment: Option<FileAttachment> = msg
        .metadata
        .as_ref()
        .and_then(|m| serde_json::from_value(m.clone()).ok());

    let image_desc = match &attachment {
        Some(att) => format!("image '{}' ({} bytes)", att.filename, att.size),
        None => "an image".to_string(),
    };

    let ack = format!(
        "I've received {}. I can see images, but detailed image analysis is not yet supported. Feel free to describe what you'd like me to focus on!",
        image_desc
    );

    if let Err(e) = framework
        .send_reply(&conversation_id, ack, Some(msg.id))
        .await
    {
        error!(error = %e, "Failed to send image acknowledgment");
    }
}

// ===================================================================
// Main
// ===================================================================

#[tokio::main]
async fn main() -> Result<()> {
    // Explicitly install the ring crypto provider for rustls.
    // Without this, rustls panics when both aws-lc-rs and ring features
    // are present in the dependency tree because it cannot auto-select.
    let _ = rustls::crypto::ring::default_provider().install_default();

    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            env::var("RUST_LOG")
                .unwrap_or_else(|_| "chatbot_agent=info,agentlink_rust_sdk=warn,rig_core=warn".into()),
        )
        .init();

    // Load Rigent framework configuration
    let mut config = FrameworkConfig::from_env()?;

    // Default skill to chatbot if not explicitly set
    if env::var("SKILL_NAME").is_err() {
        config.skill_name = "chatbot".to_string();
    }

    // Default max_history to 20 if not explicitly set
    if env::var("MAX_HISTORY").is_err() {
        config.max_history = 20;
    }

    info!(
        provider = %config.llm_provider,
        model = %config.llm_model,
        skill = %config.skill_name,
        max_turns = config.max_turns,
        max_history = config.max_history,
        "Starting chatbot agent"
    );

    // Initialize Rigent framework (connects to AgentLink, loads skill, builds LLM agent, sets up memory)
    let framework = AgentFramework::new(&config).await?;

    // Register message handler
    let msg_framework = framework.clone();
    framework.sdk_client.on(MESSAGE_CREATED, move |payload| {
        let fw = msg_framework.clone();

        async move {
            let msg = payload.message;

            // Ignore our own messages
            if msg.sender_id == fw.my_user_id {
                return;
            }

            match msg.kind {
                MessageType::Text => {
                    handle_text_message(&fw, msg).await;
                }
                MessageType::File => {
                    handle_file_message(&fw, msg).await;
                }
                MessageType::Image => {
                    handle_image_message(&fw, msg).await;
                }
                _ => {
                    info!(
                        conversation_id = %msg.conversation_id,
                        kind = ?msg.kind,
                        "Ignoring unsupported message type"
                    );
                }
            }
        }
    });

    // Register connection event handlers
    framework.sdk_client.on(CONNECTION_READY, |payload| async move {
        info!(
            user_id = %payload.user_id,
            linkid = %payload.linkid,
            "WebSocket connected and ready"
        );
    });

    framework.sdk_client.on(ERROR, |payload| async move {
        error!(
            code = %payload.code,
            message = %payload.message,
            "WebSocket error received"
        );
    });

    // Set agent availability to online
    if let Err(e) = framework.set_availability(true).await {
        error!(error = %e, "Failed to set agent availability to online");
    } else {
        info!("Agent availability set to online");
    }

    // Run WebSocket event loop in a background task
    let mut poll_client = framework.sdk_client.clone();
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

    // Set agent availability to offline on shutdown
    if let Err(e) = framework.set_availability(false).await {
        error!(error = %e, "Failed to set agent availability to offline");
    } else {
        info!("Agent availability set to offline");
    }

    info!("Chatbot agent stopped");
    Ok(())
}
