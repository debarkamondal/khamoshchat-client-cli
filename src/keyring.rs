//! Thin wrapper around the `keyring` crate for Khamoshchat credentials.

use anyhow::Result;

pub fn get(service: &str, user: &str) -> Result<String> {
    let entry = keyring::Entry::new(service, user)?;
    entry.get_password().map_err(|e| anyhow::anyhow!("{e}"))
}

pub fn set(service: &str, user: &str, value: &str) -> Result<()> {
    let entry = keyring::Entry::new(service, user)?;
    entry.set_password(value).map_err(|e| anyhow::anyhow!("{e}"))
}

pub fn delete(service: &str, user: &str) -> Result<()> {
    let entry = keyring::Entry::new(service, user)?;
    entry.delete_credential().map_err(|e| anyhow::anyhow!("{e}"))
}
