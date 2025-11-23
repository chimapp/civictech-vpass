# Technical Research: Channel Membership Verification System (MVP)

**Feature**: 001-channel-membership-verification  
**Date**: 2025-10-12  
**Status**: Phase 0 Complete (foundation decisions agreed)

The MVP focuses on issuing YouTube membership cards only. All technology choices below are validated against that scope.

---

## Decision 1 — Web Framework

- **Choice**: Axum (Tokio-based).
- **Why**: Modern async router, excellent middleware story (tower), ergonomic extractors for sessions/config injection, widely adopted, and pairs naturally with SQLx and `oauth2`.
- **Best Practices**:
  - Structure routers modularly (`auth`, `cards`).
  - Use `tower::ServiceBuilder` for request ID, tracing, and session middleware.
  - Layer Askama rendering via Axum response converters.

Alternatives (Actix, Rocket, Warp) were rejected due to heavier abstractions or less ergonomic async story for this scale.

---

## Decision 2 — OAuth & HTTP Client

- **Choice**: `oauth2` crate with Reqwest-backed HTTP client.
- **Scope**: Google OAuth 2.0 (authorization code + PKCE) targeting YouTube read-only scopes for membership/video access.
- **Implementation Notes**:
  - Scopes required: `https://www.googleapis.com/auth/youtube.readonly`.
  - Persist state + PKCE verifier server-side to mitigate CSRF and substitution attacks.
  - Use `oauth2::basic::BasicClient` with custom redirect URI configured via environment.
  - Store access/refresh tokens encrypted with Ring AEAD; wrap plaintext with `secrecy::Secret` while in memory.
  - Refresh tokens on-demand prior to expiration using stored expiry timestamps.

Reqwest remains the HTTP client for Google APIs: share a single `reqwest::Client` with sensible timeouts and exponential backoff for 429/5xx responses.

---

## Decision 3 — Persistence Layer

- **Choice**: SQLx (async, compile-time checked queries).
- **Entities**: `card_issuers`, `members`, `oauth_sessions`, `membership_cards` as defined in `data-model.md`.
- **Best Practices**:
  - Use `sqlx::query!` / `query_as!` macros with offline mode once metadata captured.
  - Wrap issuance operations in transactions: insert/update member, insert card, insert session if applicable.
  - Set `ON DELETE CASCADE` on relationships tied to `members` to avoid orphan sessions/cards when cleaning dev data.
  - Enforce unique `(issuer_id, member_id)` constraint for active cards in the database to backstop service-level checks.

Diesel/SeaORM were rejected to avoid heavier abstractions and because raw SQL suits the limited table set.

---

## Decision 4 — QR Code Generation & Payload Signing

- **Choice**: `qrcode` crate for QR generation, `ring::hmac` for HMAC-SHA256 signatures.
- **Payload Strategy**:
  ```json
  {
    "card_id": "uuid",
    "issuer_id": "uuid",
    "member_youtube_id": "string",
    "channel_name": "string",
    "membership_label": "string",
    "membership_confirmed_at": "ISO8601",
    "membership_verification_token": "string",
    "signature": "hex"
  }
  ```
  - Keep payload compact (< 300 bytes) to guarantee wallet compatibility.
  - Use canonical serialization (sorted keys) before signing to avoid signature drift.
- **Output**: Generate SVG for inline display and optionally PNG/data URL for download.
- **Storage**: Persist payload JSON + signature for future verification/traceability.

---

## Decision 5 — Templates & Frontend

- **Choice**: Askama templates with minimal progressive enhancement (vanilla JS only if needed).
- **Screens**:
  - OAuth callback transition page (loading state, error display).
  - Claim card page (issuer selection, success state with QR + download button).
- **Implementation Tips**:
  - Serve static assets via `tower_http::services::ServeDir` if CSS/JS required.
  - Keep forms POSTing to Axum handlers; rely on server-rendered responses for deterministic UX.

No QR scanning or organizer UI is required in this slice, so frameworks like React or html5-qrcode are not needed yet.

---

## Decision 6 — Observability & Operations

- **Logging/Tracing**: Use `tracing` + `tracing-subscriber` with JSON log option for production. Annotate spans for OAuth callbacks, membership verification API calls, and database transactions.
- **Metrics**: Expose counters (claims attempted, claims succeeded, claims failed by reason) via a simple registry if needed; otherwise rely on logs.
- **Health Check**: Provide `/health` endpoint verifying DB connectivity and application readiness.
- **Secrets Management**: For development, environment variables (.env). For production, integrate with the platform’s secret manager (documented in deployment notes). Rotate AEAD keys with version identifiers stored in config and persisted alongside ciphertext.

---

## Open Points for Future Specs

1. Organizer verification UX and QR scanning library selection.
2. Strategy for automated revocation (cron vs external worker) once additional flows exist.
3. Extending OAuth layer to support Twitch or other platforms while reusing current abstractions.
