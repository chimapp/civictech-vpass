# Tasks: Channel Membership Verification Card System

**Input**: Design documents from `/specs/001-channel-membership-verification/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/

**Tests**: Tests are NOT included - implement tests only if explicitly requested later

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`
- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3, US4)
- Include exact file paths in descriptions

## Path Conventions
- **Project root**: `src/`, `tests/`, `migrations/`, `web/`
- Paths follow the structure defined in plan.md

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Project initialization and basic structure

- [ ] T001 [P] Initialize Rust project with `cargo init --name vpass`
- [ ] T002 [P] Create project directory structure per plan.md (src/{models,services,api,db,jobs}, web/{static,templates}, tests/{integration,unit,fixtures}, migrations/)
- [ ] T003 Configure Cargo.toml with dependencies: axum, tokio, sqlx (postgres, runtime-tokio-rustls), oauth2, reqwest, qrcode, tower-sessions, tower-sessions-sqlx-store, tokio-cron-scheduler, serde, serde_json, askama, tracing, tracing-subscriber, config, secrecy, ring or orion (encryption), thiserror, anyhow
- [ ] T004 [P] Create .env.example file with required environment variables (DATABASE_URL, YouTube/Twitch OAuth credentials, session secrets, encryption key, etc.)
- [ ] T005 [P] Create docker-compose.yml for PostgreSQL development database
- [ ] T006 [P] Setup .gitignore for Rust (target/, .env, Cargo.lock for libs)
- [ ] T007 [P] Configure rustfmt and clippy settings in rust-toolchain.toml or .cargo/config.toml

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core infrastructure that MUST be complete before ANY user story can be implemented

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [ ] T008 Create database migration framework setup in migrations/ directory using SQLx
- [ ] T009 Create initial database schema migration (migrations/YYYYMMDDHHMMSS_create_initial_schema.sql) with all 6 tables: card_issuers, oauth_sessions, membership_cards, revocations, verification_events, subscription_check_jobs (per data-model.md)
- [ ] T010 [P] Implement error types in src/error.rs using thiserror: OAuthError, PlatformApiError, DatabaseError, ValidationError, NotFoundError
- [ ] T011 [P] Implement configuration management in src/config.rs using config crate: load from .env, define Config struct with all required fields
- [ ] T012 [P] Implement database connection pooling in src/db/mod.rs: create PgPool using sqlx, implement connection health check
- [ ] T013 [P] Implement encryption utilities in src/services/encryption.rs: AES-256-GCM encrypt/decrypt functions for OAuth tokens using ring or orion
- [ ] T014 [P] Implement HMAC signature utilities in src/services/signature.rs: HMAC-SHA256 sign/verify for QR code payloads
- [ ] T015 Implement base Axum application in src/main.rs: setup router, middleware stack, database pool injection, tracing subscriber, graceful shutdown
- [ ] T016 [P] Implement session middleware in src/api/middleware/session.rs using tower-sessions: PostgreSQL backend, secure cookie configuration
- [ ] T017 [P] Implement authentication middleware in src/api/middleware/auth.rs: extract session, verify user role (member/organizer), inject user context
- [ ] T018 [P] Create base model structs in src/models/mod.rs: define common traits, UUID handling, timestamp utilities

**Checkpoint**: Foundation ready - user story implementation can now begin in parallel

---

## Phase 3: User Story 1 - Channel Member Claims Membership Card (Priority: P1) 🎯 MVP

**Goal**: Enable channel members to authenticate via OAuth, verify their membership status through platform APIs, and receive a QR code for their digital membership card

**Independent Test**: Have a test user with real YouTube/Twitch membership claim a card and successfully import QR code into 數位皮夾

### Database Models for User Story 1

