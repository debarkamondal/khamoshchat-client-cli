//! Clap CLI definitions and config extraction.

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug, Clone)]
#[command(
    name = "khamoshchat",
    about = "E2EE CLI messenger for Khamoshchat",
    version = "0.1.0"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Override config directory (default: ~/.config/khamoshchat/)
    #[arg(short, long)]
    pub config: Option<PathBuf>,
}

#[derive(Subcommand, Debug, Clone)]
pub enum Commands {
    /// Register or re-link this device
    Auth {
        /// Skip opening browser — print the URL instead
        #[arg(long)]
        no_open: bool,
    },

    /// List all conversations
    List,

    /// Open an interactive chat with a given contact
    Chat {
        /// Contact identifier (phone or registered username)
        contact: String,
    },

    /// Send a message without entering interactive mode
    Send {
        /// Contact to message
        contact: String,
        /// Message text
        #[arg(trailing_var_arg = true)]
        message: Vec<String>,
    },

    /// Show the key fingerprint for a contact
    Verify {
        /// Contact identifier
        contact: String,
    },

    /// Start background daemon (future)
    Daemon {
        /// PID file path
        #[arg(long, default_value = "/run/khamoshchat.pid")]
        pidfile: PathBuf,
    },
}

/// Build the clap CLI.
pub fn build() -> Cli {
    Cli::parse()
}

/// Aggregated runtime config derived from CLI args + env.
#[derive(Debug)]
pub struct Config {
    pub config_dir: PathBuf,
    pub command: Commands,
}

impl Config {
    pub fn from_matches(matches: &Cli) -> anyhow::Result<Self> {
        let config_dir = matches.config.clone()
            .unwrap_or_else(|| {
                dirs::config_dir()
                    .unwrap_or_else(|| PathBuf::from("."))
                    .join("khamoshchat")
            });

        std::fs::create_dir_all(&config_dir)?;
        Ok(Self {
            config_dir,
            command: matches.command.clone(),
        })
    }
}
