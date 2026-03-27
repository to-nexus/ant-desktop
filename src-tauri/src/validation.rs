use url::Url;

/// Validate a server/realtime base URL.
/// Ensures the URL has a supported scheme (http/https) and a valid host.
pub fn validate_server_url(raw: &str) -> Result<String, String> {
    let parsed = Url::parse(raw).map_err(|e| format!("invalid URL: {e}"))?;
    match parsed.scheme() {
        "https" | "http" => {}
        s => return Err(format!("unsupported scheme: {s}")),
    }
    if parsed.host_str().is_none() {
        return Err("missing host".into());
    }
    Ok(parsed.to_string().trim_end_matches('/').to_string())
}

/// Validate a web URL (for opening in a browser).
/// Empty string is allowed (clears the custom URL).
pub fn validate_web_url(raw: &str) -> Result<String, String> {
    if raw.is_empty() {
        return Ok(String::new());
    }
    let parsed = Url::parse(raw).map_err(|e| format!("invalid URL: {e}"))?;
    match parsed.scheme() {
        "https" | "http" => {}
        s => return Err(format!("unsupported scheme: {s}")),
    }
    Ok(parsed.to_string().trim_end_matches('/').to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_https_server() {
        let result = validate_server_url("https://ant.crosstoken.io").unwrap();
        assert!(result.starts_with("https://"));
    }

    #[test]
    fn valid_http_localhost() {
        let result = validate_server_url("http://127.0.0.1:4101").unwrap();
        assert!(result.contains("127.0.0.1"));
    }

    #[test]
    fn reject_ftp_scheme() {
        assert!(validate_server_url("ftp://example.com").is_err());
    }

    #[test]
    fn reject_javascript_scheme() {
        assert!(validate_server_url("javascript:alert(1)").is_err());
    }

    #[test]
    fn reject_empty_url() {
        assert!(validate_server_url("").is_err());
    }

    #[test]
    fn web_url_empty_allowed() {
        assert_eq!(validate_web_url("").unwrap(), "");
    }

    #[test]
    fn web_url_valid() {
        let result = validate_web_url("https://ant.crosstoken.io").unwrap();
        assert!(result.starts_with("https://"));
    }

    #[test]
    fn web_url_reject_data_scheme() {
        assert!(validate_web_url("data:text/html,<h1>hi</h1>").is_err());
    }
}
