use ring::hmac;

#[derive(thiserror::Error, Debug)]
pub enum SignatureError {
    #[error("Invalid signature format")]
    InvalidFormat,

    #[error("Signature verification failed")]
    VerificationFailed,
}

/// Signs a payload using HMAC-SHA256 and returns a hex-encoded signature.
pub fn sign(payload: &str, key: &[u8]) -> String {
    let key = hmac::Key::new(hmac::HMAC_SHA256, key);
    let signature = hmac::sign(&key, payload.as_bytes());
    hex::encode(signature.as_ref())
}

/// Verifies an HMAC-SHA256 signature for a payload.
/// Returns true if the signature is valid, false otherwise.
pub fn verify(payload: &str, signature_hex: &str, key: &[u8]) -> Result<bool, SignatureError> {
    let signature_bytes = hex::decode(signature_hex).map_err(|_| SignatureError::InvalidFormat)?;

    let key = hmac::Key::new(hmac::HMAC_SHA256, key);

    match hmac::verify(&key, payload.as_bytes(), &signature_bytes) {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sign_and_verify() {
        let key = b"test-signing-key-should-be-secret";
        let payload = "card_id=123&member_id=456&issuer_id=789";

        let signature = sign(payload, key);
        assert!(!signature.is_empty());

        let is_valid = verify(payload, &signature, key).unwrap();
        assert!(is_valid);
    }

    #[test]
    fn test_verify_fails_with_wrong_key() {
        let key1 = b"key-one";
        let key2 = b"key-two";
        let payload = "some data";

        let signature = sign(payload, key1);
        let is_valid = verify(payload, &signature, key2).unwrap();

        assert!(!is_valid);
    }

    #[test]
    fn test_verify_fails_with_tampered_payload() {
        let key = b"test-key";
        let payload = "original data";
        let tampered = "tampered data";

        let signature = sign(payload, key);
        let is_valid = verify(tampered, &signature, key).unwrap();

        assert!(!is_valid);
    }

    #[test]
    fn test_verify_fails_with_invalid_hex() {
        let key = b"test-key";
        let payload = "data";

        let result = verify(payload, "not-valid-hex!!!", key);
        assert!(result.is_err());
    }

    #[test]
    fn test_deterministic_signatures() {
        let key = b"test-key";
        let payload = "same payload";

        let sig1 = sign(payload, key);
        let sig2 = sign(payload, key);

        // HMAC should be deterministic
        assert_eq!(sig1, sig2);
    }
}