- [ ] T019 [P] [US1] Implement CardIssuer model in src/models/issuer.rs: struct with id, issuer_type, platform, platform_channel_id, channel_name, is_verified, created_at, updated_at; SQL queries for CRUD operations
- [ ] T020 [P] [US1] Implement OAuthSession model in src/models/oauth_session.rs: struct with id, user_role, platform, platform_user_id, access_token (encrypted), refresh_token (encrypted), token_expires_at, scope, issuer_id, created_at, last_used_at; SQL queries for session management
- [ ] T021 [P] [US1] Implement MembershipCard model in src/models/card.rs: struct with id, issuer_id, platform, member_platform_id, member_display_name, membership_level, subscription_start_date, subscription_duration_months, is_active_member, supporter_metrics (JSONB), supplementary_data (JSONB), qr_code_payload, qr_code_signature, is_revoked, needs_refresh, issued_at, revoked_at; SQL queries for CRUD and finding active cards

### OAuth Integration for User Story 1

- [ ] T022 [P] [US1] Implement YouTube OAuth client in src/services/oauth/youtube.rs: build authorization URL with PKCE, exchange code for token, refresh token logic, API client setup with OAuth token
- [ ] T023 [P] [US1] Implement Twitch OAuth client in src/services/oauth/twitch.rs: build authorization URL, exchange code for token, refresh token logic, API client setup with OAuth token
- [ ] T024 [US1] Implement OAuth service coordinator in src/services/oauth/mod.rs: platform-agnostic OAuth interface, platform selection logic, token encryption/decryption integration, session creation

### Platform API Integration for User Story 1

- [ ] T025 [P] [US1] Implement YouTube membership checker in src/services/membership_checker.rs (YouTube module): call YouTube Data API v3 to retrieve member data for authenticated user, parse membership level, subscription start date, engagement metrics, handle API errors and rate limits
- [ ] T026 [P] [US1] Implement Twitch subscription checker in src/services/membership_checker.rs (Twitch module): call Twitch API to retrieve subscription data, parse subscription tier, duration, handle API errors and rate limits
- [ ] T027 [US1] Implement membership validation service in src/services/membership_checker.rs: validate membership is active, extract required card data from platform response, handle edge cases (expired, never subscribed, platform unavailable)

### QR Code Generation for User Story 1

- [ ] T028 [US1] Implement QR code generator in src/services/qr_generator.rs: generate QR code payload JSON (card_id, issuer_id, member_platform_id, platform, membership_level, issued_at), sign payload with HMAC-SHA256, generate QR code image (SVG/PNG/data URL) using qrcode crate, handle different output formats

### Card Issuance Service for User Story 1

- [ ] T029 [US1] Implement card issuance service in src/services/card_issuer.rs: verify user session is member role, retrieve membership data from platform API via membership_checker, validate no duplicate active card exists, create membership card record in database, generate QR code, return card with QR code URL

### API Endpoints for User Story 1

- [ ] T030 [US1] Implement OAuth login endpoint in src/api/auth.rs (GET /auth/{platform}/login): validate platform and role parameters, generate OAuth state with CSRF token, store state in session, redirect to platform OAuth URL
- [ ] T031 [US1] Implement OAuth callback endpoint in src/api/auth.rs (GET /auth/{platform}/callback): validate state parameter, exchange authorization code for tokens, encrypt and store tokens in oauth_sessions table, create session cookie, redirect to appropriate page (claim card or verify)
- [ ] T032 [P] [US1] Implement session info endpoint in src/api/auth.rs (GET /auth/session): extract session from middleware, return user_role, platform, platform_user_id, issuer_id (if organizer), authenticated_at
- [ ] T033 [P] [US1] Implement logout endpoint in src/api/auth.rs (POST /auth/logout): delete session from database and cookie, return success message
- [ ] T034 [US1] Implement card claiming endpoint in src/api/cards.rs (POST /cards/claim): extract member session, validate issuer_id exists, call card_issuer service, return card with QR code URL, handle errors (not subscribed, duplicate card, platform API unavailable)
- [ ] T035 [P] [US1] Implement get card details endpoint in src/api/cards.rs (GET /cards/{card_id}): query card from database, verify user owns card (member session), return card details
- [ ] T036 [P] [US1] Implement list my cards endpoint in src/api/cards.rs (GET /cards/my-cards): extract member session, query all cards for platform_user_id, optionally include revoked cards, return list
- [ ] T037 [P] [US1] Implement get QR code endpoint in src/api/cards.rs (GET /cards/{card_id}/qr): query card from database, verify user owns card, generate QR code in requested format (svg/png/data_url) using qr_generator, return image or JSON with data URL
- [ ] T038 [P] [US1] Implement list issuers endpoint in src/api/cards.rs (GET /issuers): query card_issuers table, optionally filter by platform and verified status, return list of issuers

