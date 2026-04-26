//! Top-level client state machine and command dispatcher.
//! All output goes to stdout in either human-readable or JSON format.

use crate::auth;
use crate::cli::{Cli, Commands, ContactCommands};
use crate::storage;
use anyhow::Result;
use serde::Serialize;

/// Human-readable output wrapper for `--json` mode.
#[derive(Serialize)]
struct JsonEnvelope<'a> {
    cmd: &'a str,
    data: serde_json::Value,
}

macro_rules! json_out {
    ($cli:expr, $cmd:literal, $data:expr) => {
        if $cli.json {
            let envelope = JsonEnvelope { cmd: $cmd, data: $data };
            println!("{}", serde_json::to_string(&envelope).unwrap());
            return Ok(());
        }
    };
}

pub async fn run(cli: Cli) -> Result<()> {
    match &cli.command {
        Commands::Auth { no_open } => {
            auth::google_oauth(*no_open).await?;
        }

        Commands::List => {
            let store = storage::Store::new(&cli.config_dir())?;
            let chats = store.list_chats()?;

            json_out!(&cli, "list", serde_json::json!(chats));

            if chats.is_empty() {
                println!("No conversations yet.");
            } else {
                println!("{:<20}  {:<30}  {}", "Contact", "Name", "Last Active", );
                println!("{}", "-".repeat(65));
                for chat in chats {
                    println!("{:<20}  {:<30}  {}", chat.phone, chat.name, chat.last_message_at);
                }
            }
        }

        Commands::History { contact, limit, before } => {
            let store = storage::Store::new(&cli.config_dir())?;
            let msgs = store.get_history(contact, *limit, before.as_deref())?;

            json_out!(&cli, "history", serde_json::json!(msgs));

            if msgs.is_empty() {
                println!("No messages with {contact}.");
            } else {
                println!("{:<8}  {:<20}  {:<6}  {}", "ID", "Timestamp", "Dir", "Content");
                println!("{}", "-".repeat(80));
                for m in msgs {
                    let dir = if m.outgoing { "→" } else { "←" };
                    let preview = if m.content.len() > 60 {
                        format!("{}…", &m.content[..60])
                    } else {
                        m.content.clone()
                    };
                    println!("{:<8}  {:<20}  {:<6}  {}", m.id, m.created_at, dir, preview);
                }
            }
        }

        Commands::Send { contact, message } => {
            let msg = message.join(" ");
            if msg.is_empty() {
                anyhow::bail!("Empty message");
            }

            // TODO: encrypt via crypto pipeline, save to outbox, publish via MQTT
            println!("[WARN] Send not yet wired — would E2EE-send to {contact}: {msg}");

            json_out!(&cli, "send", serde_json::json!({
                "contact": contact,
                "status": "not_implemented",
                "preview": &msg,
            }));
        }

        Commands::Contacts { sub } => {
            let store = storage::Store::new(&cli.config_dir())?;

            match sub {
                ContactCommands::Add { phone, name } => {
                    store.add_contact(phone, name)?;

                    json_out!(&cli, "contacts:add", serde_json::json!({
                        "phone": phone, "name": name
                    }));

                    println!("✓ Contact added: {name} ({phone})");
                }
                ContactCommands::List => {
                    let contacts = store.list_contacts()?;

                    json_out!(&cli, "contacts:list", serde_json::json!(contacts));

                    if contacts.is_empty() {
                        println!("No contacts yet.");
                    } else {
                        println!("{:<20}  {}", "Phone", "Name");
                        println!("{}", "-".repeat(40));
                        for c in contacts {
                            println!("{:<20}  {}", c.phone, c.name);
                        }
                    }
                }
            }
        }

        Commands::Verify { contact } => {
            let store = storage::Store::new(&cli.config_dir())?;
            let fp = store.get_fingerprint(contact)?;

            match fp {
                Some(fingerprint) => {
                    json_out!(&cli, "verify", serde_json::json!({
                        "contact": contact, "fingerprint": fingerprint
                    }));

                    println!("Safety number fingerprint for {contact}:");
                    println!("{}", fingerprint);
                    println!("\nVerify this matches the contact's app.");
                }
                None => {
                    println!("Contact '{contact}' not found.");
                }
            }
        }

        Commands::Status => {
            let store = storage::Store::new(&cli.config_dir())?;
            let status = store.account_status()?;

            json_out!(&cli, "status", serde_json::json!(status));

            println!("Account: {}", status.email.as_deref().unwrap_or("not authenticated"));
            println!("Phone:   {}", status.phone.as_deref().unwrap_or("not set"));
            println!("MQTT:    {}", if status.mqtt_connected { "connected" } else { "disconnected" });
            println!("Device ID: {}", status.device_id.as_deref().unwrap_or("none"));
        }

        Commands::Daemon => {
            println!("Starting daemon mode… (MQTT listener)");
            // TODO: start MQTT event loop, print received messages as JSON Lines to stdout
            println!("[INFO] Daemon mode not yet implemented — see client.rs stub.");
        }
    }
    Ok(())
}
