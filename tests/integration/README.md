# Integration Tests

Integration tests for the VPass application. These tests verify that different components work correctly together.

## Structure

- `auth_flow_test.rs` - OAuth authentication flow tests
- `card_lifecycle_test.rs` - Card issuance and verification lifecycle tests
- `api_endpoints_test.rs` - API endpoint integration tests

## Running Integration Tests

```bash
# Run all integration tests
cargo test --test '*'

# Run specific integration test
cargo test --test auth_flow_test

# Run with output
cargo test --test '*' -- --nocapture
```

## Test Database

Integration tests use a separate test database. Ensure you have:

```bash
# Set test database URL in .env or environment
export DATABASE_URL=postgresql://postgres:password@localhost:5432/vpass_test

# Run migrations for test database
sqlx migrate run
```

## TODO

- T054: Implement authentication flow integration tests
- T055: Implement card lifecycle integration tests
- T056: Implement API endpoint integration tests
