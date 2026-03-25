use base64::Engine;

use crate::auth::AuthError;

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
}
