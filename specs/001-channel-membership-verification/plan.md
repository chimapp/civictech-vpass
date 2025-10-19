# Implementation Plan: Channel Membership Verification Card System (MVP)

**Branch**: `001-channel-membership-verification` | **Date**: 2025-10-12 | **Spec**: [spec.md](./spec.md)  
**Input**: MVP feature specification scoped to YouTube member-side issuance.

## Summary

Deliver a production-ready slice that lets YouTube channel members authenticate, prove their active membership by posting a comment on a members-only verification video, and receive a digitally signed QR code compatible with 數位皮夾. Organizer verification, automated revocation, and multi-platform support are deferred to later specs, so this plan concentrates on the member issuance flow, secure token handling, and minimal templating needed to guide the user through the process.

## Technical Context

- **Language/Version**: Rust (stable, latest).
- **Primary crates**: Axum (web), SQLx (PostgreSQL, async), `oauth2` + Reqwest (OAuth + HTTP), `qrcode` (QR generation), Askama (HTML templates), tower-sessions + tower-sessions-sqlx-store (session management), Serde, Tracing, Config, Secrecy, Ring (AEAD for token encryption), ThisError/Anyhow for error handling.
- **Storage**: PostgreSQL for issuers, OAuth sessions, membership cards.
- **Runtime**: Tokio async runtime; deploy as single binary (no background workers needed yet).
- **Testing**: `cargo test` (unit + integration) once scaffolding exists, though automated tests may be stubbed in later milestone.
- **Target Platform**: Linux container.

## Project Structure (MVP)

```
specs/001-channel-membership-verification/
├── plan.md
├── spec.md
├── research.md
├── data-model.md
├── quickstart.md
└── contracts/

src/
├── main.rs                      # Axum app bootstrap
├── config.rs                    # Config loader (env + defaults)
├── error.rs                     # Application error types
├── db/
│   └── mod.rs                   # SQLx connection pool helpers
├── models/
│   ├── mod.rs
│   ├── issuer.rs
│   ├── oauth_session.rs
│   ├── member.rs                # Optional cached profile info
│   └── card.rs
├── services/
│   ├── mod.rs
│   ├── encryption.rs            # AEAD helpers for tokens
│   ├── signature.rs             # HMAC signing for QR payloads
│   ├── oauth/
│   │   ├── mod.rs
│   │   └── youtube.rs
│   ├── comment_verifier.rs      # Members-only comment lookup & validation
│   ├── card_issuer.rs           # Core issuance workflow
│   └── qr_generator.rs          # Payload assembly + QR rendering
├── api/
│   ├── mod.rs
│   ├── auth.rs                  # Login/callback/logout/session endpoints
│   ├── cards.rs                 # Claim/list/show/qr routes
│   └── middleware/
│       ├── mod.rs
│       ├── session.rs
│       └── auth.rs              # Role enforcement (member-only)
├── web/
│   └── templates/
│       ├── oauth_callback.html
│       └── claim_card.html
└── lib.rs                       # Optional for shared test helpers

migrations/
└── 2025XXXXXXXX_create_initial_schema.sql

tests/                           # Placeholder for future integration/unit tests
├── integration/
└── unit/

Cargo.toml, .env.example, docker-compose.yml
```

## Delivery Phases

1. **Phase 1 — Project Setup**
   - Initialize the Rust workspace (`cargo init`).
   - Establish base directories, `.gitignore`, and Rust toolchain configuration.
   - Configure `Cargo.toml` with Axum, SQLx (postgres + runtime-tokio-rustls), oauth2, Reqwest, tower-sessions, qrcode, Askama, Serde, Tracing, Config, Ring, Secrecy, ThisError, Anyhow.
   - Provide infrastructure helpers (.env.example, docker-compose with Postgres).

2. **Phase 2 — Foundation**
   - Configure application settings loading (`config.rs`).
   - Establish SQLx connection pool module and baseline migrations (issuers, oauth_sessions, membership_cards, members).
   - Implement domain models with SQLx queries.
   - Add error handling, encryption utilities (AES-256-GCM), signature helper (HMAC-SHA256), session middleware, and authentication middleware.
   - Bootstrap Axum app with tracing, shared state injection, and graceful shutdown.

3. **Phase 3 — User Story 1 (YouTube Card Issuance)**
   - Implement YouTube OAuth client + coordinator (state/PKCE, token exchange, refresh).
   - Implement comment verification service (YouTube comment API, validation rules, rate-limit aware).
   - Build QR generator (payload assembly, signing, qrcode rendering).
   - Implement issuance service (duplicate prevention, DB writes, error surfacing).
   - Expose auth + card API endpoints and hook them into templates.
   - Create minimal Askama templates for OAuth callback and card claiming UI.

4. **Phase 4 — Polish & Readiness**
  - Add request ID + tracing instrumentation covering OAuth, comment verification, issuance.
   - Provide health endpoint and optional metrics stub if ops requires it.
   - Document setup/operation in README, quickstart validation, deployment notes (including key rotation guidance).
   - Run formatting (`cargo fmt`) and linting (`cargo clippy`) as part of final QA.

## Security & Reliability Focus

- **Secrets**: Store encryption keys/YouTube client secrets via env vars for development; document path to a managed secret store for production. Rotate AEAD keys using key versioning policy.
- **Token Handling**: Encrypt access/refresh tokens using `ring::aead`, mask tokens in logs, and ensure decrypt operations are centralized.
- **API Rate Limits**: Implement shared HTTP client with retry/backoff and minimal caching of comment lookups per issuance request.
- **Observability**: Wrap OAuth/callback, comment verifier, card issuance, and error paths with structured `tracing` spans and metrics counters supporting SC-001/SC-004/SC-007.
- **Data Integrity**: Use database constraints (unique indexes) to enforce one active card per member/issuer pair; ensure QR signatures include issuer + card identifiers for future verification.

## Risks & Mitigations

- **YouTube API Quotas**: Mitigate with single-call-per-issuance design and alerting if quotas near limits.
- **Wallet Format Drift**: Coordinate early with 數位皮夾 team; keep QR payload builder encapsulated for rapid updates.
- **Future Expansion**: Design services (OAuth, comment verifier, card issuer) with traits or enums so Twitch/community support can be layered in without heavy refactoring.

## Deliverables

- Running Axum service that issues YouTube membership cards end-to-end.
- Database schema, migrations, and seed data for development use.
- Minimal frontend templates guiding the member issuance flow.
- Documentation covering setup, secrets, and operational checks.

## Post-MVP Follow-ups

- Organizer verification spec (`002-card-verification`).
- Revocation/refresh automation (`003-card-lifecycle-automation`).
- Multi-platform issuer support (`00X-multi-platform`).