### Frontend for User Story 1

- [ ] T039 [P] [US1] Create OAuth callback HTML template in web/templates/oauth_callback.html: loading spinner, success/error messages, auto-redirect logic
- [ ] T040 [P] [US1] Create claim card HTML template in web/templates/claim_card.html: issuer selection dropdown, form to submit supplementary data, display QR code after successful claim, download QR code button
- [ ] T041 [US1] Implement frontend route handlers in src/main.rs: serve static files from web/static/, render templates using askama, integrate with API endpoints

**Checkpoint**: At this point, User Story 1 should be fully functional and testable independently. Members can authenticate, claim cards, and get QR codes.

---

## Phase 4: User Story 2 - Event Organizer Verifies Membership Card (Priority: P2)

**Goal**: Enable event organizers to authenticate as channel owners and verify membership cards scanned from attendees' digital wallets

**Independent Test**: Have a test organizer authenticate, scan test card QR codes, and verify that valid cards are accepted while revoked cards are rejected

### Database Models for User Story 2

- [ ] T042 [US2] Implement VerificationEvent model in src/models/verification_event.rs: struct with id, card_id, verifier_issuer_id, verification_result, verification_context (JSONB), verified_at; SQL queries for creating events and querying history

### Card Verification Service for User Story 2

- [ ] T043 [US2] Implement card verifier service in src/services/card_verifier.rs: parse QR code payload JSON, verify HMAC-SHA256 signature, query card from database by card_id, check card is not revoked (is_revoked = false), verify card belongs to organizer's channel (issuer_id matches), return verification result (success/revoked/invalid_signature/wrong_issuer/not_found)

### API Endpoints for User Story 2

- [ ] T044 [US2] Implement verify scan endpoint in src/api/verification.rs (POST /verify/scan): extract organizer session, parse qr_payload from request body, call card_verifier service, log verification event to verification_events table, return verification result with card details if success
- [ ] T045 [P] [US2] Implement verification history endpoint in src/api/verification.rs (GET /verify/history): extract organizer session, query verification_events for verifier_issuer_id, support pagination (limit/offset), return list of verification events with card member names

### Frontend for User Story 2

- [ ] T046 [P] [US2] Create QR scanner JavaScript in web/static/js/qr-scanner.js: use html5-qrcode library, initialize camera, scan QR code, extract payload, send to /verify/scan endpoint, display result
- [ ] T047 [US2] Create verify card HTML template in web/templates/verify_card.html: QR scanner interface, display verification result (success with card details or failure with reason), button to scan next card, link to verification history

### Integration for User Story 2

- [ ] T048 [US2] Update OAuth callback logic in src/api/auth.rs: if user_role is organizer, verify channel ownership via platform API, create or find card_issuer record, link oauth_session to issuer_id, redirect to verification page
- [ ] T049 [US2] Add organizer-specific session validation in src/api/middleware/auth.rs: verify issuer_id is set for organizer sessions, load issuer data into context

**Checkpoint**: At this point, User Stories 1 AND 2 should both work independently. Members can claim cards, organizers can verify them.

---

## Phase 5: User Story 3 - Card Auto-Revocation on Subscription Cancellation (Priority: P3)

**Goal**: Automatically detect subscription cancellations via periodic platform API checks and revoke corresponding membership cards

**Independent Test**: Cancel a test subscription, wait for cron job to run, verify that the card fails verification

### Database Models for User Story 3

- [ ] T050 [P] [US3] Implement Revocation model in src/models/revocation.rs: struct with id, card_id, reason, reason_detail, new_card_id, revoked_by, revoked_at; SQL queries for creating revocation records and querying history
- [ ] T051 [P] [US3] Implement SubscriptionCheckJob model in src/models/subscription_check_job.rs: struct with id, job_started_at, job_completed_at, cards_checked, cards_revoked, errors_count, error_details (JSONB), job_status; SQL queries for creating and updating job records

