//! Top-level client state machine and command dispatcher.

use crate::cli::Config;
use crate::auth;
use crate::storage;
use anyhow::Result;

pub async fn run(cfg: Config) -> Result<()> {
    match &cfg.command {
        crate::cli::Commands::Auth { no_open } => {
            auth::google_oauth(*no_open).await?;
        }
        crate::cli::Commands::List => {
            let store = storage::Store::new(&cfg.config_dir)?;
            let chats = store.list_chats()?;
            if chats.is_empty() {
                println!("No conversations yet.");
            } else {
                for chat in chats {
                    println!("  {}  (last: {})", chat.name, chat.last_message_at);
                }
            }
        }
        crate::cli::Commands::Chat { contact } => {
            let store = storage::Store::new(&cfg.config_dir)?;
            let chat_db = store.open_chat(contact)?;
            // TODO: launch ratatui interactive TUI
            println!("[TUI] Interactive chat with {contact} — TUI not yet implemented.");
            let _ = chat_db;
        }
        crate::cli::Commands::Send { contact, message } => {
            let msg = message.join(" ");
            if msg.is_empty() {
                anyhow::bail!("Empty message");
            }
            // TODO: actual send via MQTT + crypto
            println!("[WARN] Send path not yet implemented — would send to {contact}: {msg}");
        }
        crate::cli::Commands::Verify { contact } => {
            let store = storage::Store::new(&cfg.config_dir)?;
            if let Some(fp) = store.get_fingerprint(contact)? {
                println!("Fingerprint: {}", fp);
            } else {
                println!("Contact '{contact}' not found.");
            }
        }
        crate::cli::Commands::Daemon { pidfile } => {
            // TODO: daemonize + MQTT background loop
            println!("[WARN] Daemon mode not yet implemented.");
            std::fs::write(pidfile, std::process::id().to_string())?;
        }
    }
    Ok(())
}
