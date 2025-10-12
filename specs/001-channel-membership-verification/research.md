# Technical Research: Channel Membership Verification System

**Feature**: 001-channel-membership-verification
**Date**: 2025-10-12
**Status**: Phase 0 Complete

## Overview

This document resolves all "NEEDS CLARIFICATION" items from the Technical Context and provides technology decisions for the Rust-based membership card system.

---

## Decision 1: Web Framework

**Decision**: **Axum**

**Rationale**:
- Built on top of `tokio` and `hyper` - modern async runtime
- Excellent type safety with compile-time routing validation
- Minimal boilerplate compared to Actix-web
- Strong ecosystem compatibility (works well with `tower` middleware)
- Active development by Tokio team
- Better ergonomics for OAuth flows (async/await friendly)

**Alternatives Considered**:
- **Actix-web**: More mature, but more complex actor model. Overkill for this use case.
- **Rocket**: Easier for beginners, but async support still maturing. Compile times can be slower.
- **Warp**: Too low-level, requires more manual wiring.

**Best Practices**:
- Use `axum::Router` for route organization
- Leverage `axum::Extension` for dependency injection (DB pool, config)
- Use `tower::ServiceBuilder` for middleware stacking
- Implement custom extractors for session validation

---

## Decision 2: OAuth Client Library

**Decision**: **`oauth2` crate**

**Rationale**:
- De facto standard for OAuth 2.0 in Rust ecosystem
- Platform-agnostic, works with any OAuth provider
- Supports authorization code flow (required for YouTube/Twitch)
- PKCE support for enhanced security
- Well-documented with examples for Google and Twitch

**Alternatives Considered**:
- **Custom implementation**: Too error-prone, not worth the risk
- **Platform-specific SDKs**: None exist for Rust; would need to use FFI

**Best Practices**:
- Store OAuth tokens encrypted at rest in PostgreSQL
- Use `secrecy` crate to prevent accidental token logging
- Implement token refresh logic before expiration
- Set minimal OAuth scopes:
  - YouTube: `https://www.googleapis.com/auth/youtube.readonly` (for channel membership checks)
  - Twitch: `user:read:subscriptions` (for subscription data)

**Integration Notes**:
- YouTube Data API v3: Use `/channels` and `/members` endpoints after OAuth
- Twitch API: Use `/subscriptions` endpoint with authenticated user token
- Both require registering OAuth applications and obtaining client ID/secret

---

## Decision 3: Database ORM/Query Builder

**Decision**: **SQLx** (compile-time checked queries, no ORM)

**Rationale**:
- Compile-time verification of SQL queries against actual database schema
- No runtime ORM overhead
- Explicit SQL control (important for complex queries like revocation history)
- Excellent async support with `tokio`
- Migration support built-in
- Lighter weight than Diesel for this project's scope

**Alternatives Considered**:
- **Diesel**: More mature, but heavier ORM approach. Compile times are slower. Less ergonomic for async.
- **SeaORM**: Good async support, but adds abstraction layer we don't need. Queries can be less predictable.

**Best Practices**:
- Use `sqlx::query!` macro for compile-time verified queries
- Organize queries in dedicated modules per entity
- Use transactions for multi-step operations (e.g., card issuance + logging)
- Implement connection pooling with `sqlx::PgPool`
- Use `SERIAL` or `UUID` for primary keys (UUID preferred for distributed security)

**Schema Strategy**:
- Use migrations in `migrations/` directory
- Run `sqlx migrate run` before application start
- Version control all migration files

---

## Decision 4: QR Code Generation

**Decision**: **`qrcode` crate**

**Rationale**:
- Pure Rust implementation, no C dependencies
- Supports multiple output formats (PNG, SVG, terminal)
- Configurable error correction levels
- Simple API, well-maintained
- Can generate data URLs for direct embedding in HTML

**Alternatives Considered**:
- **`fast_qr`**: Faster, but less flexible output formats
- **External service (e.g., QR code API)**: Adds network dependency and latency

**Best Practices**:
- Use error correction level `Medium` or `High` for better scan reliability
- Encode card data as JSON with signature for tamper detection
- Consider data size limits (QR codes have capacity constraints)
- Generate SVG for web display, PNG for download

