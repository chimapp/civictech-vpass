# Implementation Plan: Channel Membership Verification Card System

**Branch**: `001-channel-membership-verification` | **Date**: 2025-10-12 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/001-channel-membership-verification/spec.md`

**Note**: This template is filled in by the `/speckit.plan` command. See `.specify/templates/commands/plan.md` for the execution workflow.

## Summary

Build a membership card issuance and verification system for YouTube/Twitch channel members. Members authenticate via platform OAuth, system retrieves membership data from platform APIs, generates QR codes compatible with 數位皮夾 (digital wallet), and provides verification interface for event organizers. Background cron job monitors subscription status for automatic card revocation.

## Technical Context

**Language/Version**: Rust (stable, latest)
**Primary Dependencies**:
- Web framework: NEEDS CLARIFICATION (Axum vs Actix-web vs Rocket)
- OAuth client library: NEEDS CLARIFICATION (oauth2 crate ecosystem)
- Database ORM/query builder: NEEDS CLARIFICATION (Diesel vs SQLx vs SeaORM)
- QR code generation: NEEDS CLARIFICATION (qrcode crate vs alternatives)
- HTTP client for platform APIs: NEEDS CLARIFICATION (reqwest vs hyper)

**Storage**: PostgreSQL (stores cards, revocations, OAuth tokens, verification events)
**Testing**: cargo test (unit + integration tests)
**Target Platform**: Linux server (containerized deployment assumed)
**Project Type**: Web application (backend-heavy with frontend for OAuth flows and QR scanning)
**Performance Goals**:
- Card issuance: < 5 seconds end-to-end (per SC-001)
- Verification: < 3 seconds per scan (per SC-002)
- Support 10+ concurrent verifiers (per SC-006)

**Constraints**:
- Must securely store OAuth tokens (FR-030)
- Must handle platform API rate limits (AS-010)
- Requires internet connectivity (AS-008)
- QR code format compatibility with 數位皮夾 (FR-009)

**Scale/Scope**:
- Initial deployment: small-scale (< 1000 cards issued)
- Multiple channel issuers supported
- YouTube + Twitch platform integration
- Future: Discord/Telegram integration (out of scope for v1)

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

**Status**: ✅ PASSED (No constitution file exists yet - no gates to evaluate)

**Note**: Constitution file at `.specify/memory/constitution.md` is a template without specific principles. Once project constitution is established, re-evaluate this plan against defined gates.

## Project Structure

### Documentation (this feature)

```
specs/[###-feature]/
├── plan.md              # This file (/speckit.plan command output)
├── research.md          # Phase 0 output (/speckit.plan command)
├── data-model.md        # Phase 1 output (/speckit.plan command)
├── quickstart.md        # Phase 1 output (/speckit.plan command)
├── contracts/           # Phase 1 output (/speckit.plan command)
└── tasks.md             # Phase 2 output (/speckit.tasks command - NOT created by /speckit.plan)
```

### Source Code (repository root)

```
src/
├── main.rs                    # Application entry point, web server setup
├── models/                    # Database models (Card, Issuer, Member, etc.)
│   ├── mod.rs
│   ├── card.rs
│   ├── issuer.rs
│   ├── member.rs
│   ├── oauth_session.rs
│   ├── verification_event.rs
│   └── revocation.rs
├── services/                  # Business logic services
│   ├── mod.rs
│   ├── oauth/                # OAuth integration (YouTube, Twitch)
│   │   ├── mod.rs
│   │   ├── youtube.rs
│   │   └── twitch.rs
│   ├── card_issuer.rs        # Card issuance logic
│   ├── card_verifier.rs      # Card verification logic
│   ├── qr_generator.rs       # QR code generation
│   └── membership_checker.rs # Platform API membership validation
├── api/                       # HTTP API endpoints
│   ├── mod.rs
│   ├── auth.rs               # OAuth callbacks
│   ├── cards.rs              # Card claiming endpoints
│   ├── verification.rs       # Verification endpoints
│   └── middleware/           # Auth middleware, error handling
│       ├── mod.rs
│       └── session.rs
├── db/                        # Database connection, migrations
│   ├── mod.rs
│   ├── schema.rs             # Generated schema (Diesel) or manual (SQLx)
│   └── migrations/
├── jobs/                      # Cron job logic
│   ├── mod.rs
│   └── subscription_checker.rs  # Periodic subscription status check
├── config.rs                  # Configuration management
├── error.rs                   # Error types
└── lib.rs                     # Library exports for testing

web/                           # Frontend (minimal, mainly OAuth redirects + QR scanner)
├── static/
│   ├── css/
│   └── js/
│       └── qr-scanner.js     # QR code scanning interface
└── templates/                 # HTML templates (if using template engine)
    ├── claim_card.html
    ├── verify_card.html
    └── oauth_callback.html

tests/
├── integration/              # Integration tests (API endpoints, DB)
│   ├── card_issuance_test.rs
│   ├── verification_test.rs
│   └── oauth_flow_test.rs
├── unit/                     # Unit tests (services, models)
│   ├── qr_generator_test.rs
│   └── membership_checker_test.rs
└── fixtures/                 # Test data, mock responses
    └── platform_api_responses.json

migrations/                   # Database migrations (Diesel/SQLx format)
└── [timestamp]_create_initial_schema.sql

Cargo.toml                    # Rust dependencies
.env.example                  # Environment variables template
docker-compose.yml            # PostgreSQL + app container setup
```

**Structure Decision**: Web application with backend-heavy Rust service. Frontend is minimal (static HTML/JS for OAuth redirects and QR scanning UI). Cron job runs as separate process or integrated scheduler within main binary.

## Complexity Tracking

*Fill ONLY if Constitution Check has violations that must be justified*

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| [e.g., 4th project] | [current need] | [why 3 projects insufficient] |
| [e.g., Repository pattern] | [specific problem] | [why direct DB access insufficient] |
