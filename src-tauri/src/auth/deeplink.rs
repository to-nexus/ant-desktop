use url::Url;

use crate::auth::AuthError;

/// The shape of an `ant-desktop://connect` URI, parsed but **not yet trusted**.
///
/// `server_raw` is deliberately unvalidated here: whether an origin may be
/// named by a deep link is a policy question that needs app state (the
/// configured server, any pairing in flight), which the parser does not have.
/// `lib::process_deep_link_url` owns that decision via
/// `validation::validate_deeplink_server_url`.
pub struct DeepLinkParams {
    pub token: String,
    pub server_raw: String,
    /// Nonce echoed back by the web app, present only when this Desktop started
    /// the pairing. Absent for a link built by a page we never handed a nonce.
    pub state: Option<String>,
}

pub fn parse_connect_url(url_str: &str) -> Result<DeepLinkParams, AuthError> {
    let url = Url::parse(url_str)
        .map_err(|e| AuthError::InvalidDeepLink(format!("URL parse error: {e}")))?;

    if url.scheme() != "ant-desktop" {
        return Err(AuthError::InvalidDeepLink(format!(
            "unexpected scheme: {}",
            url.scheme()
        )));
    }

    if url.host_str() != Some("connect") {
        return Err(AuthError::InvalidDeepLink(format!(
            "unexpected host: {:?} (expected 'connect')",
            url.host_str()
        )));
    }

    let token = url
        .query_pairs()
        .find(|(k, _)| k == "token")
        .map(|(_, v)| v.to_string())
        .ok_or_else(|| AuthError::InvalidDeepLink("missing 'token' parameter".into()))?;

    let server_raw = url
        .query_pairs()
        .find(|(k, _)| k == "server")
        .map(|(_, v)| v.to_string())
        .ok_or_else(|| AuthError::InvalidDeepLink("missing 'server' parameter".into()))?;

    let state = url
        .query_pairs()
        .find(|(k, _)| k == "state")
        .map(|(_, v)| v.to_string())
        .filter(|s| !s.is_empty());

    Ok(DeepLinkParams {
        token,
        server_raw,
        state,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_deep_link() {
        let params = parse_connect_url(
            "ant-desktop://connect?token=test-jwt-123&server=https://ant.crosstoken.io",
        )
        .unwrap();
        assert_eq!(params.token, "test-jwt-123");
        assert_eq!(params.server_raw, "https://ant.crosstoken.io");
        assert_eq!(params.state, None);
    }

    #[test]
    fn parse_local_server() {
        let params =
            parse_connect_url("ant-desktop://connect?token=jwt&server=http://127.0.0.1:4101")
                .unwrap();
        assert_eq!(params.server_raw, "http://127.0.0.1:4101");
    }

    #[test]
    fn parse_pairing_state() {
        let params = parse_connect_url(
            "ant-desktop://connect?token=jwt&server=http://127.0.0.1:4101&state=nonce-abc",
        )
        .unwrap();
        assert_eq!(params.state.as_deref(), Some("nonce-abc"));
    }

    /// An empty `state=` is the same as no state — it must not match a pairing.
    #[test]
    fn parse_empty_state_is_absent() {
        let params = parse_connect_url(
            "ant-desktop://connect?token=jwt&server=http://127.0.0.1:4101&state=",
        )
        .unwrap();
        assert_eq!(params.state, None);
    }

    #[test]
    fn parse_missing_token() {
        assert!(parse_connect_url("ant-desktop://connect?server=http://localhost:4101").is_err());
    }

    #[test]
    fn parse_missing_server() {
        assert!(parse_connect_url("ant-desktop://connect?token=abc").is_err());
    }

    #[test]
    fn parse_wrong_scheme() {
        assert!(parse_connect_url("http://connect?token=a&server=b").is_err());
    }
}
