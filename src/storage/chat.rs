//! Per-chat SQLCipher database (`{contact}.db`).

use anyhow::{Context, Result};
use rusqlite::{Connection, params};
use std::path::PathBuf;

pub struct ChatDb {
    contact: String,
    conn: Connection,
}

impl ChatDb {
    /// Open (creating if needed) the per-chat DB for `contact`.
    pub fn open(config_dir: &PathBuf, contact: &str, key: &str) -> Result<Self> {
        let safe = sanitize_filename(contact);
        let path = config_dir.join(format!("{safe}.db"));

        let conn = Connection::open(&path)
            .with_context(|| format!("Failed to open chat DB for '{contact}'"))?;

        // Set the per-chat encryption key (different from primary)
        conn.execute_batch(&format!("PRAGMA key = '{}';", sqlcipher_escape(&key)))?;
        conn.execute("SELECT count(*) FROM sqlite_master;", [])?;

        let s = Self { contact: contact.to_string(), conn };
        s.init_schema()?;
        Ok(s)
    }

    fn init_schema(&self) -> Result<()> {
        self.conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS messages (
                id              INTEGER PRIMARY KEY,
                our_message_id  TEXT,
                their_message_id TEXT,
                direction       TEXT NOT NULL,
                ciphertext      BLOB NOT NULL,
                timestamp       INTEGER NOT NULL,
                status          TEXT NOT NULL DEFAULT 'sent',
                UNIQUE(our_message_id, direction)
            );

            CREATE TABLE IF NOT EXISTS ratchet_state (
                id              INTEGER PRIMARY KEY CHECK (id = 1),
                state           BLOB NOT NULL
            );

            CREATE TABLE IF NOT EXISTS prekeys (
                id              INTEGER PRIMARY KEY,
                prekey_pub      BLOB NOT NULL,
                prekey_priv     BLOB NOT NULL,
                created_at      INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS session_record (
                id              INTEGER PRIMARY KEY CHECK (id = 1),
                record          BLOB NOT NULL
            );
            ",
        )?;
        Ok(())
    }

    pub fn contact(&self) -> &str {
        &self.contact
    }

    pub fn insert_message(
        &self,
        direction: &str,
        ciphertext: &[u8],
        timestamp: i64,
    ) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO messages (direction, ciphertext, timestamp) VALUES (?1, ?2, ?3)",
            params![direction, ciphertext, timestamp],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn get_messages(&self, limit: usize) -> Result<Vec<ChatMessage>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, direction, ciphertext, timestamp, status
             FROM messages ORDER BY timestamp DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit as i64], |row| {
            Ok(ChatMessage {
                id: row.get(0)?,
                direction: row.get(1)?,
                ciphertext: row.get(2)?,
                timestamp: row.get(3)?,
                status: row.get(4)?,
            })
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    pub fn save_session_record(&self, record: &[u8]) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO session_record (id, record) VALUES (1, ?1)",
            params![record],
        )?;
        Ok(())
    }

    pub fn load_session_record(&self) -> Result<Option<Vec<u8>>> {
        let mut stmt = self.conn.prepare("SELECT record FROM session_record WHERE id = 1")?;
        let mut rows = stmt.query([])?;
        if let Some(row) = rows.next()? {
            Ok(Some(row.get(0)?))
        } else {
            Ok(None)
        }
    }

    pub fn save_ratchet_state(&self, state: &[u8]) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO ratchet_state (id, state) VALUES (1, ?1)",
            params![state],
        )?;
        Ok(())
    }

    pub fn load_ratchet_state(&self) -> Result<Option<Vec<u8>>> {
        let mut stmt = self.conn.prepare("SELECT state FROM ratchet_state WHERE id = 1")?;
        let mut rows = stmt.query([])?;
        if let Some(row) = rows.next()? {
            Ok(Some(row.get(0)?))
        } else {
            Ok(None)
        }
    }
}

#[derive(Debug)]
pub struct ChatMessage {
    pub id: i64,
    pub direction: String,
    pub ciphertext: Vec<u8>,
    pub timestamp: i64,
    pub status: String,
}

fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect()
}

fn sqlcipher_escape(key: &str) -> String {
    key.replace('\'', "''")
}
