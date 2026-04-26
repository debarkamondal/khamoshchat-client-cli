//! Clap CLI definitions — headless, integration-focused.
//! Designed for use by GUI tools, other CLIs, and AI agents.

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug, Clone)]
#[command(
    name = "khamoshchat",
    about = "E2EE CLI messenger for Khamoshchat (headless)",
    version = "0.1.0"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Override config directory (default: ~/.config/khamoshchat/)
    #[arg(short, long)]
    pub config: Option<PathBuf>,

    /// Output JSON instead of human-readable text.
    /// Recommended when piping output to other programs.
    #[arg(short, long)]
    pub json: bool,
}

#[derive(Subcommand, Debug, Clone)]
pub enum Commands {
    /// Authenticate with Google OAuth and register this device.
    Auth {
        /// Skip opening browser — print the URL instead.
        #[arg(long)]
        no_open: bool,
    },

    /// List all conversations with last-activity timestamp.
    List,

    /// Retrieve message history for a contact.
    History {
        /// Contact identifier (phone or registered username).
        contact: String,

        /// Maximum number of messages to return.
        #[arg(long, default_value = "50")]
        limit: u32,

        /// Cursor: return messages before this message ID.
        #[arg(long)]
        before: Option<String>,
    },

    /// Send a one-shot E2EE message to a contact.
    Send {
        /// Contact to message.
        contact: String,

        /// Message text.
        #[arg(trailing_var_arg = true)]
        message: Vec<String>,
    },

    /// Start a long-lived daemon that listens for incoming messages
    /// and prints them as JSON Lines to stdout.
    Daemon,

    /// Manage contacts.
    Contacts {
        #[command(subcommand)]
        sub: ContactCommands,
    },

    /// Show the safety-number fingerprint for a contact.
    Verify {
        /// Contact identifier.
        contact: String,
    },

    /// Show connection and account status.
    Status,
}

#[derive(Subcommand, Debug, Clone)]
pub enum ContactCommands {
    /// Add a new contact.
    Add {
        /// Phone number.
        phone: String,

        /// Display name.
        name: String,
    },

    /// List all registered contacts.
    List,
}

impl Cli {
    pub fn config_dir(&self) -> PathBuf {
        self.config.clone().unwrap_or_else(|| {
            dirs::config_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("khamoshchat")
        })
    }
}

/// Parse CLI arguments using clap.
pub fn build() -> Cli {
    Cli::parse()
}