### Subscription Checking Service for User Story 3

- [ ] T052 [US3] Implement subscription checker job in src/jobs/subscription_checker.rs: query all active cards (is_revoked = false), batch process cards (handle rate limits), for each card: retrieve OAuth session for member, refresh token if expired, call platform API to check subscription status, if subscription canceled or expired: mark card as revoked (is_revoked = true, revoked_at = NOW), create revocation record (reason = subscription_canceled, revoked_by = system), log job run to subscription_check_jobs table with statistics

### Cron Job Integration for User Story 3

- [ ] T053 [US3] Implement cron scheduler in src/main.rs using tokio-cron-scheduler: load cron schedule from config (default "0 */6 * * *" = every 6 hours), register subscription_checker job, implement graceful shutdown to avoid interrupting job mid-run, add logging for job start/completion

### Integration for User Story 3

- [ ] T054 [US3] Update card_verifier service in src/services/card_verifier.rs: check is_revoked flag, if revoked return verification_result = "revoked" with revoked_at timestamp
- [ ] T055 [US3] Add revocation check to verification endpoint in src/api/verification.rs: include revocation reason in error response if card is revoked

**Checkpoint**: All user stories (1, 2, 3) should now work together. Cards are automatically revoked when subscriptions are canceled.

---

## Phase 6: User Story 4 - Card Refresh on Membership Changes (Priority: P3)

**Goal**: Detect membership changes (level upgrades, badge updates) and allow members to re-authenticate to claim refreshed cards with updated information

**Independent Test**: Change membership level for test user, have subscription checker detect change, verify member can re-authenticate and claim new card with updated data

### Card Refresh Logic for User Story 4

- [ ] T056 [US4] Update subscription checker job in src/jobs/subscription_checker.rs: for each card, compare current platform API membership data with stored card data (membership_level, subscription_duration_months), if changed: mark old card with needs_refresh = true, create revocation record (reason = membership_changed, revoked_by = system), log change details
- [ ] T057 [US4] Update card issuance service in src/services/card_issuer.rs: before creating new card, check if member has card marked needs_refresh = true for same issuer, if yes: revoke old card (is_revoked = true, revoked_at = NOW), link revocation to new card (new_card_id), issue new card with updated membership data
- [ ] T058 [US4] Update card_verifier service in src/services/card_verifier.rs: if card has needs_refresh = true (even if not revoked), return verification result indicating card is outdated and member should refresh via VPass

### Frontend Updates for User Story 4

- [ ] T059 [US4] Update claim card template in web/templates/claim_card.html: show notification if member has card marked needs_refresh for selected issuer, display message explaining membership changed and new card will be issued
- [ ] T060 [US4] Update verify card template in web/templates/verify_card.html: if verification returns "outdated" result, display message prompting member to refresh card via VPass

**Checkpoint**: All user stories (1, 2, 3, 4) should now be independently functional. Cards stay current with membership changes.

---

## Phase 7: Polish & Cross-Cutting Concerns

**Purpose**: Improvements that affect multiple user stories

