use url::Url;

use crate::auth::AuthError;

pub struct DeepLinkParams {
    pub token: String,
    pub server: String,
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

    let server = crate::validation::validate_server_url(&server_raw)
        .map_err(|e| AuthError::InvalidDeepLink(format!("invalid server URL: {e}")))?;

    Ok(DeepLinkParams { token, server })
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
        assert_eq!(params.server, "https://ant.crosstoken.io");
    }

    #[test]
    fn parse_local_server() {
        let params =
            parse_connect_url("ant-desktop://connect?token=jwt&server=http://127.0.0.1:4101")
                .unwrap();
        assert_eq!(params.server, "http://127.0.0.1:4101");
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
