//! Two-tier SQLCipher storage.
//!
//! ## Tier 1 — Primary DB (`__primary__.db`)
//! Contains: account, devices, contacts, inbox queue, outbox queue.
//! Key: `khamoshchat/primary_db_key` in system keyring.
//!
//! ## Tier 2 — Per-chat DB (`{contact}.db`)
//! Contains: messages, ratchet sessions, prekeys for ONE contact.
//! Key: `khamoshchat/chat_db_key` in system keyring.

mod primary;
mod chat;

pub use primary::PrimaryDb;
pub use chat::ChatDb;

use anyhow::Result;
use std::path::PathBuf;

const PRIMARY_DB: &str = "__primary__.db";

/// Handle to the primary store — caller passes it to `open_chat`.
pub struct Store {
    config_dir: PathBuf,
    primary: PrimaryDb,
}

#[derive(Debug)]
pub struct ChatSummary {
    pub name: String,
    pub last_message_at: String,
}

impl Store {
    pub fn new(config_dir: &PathBuf) -> Result<Self> {
        let primary = PrimaryDb::open(config_dir)?;
        Ok(Self {
            config_dir: config_dir.clone(),
            primary,
        })
    }

    /// List all known contacts.
    pub fn list_chats(&self) -> Result<Vec<ChatSummary>> {
        self.primary.list_contacts()
    }

    /// Open (or create) the per-chat DB for a given contact.
    pub fn open_chat(&self, contact: &str) -> Result<ChatDb> {
        let key: String = self.primary.get_chat_key()?;
        ChatDb::open(&self.config_dir, contact, &key)
    }

    /// Get the identity fingerprint for a contact (for verification).
    pub fn get_fingerprint(&self, contact: &str) -> Result<Option<String>> {
        self.primary.get_fingerprint(contact)
    }
}
