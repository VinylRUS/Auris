//! Хранилище OAuth-токена: системный keyring с файловым fallback
//! (файл лежит рядом с программой: `<корень>/data/tokens.json`).

use std::path::PathBuf;

const SERVICE: &str = "Auris";
const ACCOUNT: &str = "yandex-oauth-token";

/// Где реально лежит токен.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StorageKind {
    /// Системный keyring (Windows Credential Manager / macOS Keychain /
    /// Freedesktop Secret Service).
    Keyring,
    /// Файл `tokens.json` в каталоге данных (fallback без DBus).
    File,
}

/// Хранилище токена.
#[derive(Debug, Clone, Default)]
pub struct TokenStorage {
    /// Куда сохранили в последний раз.
    pub kind: Option<StorageKind>,
}

impl TokenStorage {
    /// Читает токен: сначала keyring, затем файл.
    pub fn load(&mut self) -> anyhow::Result<Option<String>> {
        match self.load_keyring() {
            Ok(Some(token)) => {
                self.kind = Some(StorageKind::Keyring);
                return Ok(Some(token));
            }
            Ok(None) => {}
            Err(e) => {
                log::warn!("keyring unavailable ({e}); falling back to file storage");
            }
        }
        match Self::load_file()? {
            Some(token) => {
                self.kind = Some(StorageKind::File);
                Ok(Some(token))
            }
            None => Ok(None),
        }
    }

    /// Сохраняет токен: keyring, при сбое — файл с правами 0600.
    pub fn store(&mut self, token: &str) -> anyhow::Result<()> {
        match self.store_keyring(token) {
            Ok(()) => {
                self.kind = Some(StorageKind::Keyring);
                Ok(())
            }
            Err(e) => {
                log::warn!("keyring store failed ({e}); using file storage");
                Self::store_file(token)?;
                self.kind = Some(StorageKind::File);
                Ok(())
            }
        }
    }

    /// Удаляет токен из всех хранилищ (выход из аккаунта).
    pub fn purge(&mut self) -> anyhow::Result<()> {
        let _ = self.purge_keyring();
        Self::purge_file()?;
        self.kind = None;
        Ok(())
    }

    // ------------------------------------------------------------------
    // keyring
    // ------------------------------------------------------------------

    fn load_keyring(&self) -> anyhow::Result<Option<String>> {
        let entry = keyring::Entry::new(SERVICE, ACCOUNT)?;
        match entry.get_password() {
            Ok(p) => Ok(Some(p)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(anyhow::anyhow!("{e}")),
        }
    }

    fn store_keyring(&self, token: &str) -> anyhow::Result<()> {
        let entry = keyring::Entry::new(SERVICE, ACCOUNT)?;
        // В keyring v3 нет отдельного AlreadyExists: перезапись допустима,
        // но на части бэкендов требует сначала удалить запись.
        if let Err(e) = entry.set_password(token) {
            let _ = entry.delete_credential();
            entry
                .set_password(token)
                .map_err(|e| anyhow::anyhow!("retry set: {e}"))?;
            log::debug!("keyring required delete+set: {e}");
        }
        Ok(())
    }

    fn purge_keyring(&self) -> anyhow::Result<()> {
        let entry = keyring::Entry::new(SERVICE, ACCOUNT)?;
        match entry.delete_credential() {
            Ok(()) => Ok(()),
            Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(anyhow::anyhow!("{e}")),
        }
    }

    // ------------------------------------------------------------------
    // файл (fallback)
    // ------------------------------------------------------------------

    fn file_path() -> PathBuf {
        crate::paths::data_dir().join("tokens.json")
    }

    fn load_file() -> anyhow::Result<Option<String>> {
        let path = Self::file_path();
        if !path.exists() {
            return Ok(None);
        }
        #[derive(serde::Deserialize)]
        struct FileToken {
            token: String,
        }
        let raw = std::fs::read_to_string(&path)?;
        Ok(serde_json::from_str::<FileToken>(&raw)
            .ok()
            .map(|t| t.token))
    }

    fn store_file(token: &str) -> anyhow::Result<()> {
        let path = Self::file_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::json!({ "token": token }).to_string();
        std::fs::write(&path, json)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
        }
        Ok(())
    }

    fn purge_file() -> anyhow::Result<()> {
        let path = Self::file_path();
        if path.exists() {
            std::fs::remove_file(&path)?;
        }
        Ok(())
    }
}
