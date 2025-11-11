# Implementation Plan: Channel Membership Verification Card System

**Branch**: `001-channel-membership-verification` | **Date**: 2025-11-10 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/001-channel-membership-verification/spec.md`

**Note**: This template is filled in by the `/speckit.plan` command. See `.specify/templates/commands/plan.md` for the execution workflow.

## Summary

VPass enables YouTube channel members to claim digital membership cards by authenticating via OAuth and posting a comment on a members-only video. The system validates membership, generates a Taiwan Digital Wallet (數位皮夾) compatible QR code, and persists the card for 30 days (renewable via background cronjob). Organizers can verify presented credentials via OIDVP protocol at events. The MVP prioritizes member-facing issuance with event verification capabilities implemented ahead of schedule.

## Technical Context

**Language/Version**: Rust 1.75+ (stable, edition 2021)
**Primary Dependencies**: Axum 0.7 (web framework), SQLx 0.7 (PostgreSQL), OAuth2 4.4, tower-sessions 0.12, tokio-cron-scheduler 0.10
**Storage**: PostgreSQL 14+ (membership cards, OAuth tokens encrypted at rest, verification audit logs)
**Testing**: cargo test, wiremock (API mocking), sqlx::test (transactional DB tests)
**Target Platform**: Linux server (containerized deployment via Docker)
**Project Type**: Web application (backend-heavy with minimal frontend templates via Askama)
**Performance Goals**: <5 seconds card issuance end-to-end (YouTube API + wallet API latency), <200ms verification result polling
**Constraints**: YouTube API quota limits (retry with exponential backoff, max 3 attempts/30s), wallet API failure = issuance failure (no partial cards)
**Scale/Scope**: Designed for 1000s of members across multiple channel issuers, 30-day card lifecycle with cronjob renewal, OIDVP verification at physical events

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

**Status**: ✅ No constitution file defined - using project defaults

The project constitution (`.specify/memory/constitution.md`) is still a template. Based on existing codebase inspection:
- ✅ **Simplicity**: Single Rust project, no over-abstraction
- ✅ **Testing**: Integration tests present (wiremock for API mocks), transactional DB tests via sqlx
- ✅ **Observability**: tracing + tracing-subscriber configured for structured logging
- ✅ **Security**: secrecy crate for encrypted OAuth tokens, HMAC signatures for QR payloads

**No violations detected**. Project follows pragmatic Rust web application patterns.

## Project Structure

### Documentation (this feature)

```
specs/001-channel-membership-verification/
├── spec.md              # Feature requirements (updated with clarifications)
├── plan.md              # This file (/speckit.plan command output)
├── research.md          # Phase 0 output (technology decisions)
├── data-model.md        # Phase 1 output (database schema)
├── quickstart.md        # Phase 1 output (setup guide - already exists)
└── contracts/           # Phase 1 output (API contracts)
    └── README.md        # API endpoint documentation
```

### Source Code (repository root)

```
src/
├── models/              # Database entities (SQLx models)
│   ├── card.rs          # MembershipCard (with wallet fields)
│   ├── issuer.rs        # CardIssuer
│   ├── member.rs        # Member
│   ├── oauth_session.rs # OAuthSession
│   ├── event.rs         # Event (verification events)
│   ├── verification_event.rs  # VerificationEvent (audit log)
│   ├── revocation.rs    # Revocation
│   └── mod.rs
├── services/            # Business logic
│   ├── oauth/           # OAuth integration
│   │   └── youtube.rs
│   ├── card_issuer.rs   # Card issuance orchestration
│   ├── card_verifier.rs # Card verification logic
│   ├── wallet_qr.rs     # Taiwan wallet API integration
│   ├── oidvp_verifier.rs # OIDVP verification protocol
│   ├── youtube_channel.rs # YouTube API client
│   └── mod.rs
├── api/                 # HTTP endpoints (Axum routers)
│   ├── auth.rs          # OAuth flow endpoints
│   ├── cards.rs         # Card issuance/management endpoints
│   ├── verification.rs  # Verification endpoints (OIDVP)
│   ├── middleware/      # Session management
│   └── mod.rs
├── jobs/                # Background jobs
│   └── subscription_checker.rs  # Cronjob for membership validation
├── config.rs            # Configuration management
├── db.rs                # Database connection setup
└── main.rs              # Application entry point

migrations/              # SQLx database migrations
├── 20251019222814_add_wallet_qr_data_to_cards.sql
├── 20251026010002_create_verification_events.sql
├── 20251110000001_merge_wallet_qr_to_cards.sql
└── 20251110000002_drop_verification_sessions.sql

