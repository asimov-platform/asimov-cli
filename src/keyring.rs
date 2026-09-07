// This is free and unencumbered software released into the public domain.

use asimov_directory::fs::StateDirectory;
use asimov_id::{KeyError, PublicKey};
use clientele::Utf8PathBuf;
use iroh_base::SecretKey;
use secrecy::zeroize::{Zeroize, Zeroizing};
use thiserror::Error;

const KEYRING_SERVICE: &str = "sh.asimov";

#[derive(Debug, Error)]
pub enum KeyringError {
    #[error("user not found")]
    UserNotFound,

    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("keyring error: {0}")]
    KeyringError(#[from] keyring_core::Error),

    #[error("key error: {0}")]
    KeyError(#[from] KeyError),
}

pub struct Keyring {
    path: Utf8PathBuf,
}

impl Keyring {
    pub fn my_public_key() -> Result<PublicKey, KeyringError> {
        let mut keyring = Keyring::open()?;
        let user = whoami::username().unwrap_or_else(|_| "default".to_string());
        let public_key = match keyring.get_public_key(&user)? {
            Some(public_key) => public_key,
            None => keyring.rekey(&user).map(|(_, public_key)| public_key)?,
        };
        keyring.close()?;
        Ok(public_key)
    }

    pub fn open() -> Result<Self, KeyringError> {
        let path = StateDirectory::home()?.join("keyring");
        std::fs::create_dir_all(&path)?;

        // See: <https://docs.rs/apple-native-keyring-store/latest/apple_native_keyring_store/>
        #[cfg(target_vendor = "apple")]
        keyring_core::set_default_store(apple_native_keyring_store::keychain::Store::new()?);

        // See: <https://docs.rs/windows-native-keyring-store/latest/windows_native_keyring_store/>
        #[cfg(target_os = "windows")]
        keyring_core::set_default_store(windows_native_keyring_store::Store::new()?);

        // See: <https://docs.rs/linux-keyutils-keyring-store/latest/linux_keyutils_keyring_store/>
        #[cfg(target_os = "linux")]
        keyring_core::set_default_store(linux_keyutils_keyring_store::Store::new()?);

        // See: <https://docs.rs/keyring-core/latest/keyring_core/mock/index.html>
        #[cfg(not(any(target_vendor = "apple", target_os = "windows", target_os = "linux")))]
        keyring_core::set_default_store(keyring_core::mock::Store::new()?);

        Ok(Self { path })
    }

    pub fn close(&self) -> Result<(), KeyringError> {
        keyring_core::unset_default_store();
        Ok(())
    }

    pub fn get_public_key(&self, user: &str) -> Result<Option<PublicKey>, KeyringError> {
        let key_path = self.path.join(user);
        match std::fs::read_to_string(&key_path) {
            Ok(encoded_pk) => Ok(Some(encoded_pk.trim().parse()?)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error.into()),
        }
    }

    pub fn get_secret_key(&self, user: &str) -> Result<Option<SecretKey>, KeyringError> {
        match keyring_core::Entry::new(KEYRING_SERVICE, &user)?.get_secret() {
            Ok(mut secret) => {
                let secret_key = {
                    let mut secret_bytes = Zeroizing::new([0u8; 32]);
                    secret_bytes.copy_from_slice(&secret);
                    secret.zeroize();
                    SecretKey::from_bytes(&secret_bytes)
                };
                Ok(Some(secret_key))
            },
            Err(keyring_core::Error::NoEntry) => Ok(None),
            Err(error) => Err(error.into()),
        }
    }

    pub fn ensure_secret_key(&mut self, user: &str) -> Result<SecretKey, KeyringError> {
        match self.get_secret_key(user)? {
            Some(secret_key) => Ok(secret_key),
            None => self.rekey(user).map(|(secret_key, _)| secret_key),
        }
    }

    pub fn rekey(&mut self, user: &str) -> Result<(SecretKey, PublicKey), KeyringError> {
        let secret_key = SecretKey::generate();
        {
            let secret_bytes = Zeroizing::new(secret_key.to_bytes());
            keyring_core::Entry::new(KEYRING_SERVICE, &user)?
                .set_secret(secret_bytes.as_slice())?;
        }
        let public_key: PublicKey = secret_key.public().into();
        let key_path = self.path.join(user);
        std::fs::write(&key_path, format!("{}\n", public_key))?;
        Ok((secret_key, public_key))
    }
}
