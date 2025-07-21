use anyhow::Context;
use keyring::error::Error::NoEntry;
use keyring::Entry;

fn entry_for_key(key: &str) -> anyhow::Result<Entry> {
    Entry::new(env!("CARGO_PKG_NAME"), key).context("Failed to create entry")
}

pub fn get(key: &str) -> anyhow::Result<Option<String>> {
    match entry_for_key(key)?.get_password() {
        Ok(password) => Ok(Some(password)),
        Err(NoEntry) => Ok(None),
        Err(err) => Err(anyhow::anyhow!(err).context("Failed to get secret")),
    }
}

pub fn set(key: &str, value: &str) -> anyhow::Result<()> {
    entry_for_key(key)?
        .set_password(value)
        .context("Failed to set secret")
}

pub fn remove(key: &str) -> anyhow::Result<()> {
    entry_for_key(key)?
        .delete_credential()
        .context("Failed to remove secret")
}