**QR Code Payload Format** (for 數位皮夾 compatibility):
```json
{
  "card_id": "uuid",
  "issuer_id": "uuid",
  "member_platform_id": "string",
  "platform": "youtube|twitch",
  "membership_level": "string",
  "issued_at": "ISO8601 timestamp",
  "signature": "HMAC-SHA256 of above fields"
}
```

**Note**: Coordinate with 數位皮夾 team to confirm exact format requirements (DEP-001, DEP-006).

---

## Decision 5: HTTP Client for Platform APIs

**Decision**: **`reqwest`**

**Rationale**:
- Most popular HTTP client in Rust ecosystem
- Excellent async support with `tokio`
- Built-in JSON serialization/deserialization with `serde`
- Connection pooling and retry logic support
- TLS support out of the box
- Well-documented API for OAuth bearer token headers

**Alternatives Considered**:
- **`hyper`**: Lower-level, requires more manual HTTP handling
- **`ureq`**: Synchronous only, not suitable for async web server

**Best Practices**:
- Create singleton `reqwest::Client` with connection pooling
- Set reasonable timeouts (5s connect, 30s read)
- Implement retry logic with exponential backoff for transient failures
- Handle rate limiting responses from platforms (HTTP 429)
- Use `reqwest::header::AUTHORIZATION` for OAuth bearer tokens

**Platform API Integration**:

### YouTube Data API v3
- Base URL: `https://www.googleapis.com/youtube/v3`
- Membership check endpoint: `/members?part=snippet&channelId={channel_id}`
- Requires API key + OAuth token
- Rate limit: 10,000 units/day (check costs ~1 unit)

### Twitch API
- Base URL: `https://api.twitch.tv/helix`
- Subscription check: `/subscriptions?broadcaster_id={broadcaster_id}&user_id={user_id}`
- Requires OAuth token + Client ID header
- Rate limit: 800 requests/minute per client ID

---

## Decision 6: Frontend Technology

**Decision**: **Minimal vanilla HTML/JS with QR scanner library**

**Rationale**:
- Frontend needs are minimal (OAuth redirects + QR scanning)
- No need for React/Vue/Svelte complexity
- Use `html5-qrcode` JavaScript library for QR scanning
- Server-side rendering with Rust template engine for simplicity

**Template Engine**: **`askama`** (Jinja2-like templates, compile-time checked)

**Alternatives Considered**:
- **SPA framework (React/Vue)**: Overkill for this use case
- **`tera`** (runtime templates): Less type-safe than `askama`

**Best Practices**:
- Serve static files from `web/static/` via Axum's `ServeDir`
- Use `html5-qrcode` for camera-based QR scanning
- Implement progressive enhancement (fallback if camera unavailable)

---

## Decision 7: Cron Job Strategy

**Decision**: **Integrated scheduler with `tokio-cron-scheduler` crate**

**Rationale**:
- Single binary deployment (no separate cron process)
- Async-friendly with `tokio`
- Configurable schedules via code or config
- Easier to test than external cron

**Alternatives Considered**:
- **External cron + separate binary**: More complex deployment, inter-process communication needed
- **Database-based job queue (e.g., `sqlx` with polling)**: Over-engineered for periodic task

**Best Practices**:
- Run subscription checker every 6 hours (configurable)
- Batch process cards in chunks to handle rate limits
- Log job runs to database for monitoring
- Implement graceful shutdown to avoid mid-job interruption

---

## Decision 8: Security & Token Storage

**Decision**: Encrypt OAuth tokens at rest using **`ring` or `orion` crate**

**Rationale**:
- OAuth tokens are sensitive (FR-030 requirement)
- Encryption key from environment variable
- Use AES-256-GCM for authenticated encryption

**Best Practices**:
- Store encryption key in environment variable (never in code)
- Use `secrecy::Secret` wrapper to prevent accidental logging
- Implement key rotation strategy (future enhancement)
- Consider using HashiCorp Vault or similar in production

---

## Decision 9: Session Management

**Decision**: **`tower-sessions`** with PostgreSQL backend

**Rationale**:
- Integrates with Axum/Tower middleware
- PostgreSQL backend keeps sessions durable
- Supports secure cookies with HMAC signing
- Handles session expiration automatically

**Alternatives Considered**:
- **JWT tokens**: Stateless, but harder to revoke on logout
- **Redis sessions**: Adds infrastructure dependency

