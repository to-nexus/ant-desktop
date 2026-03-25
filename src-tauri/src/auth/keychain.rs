use crate::auth::AuthError;
use crate::constants::*;
use tracing::{debug, warn};

pub fn save_jwt(jwt: &str) -> Result<(), AuthError> {
    let entry = keyring::Entry::new(KEYCHAIN_SERVICE, KEYCHAIN_JWT_KEY)
        .map_err(|e| AuthError::Keychain(e.to_string()))?;
    entry
        .set_password(jwt)
        .map_err(|e| AuthError::Keychain(e.to_string()))?;
    debug!("saved JWT to keychain");
    Ok(())
}

pub fn load_jwt() -> Result<Option<String>, AuthError> {
    let entry = keyring::Entry::new(KEYCHAIN_SERVICE, KEYCHAIN_JWT_KEY)
        .map_err(|e| AuthError::Keychain(e.to_string()))?;
    match entry.get_password() {
        Ok(jwt) => Ok(Some(jwt)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => {
            warn!("keychain read error: {e}");
            Err(AuthError::Keychain(e.to_string()))
        }
    }
}

pub fn delete_jwt() -> Result<(), AuthError> {
    let entry = keyring::Entry::new(KEYCHAIN_SERVICE, KEYCHAIN_JWT_KEY)
        .map_err(|e| AuthError::Keychain(e.to_string()))?;
    match entry.delete_credential() {
        Ok(()) => {
            debug!("deleted JWT from keychain");
            Ok(())
        }
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(AuthError::Keychain(e.to_string())),
    }
}
