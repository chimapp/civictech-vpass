use ring::aead::{
    Aad, BoundKey, Nonce, NonceSequence, OpeningKey, SealingKey, UnboundKey, AES_256_GCM,
};
use ring::error::Unspecified;
use ring::rand::{SecureRandom, SystemRandom};

const NONCE_LEN: usize = 12;

#[derive(thiserror::Error, Debug)]
pub enum EncryptionError {
    #[error("Encryption failed: {0}")]
    EncryptionFailed(String),

    #[error("Decryption failed: {0}")]
    DecryptionFailed(String),

    #[error("Invalid key length (expected 32 bytes)")]
    InvalidKeyLength,

    #[error("Invalid encrypted data format")]
    InvalidFormat,
}

impl From<Unspecified> for EncryptionError {
    fn from(_: Unspecified) -> Self {
        EncryptionError::EncryptionFailed("Cryptographic operation failed".to_string())
    }
}

struct CounterNonceSequence {
    nonce: [u8; NONCE_LEN],
}

impl CounterNonceSequence {
    fn new(nonce: [u8; NONCE_LEN]) -> Self {
        Self { nonce }
    }
}

impl NonceSequence for CounterNonceSequence {
    fn advance(&mut self) -> Result<Nonce, Unspecified> {
        Nonce::try_assume_unique_for_key(&self.nonce)
    }
}

/// Encrypts data using AES-256-GCM and returns a BYTEA-compatible format.
/// The nonce is prepended to the ciphertext.
///
/// Format: [nonce (12 bytes)][ciphertext + auth tag]
pub fn encrypt(data: &str, key: &[u8]) -> Result<Vec<u8>, EncryptionError> {
    if key.len() != 32 {
        return Err(EncryptionError::InvalidKeyLength);
    }

    let rng = SystemRandom::new();

    // Generate random nonce
    let mut nonce_bytes = [0u8; NONCE_LEN];
    rng.fill(&mut nonce_bytes)
        .map_err(|_| EncryptionError::EncryptionFailed("Failed to generate nonce".to_string()))?;

    let unbound_key = UnboundKey::new(&AES_256_GCM, key)?;
    let nonce_sequence = CounterNonceSequence::new(nonce_bytes);
    let mut sealing_key = SealingKey::new(unbound_key, nonce_sequence);

    let mut in_out = data.as_bytes().to_vec();
    sealing_key
        .seal_in_place_append_tag(Aad::empty(), &mut in_out)
        .map_err(|_| EncryptionError::EncryptionFailed("Sealing failed".to_string()))?;

    // Prepend nonce to ciphertext
    let mut result = Vec::with_capacity(NONCE_LEN + in_out.len());
    result.extend_from_slice(&nonce_bytes);
    result.extend_from_slice(&in_out);

    Ok(result)
}

/// Decrypts data that was encrypted with `encrypt`.
/// Expects format: [nonce (12 bytes)][ciphertext + auth tag]
pub fn decrypt(encrypted: &[u8], key: &[u8]) -> Result<String, EncryptionError> {
    if key.len() != 32 {
        return Err(EncryptionError::InvalidKeyLength);
    }

    if encrypted.len() < NONCE_LEN {
        return Err(EncryptionError::InvalidFormat);
    }

    // Extract nonce from the beginning
    let mut nonce_bytes = [0u8; NONCE_LEN];
    nonce_bytes.copy_from_slice(&encrypted[..NONCE_LEN]);

    let unbound_key = UnboundKey::new(&AES_256_GCM, key)?;
    let nonce_sequence = CounterNonceSequence::new(nonce_bytes);
    let mut opening_key = OpeningKey::new(unbound_key, nonce_sequence);

    let mut in_out = encrypted[NONCE_LEN..].to_vec();
    let decrypted = opening_key
        .open_in_place(Aad::empty(), &mut in_out)
        .map_err(|_| EncryptionError::DecryptionFailed("Opening failed".to_string()))?;

    String::from_utf8(decrypted.to_vec())
        .map_err(|_| EncryptionError::DecryptionFailed("Invalid UTF-8".to_string()))
}

/// Helper to derive a 32-byte key from a string (e.g., from environment variable).
/// Uses SHA-256 to ensure we always get exactly 32 bytes.
pub fn derive_key(key_string: &str) -> [u8; 32] {
    use ring::digest;

    let hash = digest::digest(&digest::SHA256, key_string.as_bytes());
    let mut key = [0u8; 32];
    key.copy_from_slice(hash.as_ref());
    key
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let key = derive_key("test-encryption-key-32-bytes-minimum");
        let plaintext = "Hello, World!";

        let encrypted = encrypt(plaintext, &key).unwrap();
        let decrypted = decrypt(&encrypted, &key).unwrap();

        assert_eq!(plaintext, decrypted);
    }

    #[test]
    fn test_encryption_is_non_deterministic() {
        let key = derive_key("test-encryption-key-32-bytes-minimum");
        let plaintext = "Same text";

        let encrypted1 = encrypt(plaintext, &key).unwrap();
        let encrypted2 = encrypt(plaintext, &key).unwrap();

        // Different nonces should produce different ciphertexts
        assert_ne!(encrypted1, encrypted2);

        // But both should decrypt to the same plaintext
        assert_eq!(decrypt(&encrypted1, &key).unwrap(), plaintext);
        assert_eq!(decrypt(&encrypted2, &key).unwrap(), plaintext);
    }

    #[test]
    fn test_wrong_key_fails() {
        let key1 = derive_key("key-one");
        let key2 = derive_key("key-two");
        let plaintext = "Secret data";

        let encrypted = encrypt(plaintext, &key1).unwrap();
        let result = decrypt(&encrypted, &key2);

        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_key_length() {
        let short_key = [0u8; 16];
        let result = encrypt("test", &short_key);

        assert!(matches!(result, Err(EncryptionError::InvalidKeyLength)));
    }
}
