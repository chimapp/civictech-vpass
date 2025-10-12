# Unit Tests

Unit tests for individual components and functions in the VPass application.

## Structure

Unit tests are located alongside the source code in `src/` directory using Rust's built-in test framework:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_example() {
        // Test code
    }
}
```

## Running Unit Tests

```bash
# Run all unit tests
cargo test --lib

# Run tests for specific module
cargo test --lib services::encryption

# Run with output
cargo test --lib -- --nocapture
```

## Test Coverage

```bash
# Install tarpaulin for coverage
cargo install cargo-tarpaulin

# Run coverage report
cargo tarpaulin --lib --out Html
```

## Key Test Areas

### Cryptography
- `services/encryption.rs` - AES-256-GCM encryption/decryption
- `services/signature.rs` - HMAC-SHA256 signing/verification

### Business Logic
- `services/card_issuer.rs` - Card generation logic
- `services/card_verifier.rs` - Card verification logic
- `services/membership_checker.rs` - Membership validation

### Models
- `models/*.rs` - Data model validation and constraints

## TODO

- T057: Add unit tests for encryption service
- T058: Add unit tests for signature service
- T059: Add unit tests for card issuance logic
- T060: Add unit tests for membership checker