**Best Practices**:
- Set secure cookie flags (`HttpOnly`, `Secure`, `SameSite=Lax`)
- Session expiry: 30 days for member sessions, 7 days for organizer sessions
- Store minimal data in session (user ID, platform, role)

---

## Decision 10: Testing Strategy

**Test Framework**: **`cargo test`** with helper crates

**Integration Testing**:
- Use `sqlx::test` attribute for database-backed tests
- Spin up test PostgreSQL instance via Docker or in-memory SQLite
- Mock platform APIs with `wiremock` crate

**Unit Testing**:
- Test services in isolation with dependency injection
- Mock external dependencies (HTTP client, database)

**Best Practices**:
- Write tests before implementation (TDD if constitution requires)
- Aim for >80% code coverage on business logic
- Use `cargo-nextest` for faster test execution

---

## Decision 11: Configuration Management

**Decision**: **`config` crate + environment variables**

**Rationale**:
- Supports multiple config sources (files, env vars)
- Type-safe config structs with `serde`
- 12-factor app principles (config via environment)

**Configuration Structure**:
```rust
struct Config {
    database_url: String,
    youtube_client_id: String,
    youtube_client_secret: SecretString,
    twitch_client_id: String,
    twitch_client_secret: SecretString,
    session_secret: SecretString,
    encryption_key: SecretString,
    base_url: String,  // For OAuth redirects
    cron_schedule: String,  // e.g., "0 */6 * * *"
}
```

---

## Decision 12: Logging & Observability

**Decision**: **`tracing` crate** with structured logging

**Rationale**:
- De facto standard for async Rust applications
- Structured logging (better than plain text)
- Integrates with Axum middleware
- Supports multiple outputs (stdout, files, monitoring services)

**Best Practices**:
- Use `tracing::instrument` on all service functions
- Log OAuth events, card issuance, verification attempts
- Implement request ID tracking via middleware
- Use log levels appropriately (ERROR for failures, INFO for key events)

---

## Decision 13: Error Handling

**Decision**: **`thiserror` for error definitions + custom error types**

**Rationale**:
- Reduces boilerplate for error type definitions
- Compatible with `anyhow` for application-level error propagation
- Clear error messages for API responses

**Error Categories**:
- `OAuthError`: OAuth flow failures
- `PlatformApiError`: YouTube/Twitch API failures
- `DatabaseError`: Database operation failures
- `ValidationError`: Input validation failures
- `NotFoundError`: Resource not found

---

## Summary of Technology Stack

| Component | Technology | Crate(s) |
|-----------|-----------|----------|
| Web Framework | Axum | `axum`, `tower`, `tower-http` |
| Database | PostgreSQL | `sqlx`, `sqlx-cli` |
| OAuth Client | OAuth2 | `oauth2` |
| HTTP Client | reqwest | `reqwest` |
| QR Code | qrcode | `qrcode` |
| Template Engine | Askama | `askama`, `askama_axum` |
| Session Management | tower-sessions | `tower-sessions`, `tower-sessions-sqlx-store` |
| Cron Scheduler | tokio-cron-scheduler | `tokio-cron-scheduler` |
| Encryption | ring or orion | `ring` or `orion` |
| Configuration | config | `config`, `secrecy` |
| Logging | tracing | `tracing`, `tracing-subscriber` |
| Error Handling | thiserror | `thiserror`, `anyhow` |
| Testing | cargo test | `tokio-test`, `wiremock`, `sqlx::test` |
| Serialization | serde | `serde`, `serde_json` |

---

## Next Steps

1. **Phase 1**: Generate `data-model.md` with PostgreSQL schema design
2. **Phase 1**: Generate API contracts in `contracts/` directory
3. **Phase 1**: Generate `quickstart.md` for local development setup
4. **Phase 2**: Generate `tasks.md` for implementation plan

---

## Open Questions

1. **數位皮夾 QR format**: Need confirmation on exact QR code payload format (DEP-001, DEP-006)
2. **Platform API limits**: Need to verify actual rate limits and costs for production usage
3. **"只有電風扇" platform**: Need clarification on what this platform is and its API availability
4. **Deployment target**: Containerized (Docker) or bare metal? Cloud provider?
5. **SSL certificates**: Self-hosted or managed (Let's Encrypt)?

These questions should be resolved before Phase 2 (task generation).
