use url::Url;

use crate::constants::DEFAULT_CLOUD_URL;

/// Hosts a **deep-link** `server` may name, on top of the two origins that are
/// always trusted (loopback and the canonical cloud origin).
///
/// Built fresh per deep link by the caller — the policy is state, not a
/// constant, because self-hosted deployments are legitimate targets and we
/// learn about them only from the user's own actions.
#[derive(Debug, Default, Clone)]
pub struct DeepLinkServerPolicy {
    /// Origin of the server the user configured through Settings (or approved
    /// earlier). Compared as a full origin — a persisted realtime base URL is
    /// exactly what a deep link from that deployment reproduces.
    pub configured_origin: Option<String>,
    /// Host of the web app this Desktop opened for pairing. Host-only, not
    /// origin: a self-hosted deployment may serve the web app and the realtime
    /// server on different ports behind one hostname.
    pub paired_web_host: Option<String>,
}

fn canonical_cloud_origin() -> Option<String> {
    Url::parse(DEFAULT_CLOUD_URL)
        .ok()
        .map(|u| u.origin().ascii_serialization())
}

/// `localhost`, any loopback IP literal, and the bracketed IPv6 form.
pub fn is_loopback_host(host: &str) -> bool {
    let bare = host.trim_start_matches('[').trim_end_matches(']');
    if bare.eq_ignore_ascii_case("localhost") {
        return true;
    }
    bare.parse::<std::net::IpAddr>()
        .map(|ip| ip.is_loopback())
        .unwrap_or(false)
}

/// Validate a server URL that arrived over the `ant-desktop://connect` scheme.
///
/// Distinct from [`validate_server_url`] on purpose. Settings is a first-party
/// surface where the user types an address they chose, so any http/https host
/// is fair game there. A deep link is **untrusted external input** — any web
/// page can trigger the scheme — and the server it names ends up receiving the
/// user's token and driving `mcp.request` into the local Figma MCP. So the
/// deep-link path takes an allowlist and refuses everything else, rather than
/// leaning on the confirmation dialog as its only trust boundary.
pub fn validate_deeplink_server_url(
    raw: &str,
    policy: &DeepLinkServerPolicy,
) -> Result<String, String> {
    let normalized = validate_server_url(raw)?;
    let parsed = Url::parse(&normalized).map_err(|e| format!("invalid URL: {e}"))?;
    let host = parsed
        .host_str()
        .ok_or_else(|| "missing host".to_string())?
        .to_ascii_lowercase();
    let origin = parsed.origin().ascii_serialization();

    let trusted = is_loopback_host(&host)
        || canonical_cloud_origin().as_deref() == Some(origin.as_str())
        || policy.configured_origin.as_deref() == Some(origin.as_str())
        || policy.paired_web_host.as_deref() == Some(host.as_str());

    if trusted {
        Ok(normalized)
    } else {
        Err(format!("untrusted deep-link server origin: {origin}"))
    }
}

/// Validate a server/realtime base URL — syntax only.
/// Ensures the URL has a supported scheme (http/https) and a valid host.
///
/// `http://` is accepted for **any** host, not just loopback — self-hosted
/// setups routinely run `ant-realtime` over plain HTTP on a LAN address, and
/// the user typing that address into Settings is the authorization.
///
/// This function deliberately carries **no origin policy**. Deep links do not
/// get to reuse it as their only gate — see [`validate_deeplink_server_url`].
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

    fn policy(configured: Option<&str>, paired: Option<&str>) -> DeepLinkServerPolicy {
        DeepLinkServerPolicy {
            configured_origin: configured.map(|s| s.to_string()),
            paired_web_host: paired.map(|s| s.to_string()),
        }
    }

    /// One row per trust decision the deep-link gate can make.
    #[test]
    fn deeplink_server_policy_matrix() {
        let cases: &[(&str, DeepLinkServerPolicy, bool)] = &[
            // Attacker-supplied origins with nothing vouching for them.
            ("https://attacker.example", policy(None, None), false),
            ("http://attacker.example", policy(None, None), false),
            ("http://10.0.0.5:4101", policy(None, None), false),
            // Canonical cloud origin is always trusted; a look-alike is not.
            ("https://ant.crosstoken.io", policy(None, None), true),
            (
                "https://ant.crosstoken.io.evil.test",
                policy(None, None),
                false,
            ),
            // Loopback in any shape, any port — local development.
            ("http://127.0.0.1:4101", policy(None, None), true),
            ("http://localhost:4101", policy(None, None), true),
            ("http://[::1]:4101", policy(None, None), true),
            // Self-hosted escape hatch: the user already configured this origin.
            (
                "https://selfhost.example",
                policy(Some("https://selfhost.example"), None),
                true,
            ),
            // ...but only that exact origin.
            (
                "https://other.example",
                policy(Some("https://selfhost.example"), None),
                false,
            ),
            (
                "https://selfhost.example:8443",
                policy(Some("https://selfhost.example"), None),
                false,
            ),
            // Pairing binds the host of the web app Desktop itself opened.
            (
                "https://selfhost.example:8443",
                policy(None, Some("selfhost.example")),
                true,
            ),
            (
                "https://attacker.example",
                policy(None, Some("selfhost.example")),
                false,
            ),
        ];

        for (raw, pol, expected) in cases {
            let allowed = validate_deeplink_server_url(raw, pol).is_ok();
            assert_eq!(
                allowed, *expected,
                "{raw} with policy {pol:?}: expected allowed={expected}, got {allowed}"
            );
        }
    }

    #[test]
    fn deeplink_server_still_rejects_bad_syntax() {
        let pol = policy(None, None);
        assert!(validate_deeplink_server_url("ftp://127.0.0.1", &pol).is_err());
        assert!(validate_deeplink_server_url("javascript:alert(1)", &pol).is_err());
        assert!(validate_deeplink_server_url("", &pol).is_err());
    }
}
