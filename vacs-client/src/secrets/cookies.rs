use crate::secrets;
use crate::secrets::SecretKey;
use anyhow::Context;
use chacha20poly1305::{
    ChaCha20Poly1305, Key, Nonce,
    aead::{Aead, AeadCore, KeyInit, OsRng},
};
use cookie_store::{CookieStore, RawCookie, RawCookieParseError};
use reqwest::header::HeaderValue;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::RwLock;
use url::Url;

const KEY_SIZE: usize = 32;
const NONCE_SIZE: usize = 12;

pub struct SecureCookieStore {
    store: RwLock<CookieStore>,
    path: PathBuf,
}

impl SecureCookieStore {
    pub fn new<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let store = if path.as_ref().exists() {
            Self::load(path.as_ref())
        } else if path.as_ref().with_extension("bak").exists() {
            log::warn!("Encrypted cookie store on disk is missing, restoring from backup");
            Self::load(path.as_ref().with_extension("bak"))
        } else {
            log::info!("Encrypted cookie store does not existing on disk, creating new one");
            CookieStore::new()
        };

        Ok(Self {
            store: RwLock::new(store),
            path: path.as_ref().to_path_buf(),
        })
    }

    pub fn load<P: AsRef<Path>>(path: P) -> CookieStore {
        log::debug!(
            "Loading encrypted cookie store from disk: {}",
            path.as_ref().display()
        );
        match fs::read(path.as_ref()) {
            Ok(encrypted) => {
                log::trace!("Decrypting cookie store");
                match Self::decrypt_store(encrypted).context("Failed to decrypt cookie store") {
                    Ok(store) => store,
                    Err(err) => {
                        log::error!("Failed to decrypt cookie store: {err}");
                        log::warn!("Resetting cookie store, re-authentication will be required");

                        let corrupted_path = path.as_ref().with_extension("corrupted");
                        if let Err(err) = fs::rename(path.as_ref(), corrupted_path.as_path()) {
                            log::warn!("Failed to preserve corrupted cookie store: {err}");
                            fs::remove_file(corrupted_path).ok();
                        }

                        if let Err(err) = secrets::remove(SecretKey::CookieStoreEncryptionKey) {
                            log::warn!("Failed to remove cookie store encryption key: {err}");
                        }

                        CookieStore::new()
                    }
                }
            }
            Err(err) => {
                log::error!("Failed to read encrypted cookie store from disk: {err}");
                log::warn!("Resetting cookie store, re-authentication will be required");
                CookieStore::new()
            }
        }
    }

    pub fn save(&self) -> anyhow::Result<()> {
        log::debug!(
            "Saving encrypted cookie store to disk: {}",
            self.path.display()
        );
        let store = self
            .store
            .read()
            .map_err(|e| anyhow::anyhow!("Failed to lock cookie store: {e}"))?;

        log::trace!("Encrypting cookie store");
        let encrypted = Self::encrypt_store(&store).context("Failed to encrypt cookie store")?;

        if self.path.exists() {
            log::trace!("Backing up existing encrypted cookie store on disk");
            fs::copy(&self.path, self.path.with_extension("bak")).ok();
        }

        log::trace!("Writing encrypted cookie store to disk");
        fs::create_dir_all(self.path.parent().unwrap()).ok();

        fs::write(&self.path, encrypted)
            .context("Failed to write encrypted cookie store to disk")?;

        log::debug!("Saved encrypted cookie store to disk");
        Ok(())
    }

    pub fn clear(&self) -> anyhow::Result<()> {
        log::debug!("Clearing cookie store");
        let mut store = self
            .store
            .write()
            .map_err(|e| anyhow::anyhow!("Failed to lock cookie store: {e}"))?;

        store.clear();

        Ok(())
    }

    fn get_or_generate_encryption_key() -> anyhow::Result<Vec<u8>> {
        match secrets::get_binary(SecretKey::CookieStoreEncryptionKey) {
            Ok(Some(key)) => {
                let key = if key.len() == KEY_SIZE {
                    key
                } else {
                    log::error!(
                        "Stored cookie store encryption key has invalid length, regenerating"
                    );

                    let key = ChaCha20Poly1305::generate_key(&mut OsRng);
                    if let Err(err) =
                        secrets::set_binary(SecretKey::CookieStoreEncryptionKey, key.as_ref())
                    {
                        log::error!("Failed to save cookie store encryption key: {err}");
                        return Err(err).context("Failed to save cookie store encryption key");
                    }

                    key.to_vec()
                };
                Ok(key)
            }
            Ok(None) => {
                log::info!("Generating new cookie store encryption key");

                let key = ChaCha20Poly1305::generate_key(&mut OsRng);
                if let Err(err) =
                    secrets::set_binary(SecretKey::CookieStoreEncryptionKey, key.as_ref())
                {
                    log::error!("Failed to save cookie store encryption key: {err}");
                    return Err(err).context("Failed to save cookie store encryption key");
                }

                Ok(key.to_vec())
            }
            Err(err) => anyhow::bail!(err.context("Failed to get cookie store encryption key")),
        }
    }

    fn decrypt_store(encrypted: Vec<u8>) -> anyhow::Result<CookieStore> {
        if encrypted.len() < NONCE_SIZE {
            anyhow::bail!(
                "Encrypted cookie store content is too short ({} bytes), expected at least {} bytes",
                encrypted.len(),
                NONCE_SIZE
            );
        }

        let (nonce, ciphertext) = encrypted.split_at(NONCE_SIZE);
        let nonce = Nonce::from_slice(nonce);
        let key = Self::get_or_generate_encryption_key()?;
        let cipher = ChaCha20Poly1305::new(Key::from_slice(&key));

        let decrypted = cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| anyhow::anyhow!(e))
            .context("Failed to decrypt cookie store")?;

        let reader = std::io::BufReader::new(decrypted.as_slice());
        cookie_store::serde::json::load(reader)
            .map_err(|e| anyhow::anyhow!(e))
            .context("Failed to deserialize cookie store")
    }

    fn encrypt_store(store: &CookieStore) -> anyhow::Result<Vec<u8>> {
        let key = Self::get_or_generate_encryption_key()?;
        let cipher = ChaCha20Poly1305::new(Key::from_slice(&key));
        let nonce = ChaCha20Poly1305::generate_nonce(&mut OsRng);

        let mut writer = std::io::BufWriter::new(Vec::new());
        cookie_store::serde::json::save(store, &mut writer)
            .map_err(|e| anyhow::anyhow!(e))
            .context("Failed to serialize cookie store")?;

        let encrypted = cipher
            .encrypt(&nonce, writer.buffer())
            .map_err(|e| anyhow::anyhow!(e))
            .context("Failed to encrypt cookie store")?;

        let mut output = nonce.to_vec();
        output.extend(&encrypted);

        Ok(output)
    }
}

impl reqwest::cookie::CookieStore for SecureCookieStore {
    fn set_cookies(&self, cookie_headers: &mut dyn Iterator<Item = &HeaderValue>, url: &Url) {
        let mut store = self.store.write().expect("Failed to lock cookie store");

        let cookies = cookie_headers.filter_map(|val| {
            std::str::from_utf8(val.as_bytes())
                .map_err(RawCookieParseError::from)
                .and_then(RawCookie::parse)
                .map(|c| c.into_owned())
                .ok()
        });

        store.store_response_cookies(cookies, url);
    }

    fn cookies(&self, url: &Url) -> Option<HeaderValue> {
        let store = self.store.read().expect("Failed to lock cookie store");

        let s = store
            .get_request_values(url)
            .map(|(name, value)| format!("{name}={value}"))
            .collect::<Vec<_>>()
            .join("; ");

        if s.is_empty() {
            return None;
        }

        HeaderValue::from_str(s.as_str()).ok()
    }
}
