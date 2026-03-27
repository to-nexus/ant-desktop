use base64::Engine;

use crate::auth::AuthError;

const EXP_GRACE_PERIOD_SECS: u64 = 60;

pub fn decode_user_id(jwt: &str) -> Result<String, AuthError> {
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

    payload
        .get("sub")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| AuthError::InvalidJwt("missing 'sub' claim".into()))
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
}
