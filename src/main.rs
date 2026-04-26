//! Khamoshchat CLI — Pure Rust E2EE messaging client for Linux.
//!
//! Architecture:
//!   - `cli`     — Clap argument parsing
//!   - `client`  — Top-level state machine / event loop
//!   - `storage` — Two-tier SQLCipher (primary + per-chat)
//!   - `crypto`  — Signal Protocol via libsignal-dezire
//!   - `mqtt`    — rumqttc async MQTT transport
//!   - `auth`    — Google OAuth with local redirect server

mod cli;
mod client;
mod storage;
mod crypto;
mod mqtt;
mod auth;
mod keyring;

use anyhow::Result;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let cli = cli::build();

    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(client::run(cli))
}