- [ ] T061 [P] Add comprehensive logging with tracing spans in all services: OAuth flows, card issuance, verification, cron jobs
- [ ] T062 [P] Implement structured error responses across all API endpoints: consistent ErrorResponse format, appropriate HTTP status codes, detailed error context
- [ ] T063 [P] Add request ID middleware in src/api/middleware/request_id.rs: generate unique request ID per request, include in all logs, return in response headers
- [ ] T064 [P] Implement health check endpoint in src/api/health.rs (GET /health): check database connection, return service status
- [ ] T065 [P] Add metrics endpoint in src/api/metrics.rs (GET /metrics): expose Prometheus-compatible metrics (requests, latencies, errors)
- [ ] T066 [P] Create README.md: project overview, setup instructions reference to quickstart.md, architecture diagram, API documentation link
- [ ] T067 [P] Create CONTRIBUTING.md: development workflow, code style guidelines, testing requirements, PR process
- [ ] T068 [P] Add database migration validation script in scripts/validate-migrations.sh: check all migrations can be applied and reverted cleanly
- [ ] T069 [P] Create seed data script in scripts/seed-dev-data.sql: insert test card_issuers for development, example OAuth sessions (with fake tokens)
- [ ] T070 Add rate limiting middleware in src/api/middleware/rate_limit.rs: per-IP rate limits, per-user rate limits for authenticated endpoints
- [ ] T071 Add CORS middleware configuration in src/main.rs: configure allowed origins, methods, headers for production deployment
- [ ] T072 [P] Add security headers middleware in src/api/middleware/security.rs: set X-Frame-Options, X-Content-Type-Options, Strict-Transport-Security, Content-Security-Policy
- [ ] T073 Run cargo clippy and fix all warnings: ensure code passes linter with zero warnings
- [ ] T074 Run cargo fmt and verify formatting: ensure consistent code style
- [ ] T075 [P] Add Docker multi-stage build Dockerfile: build stage with Rust compiler, runtime stage with minimal image, copy binary and assets
- [ ] T076 Update docker-compose.yml: add app service, configure networking, add volume mounts for development
- [ ] T077 [P] Create deployment documentation in docs/DEPLOYMENT.md: environment variables, database migration steps, SSL certificate setup, monitoring setup
- [ ] T078 Validate quickstart.md: follow all setup steps on clean environment, verify all commands work, update any outdated instructions

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion - BLOCKS all user stories
- **User Story 1 (Phase 3)**: Depends on Foundational (Phase 2)  - No dependencies on other stories
- **User Story 2 (Phase 4)**: Depends on Foundational (Phase 2) - May integrate with US1 but independently testable
- **User Story 3 (Phase 5)**: Depends on Foundational (Phase 2) and User Story 1 (Phase 3) - Needs card issuance to exist before revocation
- **User Story 4 (Phase 6)**: Depends on Foundational (Phase 2) and User Story 1 (Phase 3) and User Story 3 (Phase 5) - Builds on revocation logic
- **Polish (Phase 7)**: Depends on desired user stories being complete

### User Story Dependencies

```
Foundational (Phase 2)
    ↓
    ├─→ User Story 1 (Phase 3) - MVP ✅ Independent
    │       ↓
    │       ├─→ User Story 3 (Phase 5) ✅ Revocation needs cards to exist
    │       │       ↓
    │       │       └─→ User Story 4 (Phase 6) ✅ Refresh builds on revocation
    │       │
    │       └─→ User Story 2 (Phase 4) ✅ Independent (can start in parallel with US3)
    │
    └─→ Polish (Phase 7)
```

### Within Each User Story

- Models before services (models define data structures services use)
- OAuth integration before platform API integration
- Services before API endpoints (endpoints call services)
- API endpoints before frontend templates
- Integration tasks last within story

### Parallel Opportunities

**Phase 1 (Setup)**: T001-T007 all marked [P] can run in parallel

**Phase 2 (Foundational)**: T010-T018 marked [P] can run in parallel (T009 must complete first for schema)

**Phase 3 (User Story 1)**:
- Models (T019-T021) can run in parallel
- OAuth clients (T022-T023) can run in parallel after T024 defines interface
- Platform checkers (T025-T026) can run in parallel
- Endpoints (T032-T033, T035-T038) marked [P] can run in parallel after services complete
- Frontend templates (T039-T040) can run in parallel

**Phase 4 (User Story 2)**: T045-T047 marked [P] can run in parallel after T044 completes

**Phase 5 (User Story 3)**: T050-T051 can run in parallel

**Phase 6 (User Story 4)**: T059-T060 can run in parallel

**Phase 7 (Polish)**: Most tasks marked [P] (T061-T078) can run in parallel

**Cross-Phase**: Once Foundational completes, User Story 1 and User Story 2 can start in parallel (different team members)

---

## Parallel Example: User Story 1

