pub mod deeplink;
pub mod jwt;
pub mod keychain;

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("no JWT token found")]
    NoToken,

    #[error("invalid JWT format: {0}")]
    InvalidJwt(String),

    #[error("keychain error: {0}")]
    Keychain(String),

    #[error("invalid deep link: {0}")]
    InvalidDeepLink(String),
}
