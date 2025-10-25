# VPass - Channel Membership Verification Card System

A Rust-based web application for issuing and verifying digital membership cards for YouTube and Twitch channel members.

## Project Status

🚧 **Project Scaffolding Complete** - Ready for implementation

## Quick Start

For detailed setup instructions, see [quickstart.md](specs/001-channel-membership-verification/quickstart.md)

### Prerequisites

- Rust (stable, latest)
- PostgreSQL 14+
- Docker (optional, recommended) OR devenv (alternative)

### Quick Setup

**Option 1: Using Docker**

```bash
# 1. Copy environment template
cp .env.example .env
# Edit .env with your OAuth credentials

# 2. Start PostgreSQL
docker-compose up -d postgres

# 3. Run database migrations
make migrate

# 4. Run the server
cargo run
```

**Option 2: Using devenv**

```bash
# 1. Install devenv if not already installed
# See: https://devenv.sh/getting-started/

# 2. Start PostgreSQL
devenv up

# 3. Copy environment template
cp .env.example .env
# Edit .env with your OAuth credentials

# 4. Run database migrations
make migrate

# 5. Run the server
cargo run
```

## Project Structure

See [CLAUDE.md](CLAUDE.md) for complete project structure and development guidelines.

## Documentation

- **Feature Spec**: [specs/001-channel-membership-verification/spec.md](specs/001-channel-membership-verification/spec.md)
- **API Contracts**: [specs/001-channel-membership-verification/contracts/openapi.yaml](specs/001-channel-membership-verification/contracts/openapi.yaml)
- **Data Model**: [specs/001-channel-membership-verification/data-model.md](specs/001-channel-membership-verification/data-model.md)
- **Implementation Tasks**: [specs/001-channel-membership-verification/tasks.md](specs/001-channel-membership-verification/tasks.md)

## Architecture

- **Language**: Rust
- **Web Framework**: Axum
- **Database**: PostgreSQL with SQLx
- **OAuth**: YouTube Data API v3, Twitch API
- **QR Codes**: qrcode crate

## Development

See [CLAUDE.md](CLAUDE.md#common-commands) for all available development commands using Make or cargo.

## License

TBD
