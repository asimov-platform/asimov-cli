// This is free and unencumbered software released into the public domain.

use iroh_base::{PublicKey, SecretKey};
use keyring_core::Error;
use secrecy::zeroize::Zeroize;

const KEYRING_SERVICE: &str = "sh.asimov";

pub struct Keyring;

impl Keyring {
    pub fn my_public_key() -> Result<PublicKey, Error> {
        let keyring = Keyring::open()?;
        let user = whoami::username().unwrap_or_else(|_| "default".to_string());
        let public_key = match keyring.get_public_key(&user)? {
            Some(pk) => pk,
            None => keyring.rekey(&user)?.0,
        };
        keyring.close()?;
        Ok(public_key)
    }

    pub fn open() -> Result<Self, Error> {
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

        Ok(Self)
    }

    pub fn close(&self) -> Result<(), Error> {
        keyring_core::unset_default_store();
        Ok(())
    }

    pub fn get_public_key(&self, _user: &str) -> Result<Option<PublicKey>, Error> {
        Ok(None) // TODO
    }

    pub fn get_secret_key(&self, user: &str) -> Result<Option<SecretKey>, Error> {
        match keyring_core::Entry::new(KEYRING_SERVICE, &user)?.get_secret() {
            Ok(secret) => {
                let mut secret_bytes: [u8; 32] = [0; 32];
                secret_bytes.copy_from_slice(&secret);
                let secret_key = SecretKey::from_bytes(&secret_bytes);
                secret_bytes.zeroize();
                return Ok(Some(secret_key));
            },
            Err(keyring_core::Error::NoEntry) => {
                let (_, secret_key) = self.rekey(user)?;
                return Ok(Some(secret_key));
            },
            Err(error) => return Err(error.into()), // TODO: print a warning
        }
    }

    pub fn rekey(&self, user: &str) -> Result<(PublicKey, SecretKey), Error> {
        let secret_key = SecretKey::generate();
        let mut secret_bytes = secret_key.to_bytes();
        keyring_core::Entry::new(KEYRING_SERVICE, &user)?.set_secret(&secret_bytes)?;
        // TODO: store the public key as well
        secret_bytes.zeroize();
        Ok((secret_key.public(), secret_key))
    }
}
