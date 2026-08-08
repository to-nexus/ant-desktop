use serde::Serialize;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::auth::jwt::DesktopClaims;
use crate::constants::{DEEPLINK_PENDING_TTL_SECS, PAIRING_TTL_SECS};

/// Upper bound on any attacker-supplied label we render.
const DISPLAY_MAX_CHARS: usize = 96;

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum ConnectionStatus {
    Initial,
    Connecting,
    Connected,
    Reconnecting,
    AuthRequired,
    Disconnected,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum FigmaStatus {
    Available,
    Unavailable,
    Unknown,
}

/// Flatten a string we did not author into something safe to show in an
/// approval prompt: control characters stripped (a newline would let a token's
/// `email` claim forge extra lines of prompt text) and length bounded.
pub fn sanitize_display(raw: &str) -> Option<String> {
    let cleaned: String = raw
        .chars()
        .filter(|c| !c.is_control())
        .take(DISPLAY_MAX_CHARS)
        .collect();
    let trimmed = cleaned.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// How an account is named in the UI: its address when the token carried one,
/// otherwise the opaque `sub`. Both go through `sanitize_display` — neither is
/// a string this app authored.
fn account_label(claims: Option<&DesktopClaims>) -> Option<String> {
    let claims = claims?;
    claims
        .email
        .as_deref()
        .and_then(sanitize_display)
        .or_else(|| sanitize_display(&claims.sub))
}

/// A deep-link connect request awaiting explicit user confirmation. Parked
/// here instead of being auto-applied so a web page cannot silently swap the
/// app's server/account via the `ant-desktop://connect` scheme.
#[derive(Debug, Clone)]
pub struct PendingConnect {
    pub token: String,
    pub server: String,
    pub claims: Option<DesktopClaims>,
    /// Deadline, not a birth time — the TTL is applied once, here, so no reader
    /// has to know the constant.
    pub expires_at: Instant,
}

impl PendingConnect {
    pub fn new(token: String, server: String, claims: Option<DesktopClaims>) -> Self {
        Self {
            token,
            server,
            claims,
            expires_at: Instant::now() + Duration::from_secs(DEEPLINK_PENDING_TTL_SECS),
        }
    }

    /// A parked request holds a bearer token, so it stops being approvable
    /// rather than waiting indefinitely for a click.
    pub fn is_expired(&self) -> bool {
        Instant::now() >= self.expires_at
    }
}

/// A pairing this Desktop started: the user asked us to open the web app, and
/// we handed the browser a one-shot nonce to echo back in the deep link. A
/// matching echo proves the link originated from a flow the user began *here*,
/// which is what makes it safe to apply without a confirmation prompt.
#[derive(Debug, Clone)]
pub struct PendingPairing {
    pub nonce: String,
    /// Host of the web app we opened — also widens the deep-link server policy
    /// to that host, so self-hosted deployments pair without pre-configuration.
    pub web_host: Option<String>,
    pub expires_at: Instant,
}

impl PendingPairing {
    pub fn new(nonce: String, web_host: Option<String>) -> Self {
        Self {
            nonce,
            web_host,
            expires_at: Instant::now() + Duration::from_secs(PAIRING_TTL_SECS),
        }
    }

    pub fn is_expired(&self) -> bool {
        Instant::now() >= self.expires_at
    }

    pub fn matches(&self, nonce: &str) -> bool {
        !self.is_expired() && self.nonce == nonce
    }
}

#[derive(Debug)]
pub struct AppState {
    pub connection_status: ConnectionStatus,
    pub figma_status: FigmaStatus,
    pub server_url: Option<String>,
    pub web_url: Option<String>,
    pub jwt: Option<String>,
    pub account: Option<DesktopClaims>,
    pub machine_id: String,
    pub last_heartbeat: Option<Instant>,
    pub mcp_request_count: u64,
    pub last_mcp_request: Option<Instant>,
    pub pending_connect: Option<PendingConnect>,
    pub pending_pairing: Option<PendingPairing>,
}

impl AppState {
    /// `sub` of the connected account — what the bridge registers under.
    pub fn user_id(&self) -> Option<String> {
        self.account.as_ref().map(|c| c.sub.clone())
    }

    /// Best label for the connected account: address if the token carried one,
    /// otherwise the opaque id.
    pub fn account_label(&self) -> Option<String> {
        account_label(self.account.as_ref())
    }

    pub fn new() -> Self {
        Self {
            connection_status: ConnectionStatus::Initial,
            figma_status: FigmaStatus::Unknown,
            server_url: None,
            web_url: None,
            jwt: None,
            account: None,
            machine_id: uuid::Uuid::new_v4().to_string(),
            last_heartbeat: None,
            mcp_request_count: 0,
            last_mcp_request: None,
            pending_connect: None,
            pending_pairing: None,
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

pub type SharedAppState = Arc<Mutex<AppState>>;

/// What the UI is allowed to see about a parked connect request.
///
/// The token is **never** in here. Everything else exists so the approval
/// prompt can state what is actually being changed — which server *and whose
/// account* — instead of asking the user to vouch for a bare URL.
///
/// Exposing this through the polled snapshot (rather than only through the
/// `auth-connect-request` event) is also what makes a cold-start deep link
/// survive: the request is state, so the UI finds it whenever it mounts.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PendingConnectView {
    pub server: String,
    pub account: Option<String>,
    pub current_account: Option<String>,
    pub is_account_switch: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppStateSnapshot {
    pub connection_status: ConnectionStatus,
    pub figma_status: FigmaStatus,
    pub server_url: Option<String>,
    pub web_url: Option<String>,
    pub machine_id: String,
    pub mcp_request_count: u64,
    pub last_heartbeat_ago_ms: Option<u64>,
    pub last_mcp_request_ago_ms: Option<u64>,
    pub pending_connect: Option<PendingConnectView>,
}

impl PendingConnectView {
    fn from_pending(pending: &PendingConnect, state: &AppState) -> Self {
        let account = account_label(pending.claims.as_ref());
        let pending_sub = pending.claims.as_ref().map(|c| c.sub.as_str());
        let current_sub = state.account.as_ref().map(|c| c.sub.as_str());
        Self {
            server: pending.server.clone(),
            account,
            current_account: state.account_label(),
            // Only a *known* mismatch warrants the warning — an unreadable
            // token (local mode's `local` sentinel) must not read as a switch.
            is_account_switch: match (pending_sub, current_sub) {
                (Some(a), Some(b)) => a != b,
                _ => false,
            },
        }
    }
}

impl AppStateSnapshot {
    pub fn from_state(state: &AppState) -> Self {
        let now = Instant::now();
        Self {
            pending_connect: state
                .pending_connect
                .as_ref()
                .filter(|p| !p.is_expired())
                .map(|p| PendingConnectView::from_pending(p, state)),
            connection_status: state.connection_status.clone(),
            figma_status: state.figma_status.clone(),
            server_url: state.server_url.clone(),
            web_url: state.web_url.clone(),
            machine_id: state.machine_id.clone(),
            mcp_request_count: state.mcp_request_count,
            last_heartbeat_ago_ms: state
                .last_heartbeat
                .map(|t| now.duration_since(t).as_millis() as u64),
            last_mcp_request_ago_ms: state
                .last_mcp_request
                .map(|t| now.duration_since(t).as_millis() as u64),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn claims(sub: &str, email: Option<&str>) -> DesktopClaims {
        DesktopClaims {
            sub: sub.to_string(),
            email: email.map(|s| s.to_string()),
        }
    }

    fn pending(server: &str, claims: Option<DesktopClaims>) -> PendingConnect {
        PendingConnect::new("jwt".into(), server.into(), claims)
    }

    #[test]
    fn sanitize_display_strips_control_chars() {
        // A newline would forge an extra line in the approval prompt.
        assert_eq!(
            sanitize_display("evil@test\n\nApproved server: https://ant.crosstoken.io").as_deref(),
            Some("evil@testApproved server: https://ant.crosstoken.io")
        );
        assert_eq!(sanitize_display("a\tb\r\nc").as_deref(), Some("abc"));
    }

    #[test]
    fn sanitize_display_caps_length_and_rejects_blank() {
        let long = "x".repeat(500);
        assert_eq!(sanitize_display(&long).unwrap().chars().count(), 96);
        assert_eq!(sanitize_display(""), None);
        assert_eq!(sanitize_display("   \n  "), None);
    }

    #[test]
    fn view_prefers_email_then_falls_back_to_sub() {
        let state = AppState::new();
        let with_email = PendingConnectView::from_pending(
            &pending("https://x.test", Some(claims("u-1", Some("a@b.test")))),
            &state,
        );
        assert_eq!(with_email.account.as_deref(), Some("a@b.test"));

        let no_email = PendingConnectView::from_pending(
            &pending("https://x.test", Some(claims("u-1", None))),
            &state,
        );
        assert_eq!(no_email.account.as_deref(), Some("u-1"));
    }

    #[test]
    fn view_flags_account_switch_only_on_known_mismatch() {
        let mut state = AppState::new();
        state.account = Some(claims("victim", Some("victim@test")));

        let attacker = PendingConnectView::from_pending(
            &pending(
                "https://ant.crosstoken.io",
                Some(claims("attacker", Some("evil@test"))),
            ),
            &state,
        );
        assert!(attacker.is_account_switch);
        assert_eq!(attacker.current_account.as_deref(), Some("victim@test"));

        let same = PendingConnectView::from_pending(
            &pending(
                "https://ant.crosstoken.io",
                Some(claims("victim", Some("victim@test"))),
            ),
            &state,
        );
        assert!(!same.is_account_switch);

        // Unreadable token (local mode's `local` sentinel) — unknown, not a switch.
        let unknown =
            PendingConnectView::from_pending(&pending("http://127.0.0.1:4101", None), &state);
        assert!(!unknown.is_account_switch);
        assert_eq!(unknown.account, None);
    }

    #[test]
    fn snapshot_hides_expired_pending_connect() {
        let mut state = AppState::new();
        state.pending_connect = Some(pending(
            "https://ant.crosstoken.io",
            Some(claims("u-1", None)),
        ));
        assert!(AppStateSnapshot::from_state(&state)
            .pending_connect
            .is_some());

        state.pending_connect.as_mut().unwrap().expires_at = Instant::now();
        assert!(AppStateSnapshot::from_state(&state)
            .pending_connect
            .is_none());
    }

    #[test]
    fn pairing_matches_only_fresh_exact_nonce() {
        let mut pairing = PendingPairing::new("nonce-abc".into(), Some("ant.crosstoken.io".into()));
        assert!(pairing.matches("nonce-abc"));
        assert!(!pairing.matches("nonce-abd"));
        assert!(!pairing.matches(""));

        pairing.expires_at = Instant::now();
        assert!(!pairing.matches("nonce-abc"));
    }
}
