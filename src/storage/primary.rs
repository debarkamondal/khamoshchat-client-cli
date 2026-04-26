//! Primary SQLCipher database (`__primary__.db`).

use anyhow::{Context, Result};
use rand_core::{OsRng, RngCore};
use rusqlite::{Connection, params};
use std::path::PathBuf;

use crate::keyring;

const KEYRING_SERVICE: &str = "khamoshchat";
const PRIMARY_KEYRING_USER: &str = "primary_db_key";

pub struct PrimaryDb {
    conn: Connection,
}

impl PrimaryDb {
    /// Open (creating if needed) the primary DB, unlocking with the keyring key.
    pub fn open(config_dir: &PathBuf) -> Result<Self> {
        let path = config_dir.join("__primary__.db");
        let key = Self::load_or_create_key()?;

        // Open with SQLCipher encryption via PRAGMA key
        let conn = Connection::open(&path)
            .with_context(|| format!("Failed to open primary DB at {:?}", path))?;

        // Set the encryption key (SQLCipher PRAGMA)
        conn.execute_batch(&format!("PRAGMA key = '{}';", sqlcipher_escape(&key)))?;
        // Verify the database is accessible (SQLCipher will fail if key is wrong)
        conn.execute("SELECT count(*) FROM sqlite_master;", [])?;

        let s = Self { conn };
        s.init_schema()?;
        Ok(s)
    }

    fn load_or_create_key() -> Result<String> {
        match keyring::get(KEYRING_SERVICE, PRIMARY_KEYRING_USER) {
            Ok(k) => Ok(k),
            Err(_) => {
                let new_key = generate_key();
                keyring::set(KEYRING_SERVICE, PRIMARY_KEYRING_USER, &new_key)?;
                Ok(new_key)
            }
        }
    }

    fn init_schema(&self) -> Result<()> {
        self.conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS account (
                id          INTEGER PRIMARY KEY,
                user_id     TEXT NOT NULL UNIQUE,
                display_name TEXT,
                created_at  INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS contacts (
                id          INTEGER PRIMARY KEY,
                user_id     TEXT NOT NULL UNIQUE,
                display_name TEXT,
                identity_key TEXT NOT NULL,
                trust_level  INTEGER NOT NULL DEFAULT 0,
                created_at   INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS outbox (
                id          INTEGER PRIMARY KEY,
                recipient   TEXT NOT NULL,
                payload     BLOB NOT NULL,
                created_at  INTEGER NOT NULL,
                status      TEXT NOT NULL DEFAULT 'pending'
            );

            CREATE TABLE IF NOT EXISTS inbox (
                id          INTEGER PRIMARY KEY,
                sender      TEXT NOT NULL,
                payload     BLOB NOT NULL,
                received_at INTEGER NOT NULL,
                decrypted   INTEGER NOT NULL DEFAULT 0
            );

            CREATE TABLE IF NOT EXISTS devices (
                id          INTEGER PRIMARY KEY,
                device_id   TEXT NOT NULL UNIQUE,
                name        TEXT,
                last_seen   INTEGER
            );
            ",
        )?;
        Ok(())
    }

    pub fn list_contacts(&self) -> Result<Vec<super::ChatSummary>> {
        let mut stmt = self.conn.prepare(
            "SELECT display_name FROM contacts ORDER BY display_name",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(super::ChatSummary {
                name: row.get(0)?,
                last_message_at: "—".into(),
            })
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    pub fn get_fingerprint(&self, contact: &str) -> Result<Option<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT identity_key FROM contacts WHERE user_id = ?1 OR display_name = ?1",
        )?;
        let mut rows = stmt.query(params![contact])?;
        if let Some(row) = rows.next()? {
            let ik: String = row.get(0)?;
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut h = DefaultHasher::new();
            ik.hash(&mut h);
            let fp = format!("{:016x}", h.finish());
            Ok(Some(fp))
        } else {
            Ok(None)
        }
    }

    /// Returns the chat-db encryption key from keyring.
    pub fn get_chat_key(&self) -> Result<String> {
        keyring::get(KEYRING_SERVICE, "chat_db_key")
    }
}

/// SQLCipher requires escaping single quotes in the key.
fn sqlcipher_escape(key: &str) -> String {
    key.replace('\'', "''")
}

fn generate_key() -> String {
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &bytes)
}
