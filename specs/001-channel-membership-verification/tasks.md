# Tasks: Channel Membership Verification Card System (MVP)

**Input**: `specs/001-channel-membership-verification/spec.md`, `plan.md`, research + data-model docs  
**Scope**: YouTube-only member issuance. Organizer verification, background jobs, and multi-platform work are explicitly deferred.

**Tests**: Automated tests may be stubbed; comprehensive test coverage will be planned in a later milestone once additional flows exist.

**Format**: `[ID] [P?] Description`
- `[P]` marks tasks that can proceed in parallel with others in the same phase.
- Reference exact paths to avoid collisions.

---

## Phase 1 — Project Setup

- [ ] T001 [P] Initialize Rust project with `cargo init --name vpass` (if not already present).
- [ ] T002 [P] Create directory structure per plan (`src/{models,services,api,db,web/templates}`, `migrations/`, `tests/`).
- [ ] T003 Configure `Cargo.toml` with Axum, SQLx (postgres, runtime-tokio-rustls, offline feature), oauth2, Reqwest, tower-sessions, tower-sessions-sqlx-store, qrcode, Askama, Serde(+derive), Tracing, tracing-subscriber, Config, Secrecy, Ring, ThisError, Anyhow, uuid, time.
- [ ] T004 [P] Create `.env.example` covering `DATABASE_URL`, `YOUTUBE_CLIENT_ID`, `YOUTUBE_CLIENT_SECRET`, `YOUTUBE_REDIRECT_URI`, `YOUTUBE_VERIFICATION_VIDEO_ID`, `ENCRYPTION_KEY`, `SESSION_SECRET`, `RUST_LOG`.
- [ ] T005 [P] Add `docker-compose.yml` with PostgreSQL service and init instructions.
- [ ] T006 [P] Add `.gitignore` for Rust projects (`target/`, `.env`, `*.sqlite`, etc.).
- [ ] T007 [P] Configure toolchain defaults (`rust-toolchain.toml` or `.cargo/config.toml`) and rustfmt/clippy settings as needed.

## Phase 2 — Foundation

- [ ] T008 [P] Set up SQLx migration harness (`migrations/` folder, `sqlx` CLI metadata) and helper script if needed.
- [ ] T009 Create initial migration (`migrations/2025XXXXXXXX_create_initial_schema.sql`) adding tables: `card_issuers`, `oauth_sessions`, `members`, `membership_cards` with necessary indexes/constraints (unique member+issuer active constraint) and columns for verification video IDs and comment metadata.
- [ ] T010 Implement application configuration loader in `src/config.rs` (env files + defaults, strongly typed struct).
- [ ] T011 Implement error handling in `src/error.rs` using `thiserror` (AuthError, ApiError, DatabaseError, ValidationError).
- [ ] T012 [P] Implement database module in `src/db/mod.rs` (SQLx PgPool creation, health check helper).
- [ ] T013 [P] Implement encryption utilities in `src/services/encryption.rs` using Ring AEAD for encrypt/decrypt of tokens.
- [ ] T014 [P] Implement signature utilities in `src/services/signature.rs` (HMAC-SHA256 sign/verify helpers for QR payloads).
- [ ] T015 [P] Implement session middleware in `src/api/middleware/session.rs` using tower-sessions SQLx store with secure cookie settings.
- [ ] T016 [P] Implement auth middleware in `src/api/middleware/auth.rs` (ensure member role, attach session context).
- [ ] T017 Bootstrap Axum app in `src/main.rs` (router, middleware stack, template engine setup, tracing subscriber, graceful shutdown).
- [ ] T018 [P] Create domain models (`src/models/*.rs`) with SQLx queries: `CardIssuer`, `OAuthSession`, `Member`, `MembershipCard`.

## Phase 3 — User Story 1 (YouTube Card Issuance)

- [ ] T019 Implement YouTube OAuth client in `src/services/oauth/youtube.rs` (state/PKCE generation, code exchange, token refresh).
- [ ] T020 Implement OAuth coordinator in `src/services/oauth/mod.rs` (shared interfaces, encryption integration, session persistence).
- [ ] T021 Implement comment verification service in `src/services/comment_verifier.rs` (YouTube Data API comment lookup, validation of author/video/timestamp, quota/backoff handling).
- [ ] T022 Implement QR generator in `src/services/qr_generator.rs` (payload assembly, signature using HMAC, render as SVG/PNG/data URL via `qrcode`).
- [ ] T023 Implement card issuance service in `src/services/card_issuer.rs` (ensure comment verification is satisfied, prevent duplicates, persist card + member snapshot, return payload + QR).
- [ ] T024 Implement auth routes in `src/api/auth.rs`: 
  - `GET /auth/youtube/login` (state + redirect),
  - `GET /auth/youtube/callback` (token exchange, session persistence),
  - `GET /auth/session` (session info),
  - `POST /auth/logout` (revoke session).
- [ ] T025 Implement card + issuer routes in `src/api/cards.rs` (or `src/api/issuers.rs` if preferred):
  - `POST /cards/claim` (accept members-only comment link and trigger card issuance),
  - `GET /cards/{card_id}` (show card details for owner),
  - `GET /cards/{card_id}/qr` (return QR in requested format),
  - `GET /cards` (list member’s cards, newest first),
  - `GET /issuers` (list active YouTube issuers for selection in UI).
- [ ] T026 [P] Implement Askama templates in `web/templates/`:
  - `oauth_callback.html` (loading + redirect messaging),
  - `claim_card.html` (issuer selection, call-to-action, QR display).
- [ ] T027 Wire templates in `src/main.rs` (serve static assets if needed, route HTML endpoints).
- [ ] T028 Seed development data script (`scripts/seed-dev-data.sql` or Rust helper) to create a sample `card_issuer` entry for local testing.

## Phase 4 — Polish & Operational Readiness

- [ ] T029 [P] Add request ID + tracing context middleware (e.g., `tower-http` request-id) and instrument OAuth, comment verification, and issuance paths.
- [ ] T030 [P] Add health endpoint `GET /health` in `src/api/health.rs` verifying DB connectivity.
- [ ] T031 [P] Document MVP in `README.md` (scope, setup, YouTube credentials, QR flow) and update `docs/quickstart.md` if needed.
- [ ] T032 [P] Document deployment checklist in `docs/DEPLOYMENT.md` (env vars, key rotation steps, monitoring hooks).
- [ ] T033 Run `cargo fmt` + `cargo clippy` and resolve warnings.
- [ ] T034 Validate quickstart instructions on a clean environment; capture fixes to `.env.example`/docs.

---

## Execution Notes

- Complete Phase 2 before any Phase 3 work; Phase 3 assumes schema/models/middleware exist.
- Tasks marked `[P]` touch disjoint files and can be tackled concurrently once prerequisites land.
- Keep commits scoped per task or logical grouping to ease reviews.
- Organizer verification, revocation jobs, and Twitch integration will be introduced in future specs—do not implement them here.
