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

#[derive(Debug, serde::Serialize, Clone)]
pub struct ChatSummary {
    pub phone: String,
    pub name: String,
    pub last_message_at: String,
}

#[derive(Debug, serde::Serialize)]
pub struct ContactSummary {
    pub phone: String,
    pub name: String,
}

#[derive(Debug, serde::Serialize)]
pub struct AccountStatus {
    pub user_id: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub device_id: Option<String>,
    pub mqtt_connected: bool,
}

#[derive(Debug, serde::Serialize)]
pub struct MessageSummary {
    pub id: String,
    pub content: String,
    pub outgoing: bool,
    pub created_at: String,
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

    /// Add a contact by phone + name.
    pub fn add_contact(&self, phone: &str, name: &str) -> Result<()> {
        self.primary.add_contact(phone, name)
    }

    /// List all contacts (phone + name).
    pub fn list_contacts(&self) -> Result<Vec<ContactSummary>> {
        self.primary.list_all_contacts()
    }

    /// Return account and connection status.
    pub fn account_status(&self) -> Result<AccountStatus> {
        self.primary.account_status()
    }

    /// Retrieve message history for a contact.
    /// Note: `content` is the raw ciphertext hex until E2EE is wired up.
    pub fn get_history(
        &self,
        contact: &str,
        limit: u32,
        _before: Option<&str>,
    ) -> Result<Vec<MessageSummary>> {
        let chat_db = self.open_chat(contact)?;
        let msgs = chat_db.get_messages(limit as usize)?;
        Ok(msgs
            .into_iter()
            .map(|m| {
                let outgoing = m.direction == "outgoing";
                let content = format!("[encrypted: {} bytes]", m.ciphertext.len());
                let created_at = chrono::DateTime::from_timestamp(m.timestamp, 0)
                    .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                    .unwrap_or_else(|| m.timestamp.to_string());
                MessageSummary {
                    id: m.id.to_string(),
                    content,
                    outgoing,
                    created_at,
                }
            })
            .collect())
    }

    /// Stub: return a chat DB handle for a contact.
    /// TODO: wire up MQTT + crypto pipeline for actual send.
    pub fn get_or_create_client(&self, contact: &str) -> Result<ChatDb> {
        let key = self.primary.get_chat_key()?;
        ChatDb::open(&self.config_dir, contact, &key)
    }
}