```bash
# After Foundational phase completes, launch models in parallel:
Task T019: Implement CardIssuer model in src/models/issuer.rs
Task T020: Implement OAuthSession model in src/models/oauth_session.rs
Task T021: Implement MembershipCard model in src/models/card.rs

# After models complete, launch OAuth clients in parallel:
Task T022: Implement YouTube OAuth client in src/services/oauth/youtube.rs
Task T023: Implement Twitch OAuth client in src/services/oauth/twitch.rs

# After OAuth coordinator (T024) completes, launch platform checkers in parallel:
Task T025: Implement YouTube membership checker in src/services/membership_checker.rs
Task T026: Implement Twitch subscription checker in src/services/membership_checker.rs

# After all services complete, launch independent API endpoints in parallel:
Task T032: Implement session info endpoint in src/api/auth.rs (GET /auth/session)
Task T033: Implement logout endpoint in src/api/auth.rs (POST /auth/logout)
Task T035: Implement get card details endpoint in src/api/cards.rs (GET /cards/{card_id})
Task T036: Implement list my cards endpoint in src/api/cards.rs (GET /cards/my-cards)
Task T037: Implement get QR code endpoint in src/api/cards.rs (GET /cards/{card_id}/qr)
Task T038: Implement list issuers endpoint in src/api/cards.rs (GET /issuers)

# Frontend templates can run in parallel:
Task T039: Create OAuth callback HTML template in web/templates/oauth_callback.html
Task T040: Create claim card HTML template in web/templates/claim_card.html
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup (T001-T007)
2. Complete Phase 2: Foundational (T008-T018) - CRITICAL blocking phase
3. Complete Phase 3: User Story 1 (T019-T041)
4. **STOP and VALIDATE**: Test User Story 1 independently
   - Authenticate as member via OAuth
   - Claim card for test issuer
   - Download QR code
   - Verify QR code can be imported to 數位皮夾 (coordinate with wallet team)
5. Deploy/demo MVP if ready

### Incremental Delivery

1. Complete Setup + Foundational → **Foundation ready**
2. Add User Story 1 → Test independently → **Deploy/Demo (MVP!)** ✅
3. Add User Story 2 → Test independently → **Deploy/Demo** (now members can claim AND organizers can verify)
4. Add User Story 3 → Test independently → **Deploy/Demo** (now automatic revocation works)
5. Add User Story 4 → Test independently → **Deploy/Demo** (now cards stay current)
6. Add Polish tasks → Final release

Each story adds value without breaking previous stories.

### Parallel Team Strategy

With multiple developers:

1. **Team completes Setup + Foundational together** (T001-T018)
2. **Once Foundational is done**:
   - **Developer A**: User Story 1 (T019-T041)
   - **Developer B**: User Story 2 (T042-T049) - can start after US1 models/services partially complete
   - Wait for US1 to finish before starting US3
3. **After US1 completes**:
   - **Developer A**: User Story 3 (T050-T055)
   - **Developer C**: Polish tasks in parallel (T061-T078)
4. **After US3 completes**:
   - **Developer A or B**: User Story 4 (T056-T060)

---

## Notes

- **[P] tasks** = different files, no dependencies, can run in parallel
- **[Story] label** maps task to specific user story for traceability
- Each user story should be independently completable and testable
- Commit after each task or logical group
- Stop at any checkpoint to validate story independently
- **Foundational phase is critical** - all user stories block on its completion
- **User Story 1 is MVP** - deliver this first before adding other stories
- Avoid: vague tasks, same file conflicts, cross-story dependencies that break independence
- **Tests not included** - add test tasks later if TDD approach is requested

---

## Task Count Summary

- **Phase 1 (Setup)**: 7 tasks
- **Phase 2 (Foundational)**: 11 tasks (BLOCKING)
- **Phase 3 (User Story 1 - MVP)**: 23 tasks
- **Phase 4 (User Story 2)**: 8 tasks
- **Phase 5 (User Story 3)**: 6 tasks
- **Phase 6 (User Story 4)**: 5 tasks
- **Phase 7 (Polish)**: 18 tasks

**Total**: 78 tasks

**Parallel opportunities identified**: 47 tasks marked [P] can run in parallel with other tasks in their phase

**Independent test criteria defined for each user story**:
- US1: Member can claim card and get QR code
- US2: Organizer can verify scanned cards
- US3: Canceled subscriptions result in revoked cards
- US4: Membership changes trigger card refresh

**Suggested MVP scope**: Phase 1 + Phase 2 + Phase 3 (User Story 1 only) = 41 tasks