templates/               # Askama HTML templates
├── base.html
├── auth/
├── cards/
│   ├── show.html        # Card display page
│   └── my-cards.html
└── verification/
    ├── home.html        # Event list
    ├── scanner.html     # QR scanner interface
    └── history.html     # Verification audit log

static/                  # Frontend assets
├── css/
└── js/
    ├── credential-polling.js  # Wallet credential status polling
    └── card-delete.js

tests/
├── integration/         # API endpoint tests
└── unit/                # Service/model unit tests
```

**Structure Decision**: Single web application project (backend-focused). Frontend is minimal server-rendered HTML via Askama with progressive enhancement JavaScript. No separate frontend build step. This aligns with the "shippable within a single iteration" constraint from the spec.

**Architectural Simplifications (2025-11-10)**:
- Removed `wallet_qr_codes` table (1:1 relationship merged into `membership_cards`)
- Removed `verification_sessions` table (ephemeral state managed in frontend JavaScript)

## Complexity Tracking

*Fill ONLY if Constitution Check has violations that must be justified*

**No violations** - section not applicable.

## Phase 0: Research & Technology Decisions

**Status**: ✅ Complete (documented in existing `research.md`)

All technology choices resolved during initial implementation:
- Rust + Axum for high-performance async web server
- SQLx for compile-time SQL safety with PostgreSQL
- OAuth2 crate for Google/YouTube authentication
- OIDVP protocol for verifiable presentation at events
- Taiwan Digital Wallet API integration
- tower-sessions for encrypted session management
- tokio-cron-scheduler for background membership validation

See [research.md](./research.md) for detailed rationale.

## Phase 1: Design & Contracts

### Database Schema (data-model.md)

**Status**: ✅ Complete (documented in existing `data-model.md`)

Core entities:
- `card_issuers`: YouTube channels authorized to issue cards
- `members`: Cached member profiles from YouTube
- `oauth_sessions`: Encrypted OAuth tokens (AES-256-GCM)
- `membership_cards`: Issued cards with wallet integration fields (expires_at: 30 days default)
- `events`: Verification events for organizers
- `verification_events`: Audit log of successful verifications
- `revocations`: Soft-delete tracking (deleted_at timestamp)

Recent simplifications:
- Wallet fields merged into `membership_cards` (was separate `wallet_qr_codes` table)
- No `verification_sessions` table (frontend manages state)

See [data-model.md](./data-model.md) for complete schema.

### API Contracts (contracts/)

**Status**: ⚠️ Partial - OpenAPI spec was deleted (outdated), needs documentation update

Current endpoints (from codebase inspection):

**Authentication**:
- `GET /auth/youtube/login` - Initiate OAuth flow
- `GET /auth/youtube/callback` - OAuth callback handler
- `POST /auth/logout` - End session

**Card Management**:
- `GET /cards/my-cards` - List user's cards
- `GET /cards/:id` - Show card details
- `GET /cards/:id/qr` - Get wallet QR code
- `GET /cards/:id/poll-credential` - Poll wallet credential status
- `POST /cards/issue` - Issue new card (requires comment URL)
- `DELETE /cards/:id` - Soft-delete card

**Verification (OIDVP)**:
- `GET /verify` - List active events
- `GET /verify/:event_id/scanner` - Scanner page
- `POST /verify/:event_id/request-qr` - Generate verification QR
- `GET /verify/:event_id/check-result/:transaction_id` - Poll verification result
- `GET /verify/:event_id/history` - View verification audit log

See [contracts/README.md](./contracts/README.md) for endpoint documentation.

### Quickstart Guide

**Status**: ✅ Complete (documented in existing `quickstart.md`)

Setup instructions cover:
- Docker Compose PostgreSQL setup
- Environment variable configuration (OAuth credentials, wallet API)
- Database migrations (`sqlx migrate run`)
- Development server (`cargo run`)

See [quickstart.md](./quickstart.md) for complete setup guide.

## Phase 2: Task Decomposition

**Status**: 🚫 Not started - use `/speckit.tasks` command

Task breakdown will be generated in `tasks.md` covering:
- Migration review (ensure expires_at field exists on membership_cards)
- Cronjob implementation (30-day expiration extension logic)
- YouTube API retry logic (exponential backoff per FR-009a)
- Wallet API error handling (fail-fast per FR-008a)
- Card expiration validation (check expires_at during verification)
- Integration test coverage (rate limiting, wallet API failures)
- Performance profiling (5-second issuance target per NFR-001)

---

**Next Steps**:
1. Review this plan for accuracy against current codebase
2. Update `contracts/README.md` to document actual endpoints
3. Run `/speckit.tasks` to generate task breakdown in `tasks.md`
4. Execute implementation tasks in dependency order
