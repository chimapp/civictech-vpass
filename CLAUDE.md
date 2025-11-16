# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

Auto-generated from all feature plans. Last updated: 2025-10-12

## Project Overview

VPass - Channel Membership Verification Card System
A Rust-based web application for issuing and verifying digital membership cards for YouTube and Twitch channel members.

## Active Technologies

- **Language**: Rust (stable, latest) (001-channel-membership-verification)
- **Database**: PostgreSQL (stores cards, revocations, OAuth tokens, verification events)
- **Project Type**: Web application (backend-heavy with frontend for OAuth flows and QR scanning)

## Technology Stack

### Web Framework

- **Axum**: Main web framework built on tokio/hyper

### Database

- **SQLx**: Compile-time checked queries with PostgreSQL
- **Migrations**: Use `sqlx migrate` for schema management

### Key Dependencies

- `oauth2`: OAuth 2.0 client for YouTube/Twitch integration
- `reqwest`: HTTP client for platform APIs
- `qrcode`: QR code generation
- `tower-sessions`: Session management
- `tokio-cron-scheduler`: Background job scheduler
- `tracing`: Structured logging

## Project Structure

```
src/
├── main.rs                    # Application entry point
├── models/                    # Database models
│   ├── card.rs
│   ├── issuer.rs
│   ├── member.rs
│   └── ...
├── services/                  # Business logic
│   ├── oauth/                # OAuth integration
│   │   ├── youtube.rs
│   │   └── twitch.rs
│   ├── card_issuer.rs
│   ├── card_verifier.rs
│   └── ...
├── api/                       # HTTP endpoints
│   ├── auth.rs
│   ├── cards.rs
│   └── verification.rs
├── db/                        # Database setup
│   └── migrations/
└── jobs/                      # Background jobs
    └── subscription_checker.rs

web/                           # Frontend (minimal)
├── static/
└── templates/

tests/
├── integration/
└── unit/

migrations/                    # Database migrations
specs/                         # Feature specifications
└── 001-channel-membership-verification/
    ├── spec.md               # Feature requirements
    ├── plan.md               # Implementation plan
    ├── research.md           # Technology decisions
    ├── data-model.md         # Database schema
    ├── quickstart.md         # Setup guide
    └── contracts/            # API contracts
        └── openapi.yaml
```

## Common Commands

### Development

```bash
make dev              # Run development server with auto-reload
make build            # Build project
make test             # Run all tests
make test TEST=name   # Run specific test
make format           # Format code
make check            # Check formatting and run linter
make coverage         # Generate test coverage report
```

### Database

```bash
# Run migrations
sqlx migrate run

# Create new migration
sqlx migrate add migration_name

# Revert last migration
sqlx migrate revert

# Generate SQLx metadata for offline builds
cargo sqlx prepare
```

## Code Style

- Follow Rust standard conventions
- Use `make format` for formatting (enforced by CI)
- Pass `make check` with no warnings
- Prefer explicit error handling over `unwrap()`
- Use `tracing` for logging, not `println!`
- Write unit tests for business logic
- Write integration tests for API endpoints

## Architecture Guidelines

### OAuth Flow

1. User initiates login via `/auth/{platform}/login`
2. Redirect to platform OAuth consent
3. Platform redirects back to `/auth/{platform}/callback`
4. Exchange code for token, create session
5. Store encrypted tokens in database

### Card Issuance

1. Member authenticates via OAuth
2. System retrieves membership data from platform API
3. Validate membership status
4. Generate card with QR code payload
5. Sign payload with HMAC-SHA256

### Card Verification

1. Organizer authenticates as channel owner
2. Scan QR code from attendee's digital wallet
3. Verify signature and check revocation status
4. Log verification event

### Background Jobs

- Cron job runs every 6 hours (configurable)
- Checks subscription status via platform APIs
- Revokes cards for canceled subscriptions
- Handles API rate limits and retries

## Environment Variables

Required in `.env`:

```
DATABASE_URL=postgresql://postgres:password@localhost:5432/vpass_dev
BASE_URL=http://localhost:3000
PORT=3000
YOUTUBE_CLIENT_ID=...
YOUTUBE_CLIENT_SECRET=...
TWITCH_CLIENT_ID=...
TWITCH_CLIENT_SECRET=...
SESSION_SECRET=...
ENCRYPTION_KEY=...
RUST_LOG=info,vpass=debug
```

## Security Considerations

- OAuth tokens encrypted at rest (AES-256-GCM)
- QR codes signed with HMAC to prevent tampering
- Session cookies with `HttpOnly`, `Secure`, `SameSite=Lax`
- Never log sensitive data (tokens, platform IDs)
- Use `secrecy::Secret` wrapper for sensitive values

## Testing Strategy

- **Unit Tests**: Test services in isolation
- **Integration Tests**: Test API endpoints with test database
- **Mocking**: Use `wiremock` for platform API mocks
- **Database Tests**: Use `sqlx::test` attribute for transactional tests

## Documentation References

- Feature Spec: `specs/001-channel-membership-verification/spec.md`
- API Contracts: `specs/001-channel-membership-verification/contracts/openapi.yaml`
- Data Model: `specs/001-channel-membership-verification/data-model.md`
- Setup Guide: `specs/001-channel-membership-verification/quickstart.md`
