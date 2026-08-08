use base64::Engine;

use crate::auth::AuthError;

const EXP_GRACE_PERIOD_SECS: u64 = 60;

/// The subset of a Desktop token's payload this app reads, for **display and
/// local bookkeeping only**.
#[derive(Debug, Clone)]
pub struct DesktopClaims {
    pub sub: String,
    pub email: Option<String>,
}

/// Read the claims this app displays out of a JWT payload.
///
/// The signature is deliberately NOT verified — the Ant backend is the
/// verifying party, and this app has no key to check against. The `exp` check
/// below is an early-out for an obviously stale token, not a security control.
///
/// Consequence: **every field returned here is attacker-controlled if the token
/// is.** Never use one to authorize an action, unlock state, or decide trust.
/// They may be shown in the UI (so the user can see whose account a deep link
/// wants to attach) and used to key local storage, nothing more — and `email`
/// in particular must pass through `sanitize_display` before it reaches a
/// prompt, since it is free text chosen by whoever minted the token.
pub fn decode_claims(jwt: &str) -> Result<DesktopClaims, AuthError> {
    let parts: Vec<&str> = jwt.split('.').collect();
    if parts.len() != 3 {
        return Err(AuthError::InvalidJwt("expected 3 parts".into()));
    }

    let payload_b64 = parts[1];
    let payload_bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload_b64)
        .or_else(|_| base64::engine::general_purpose::URL_SAFE.decode(payload_b64))
        .map_err(|e| AuthError::InvalidJwt(format!("base64 decode failed: {e}")))?;

    let payload: serde_json::Value = serde_json::from_slice(&payload_bytes)
        .map_err(|e| AuthError::InvalidJwt(format!("JSON parse failed: {e}")))?;

    if let Some(exp) = payload.get("exp").and_then(|v| v.as_u64()) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        if now > exp + EXP_GRACE_PERIOD_SECS {
            return Err(AuthError::InvalidJwt("token expired".into()));
        }
    }

    let sub = payload
        .get("sub")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| AuthError::InvalidJwt("missing 'sub' claim".into()))?;

    let email = payload
        .get("email")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());

    Ok(DesktopClaims { sub, email })
}

/// `decode_claims().sub`, kept for the call sites that only need the id.
pub fn decode_user_id(jwt: &str) -> Result<String, AuthError> {
    decode_claims(jwt).map(|c| c.sub)
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;

    fn make_jwt(payload_json: &str) -> String {
        let header = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(r#"{"alg":"HS256","typ":"JWT"}"#);
        let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(payload_json);
        let signature = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode("fake-sig");
        format!("{header}.{payload}.{signature}")
    }

    #[test]
    fn decode_valid_jwt() {
        let jwt = make_jwt(r#"{"sub":"user-42","exp":9999999999}"#);
        assert_eq!(decode_user_id(&jwt).unwrap(), "user-42");
    }

    #[test]
    fn decode_missing_sub() {
        let jwt = make_jwt(r#"{"exp":9999999999}"#);
        assert!(decode_user_id(&jwt).is_err());
    }

    #[test]
    fn decode_invalid_format() {
        assert!(decode_user_id("not-a-jwt").is_err());
    }

    #[test]
    fn decode_expired_jwt() {
        let jwt = make_jwt(r#"{"sub":"user-1","exp":1000000}"#);
        let err = decode_user_id(&jwt).unwrap_err();
        assert!(err.to_string().contains("expired"));
    }

    #[test]
    fn decode_no_exp_still_works() {
        let jwt = make_jwt(r#"{"sub":"user-1"}"#);
        assert_eq!(decode_user_id(&jwt).unwrap(), "user-1");
    }

    #[test]
    fn decode_claims_reads_email() {
        let jwt = make_jwt(r#"{"sub":"user-1","email":"a@b.test","exp":9999999999}"#);
        let claims = decode_claims(&jwt).unwrap();
        assert_eq!(claims.sub, "user-1");
        assert_eq!(claims.email.as_deref(), Some("a@b.test"));
    }

    /// The BE emits `email: ''` when the account has no address — that must read
    /// as absent, not as an empty label in the confirmation prompt.
    #[test]
    fn decode_claims_treats_empty_email_as_absent() {
        let jwt = make_jwt(r#"{"sub":"user-1","email":"","exp":9999999999}"#);
        assert_eq!(decode_claims(&jwt).unwrap().email, None);
    }

    #[test]
    fn decode_claims_missing_email() {
        let jwt = make_jwt(r#"{"sub":"user-1","exp":9999999999}"#);
        assert_eq!(decode_claims(&jwt).unwrap().email, None);
    }

    /// Local mode hands out the literal string `local` instead of a JWT.
    #[test]
    fn decode_claims_rejects_local_sentinel() {
        assert!(decode_claims("local").is_err());
    }
}
