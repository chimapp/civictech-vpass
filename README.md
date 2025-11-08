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

## Secrets Management

Production `.env` files live in `secrets/*.env.enc` and are encrypted
with [SOPS](https://github.com/getsops/sops) using the Cloud KMS key configured
in `.sops.yaml`. The workflow is:

- Create the key ring/key once (see `secrets/README.md` for commands).
- Run `sops secrets/prod.env.enc` to edit secrets in place; the file stays
  encrypted in git.
- Use `scripts/decrypt-env.sh prod` whenever you need the plaintext locally; the
  result (`secrets/.prod.env`) is git-ignored.
- In Cloud Build, grant the build service account `roles/cloudkms.cryptoKeyDecrypter`
  and add a step that decrypts `secrets/prod.env.enc` before the deploy step so
  you can push the values into Secret Manager or `gcloud run deploy`.

Refer to `secrets/README.md` for detailed instructions and sample commands.

## License

TBD
