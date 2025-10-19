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

## Design System

### Visual Design Principles

VPass follows a minimalist, hardware-inspired design aesthetic with these core principles:

1. **Simplicity First**: Clean, uncluttered interfaces with generous whitespace
2. **Functional Beauty**: Every design element serves a purpose
3. **Tactile Feel**: Subtle shadows and depth suggest physical interaction
4. **Systematic Layout**: Consistent 2px gaps between elements for tight, organized grids

### Color Palette

The color scheme is inspired by precision hardware design with warm, sophisticated tones:

**Primary Colors:**
- **Navy Blue** (`#1E3A5F`) - Main headings, primary actions, authority
- **Ochre/Tan** (`#B8915F`) - Secondary actions, status indicators, warmth
- **Cool Gray** (`#8C8C88`) - Tertiary accents, subtle emphasis
- **Bright Orange** (`#FF5722`) - Critical actions, alerts, energy

**Neutrals:**
- **Warm Gray** (`#E8E6E0`) - Background, creates calm canvas
- **Off-White** (`#F5F3ED`) - Card surfaces, panels, buttons
- **Black** (`#000000`) - Text, high contrast elements
- **Mid Gray** (`#666666`) - Secondary text, labels

### Typography

- **Font Family**: Helvetica Neue, Arial, sans-serif
- **Heading Weight**: 300 (light) for large headlines
- **Body Weight**: 400 (regular) for readability
- **Labels**: Uppercase, 1-2px letter-spacing for technical feel
- **Data/IDs**: Courier New monospace for technical precision

### Component Patterns

**Buttons:**
- Off-white background with subtle shadows at rest
- Navy or ochre background on hover
- Consistent padding: 16-24px
- Uppercase labels with letter-spacing

**Cards/Panels:**
- Off-white background on warm gray
- Subtle box-shadow: `0 2px 4px rgba(0,0,0,0.08)`
- Border accents in brand colors (2-4px)
- 2px gaps in grid layouts

**Forms:**
- White input backgrounds with gray borders
- Navy blue focus states
- Clear, uppercase labels
- Monospace font for technical inputs (UUIDs, URLs)

**Status Indicators:**
- Solid color backgrounds with black text
- Uppercase, bold, high letter-spacing
- Ochre for active/connected states
- Orange for critical actions

### Layout Guidelines

- **Max Width**: 600-1000px depending on content density
- **Spacing**: Use multiples of 8px (8, 16, 24, 32, 40, 48, 60, 80)
- **Grid Gap**: 2px for tight, systematic layouts
- **Mobile**: Single column layouts, maintain spacing rhythm

### Interaction Design

- **Transitions**: 0.2s for color/background changes
- **Hover States**: Always provide visual feedback
- **Focus States**: Clear, accessible focus indicators
- **Shadows**: Elevate on hover for button-like elements

## Documentation References

- Feature Spec: `specs/001-channel-membership-verification/spec.md`
- API Contracts: `specs/001-channel-membership-verification/contracts/openapi.yaml`
- Data Model: `specs/001-channel-membership-verification/data-model.md`
- Setup Guide: `specs/001-channel-membership-verification/quickstart.md`
