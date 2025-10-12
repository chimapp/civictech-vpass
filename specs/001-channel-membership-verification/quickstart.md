# Quick Start Guide: VPass Development

**Feature**: Channel Membership Verification Card System
**Last Updated**: 2025-10-12

This guide helps you set up the VPass development environment and run the application locally.

---

## Prerequisites

### Required Software

- **Rust** (stable, latest): Install via [rustup](https://rustup.rs/)
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  ```

- **PostgreSQL** (version 14+):
  - macOS: `brew install postgresql@14`
  - Ubuntu: `apt-get install postgresql-14`
  - Or use Docker (see below)

- **Docker** (optional, recommended): For running PostgreSQL in a container
  - Install from [docker.com](https://www.docker.com/get-started)

- **SQLx CLI**: For database migrations
  ```bash
  cargo install sqlx-cli --no-default-features --features postgres
  ```

### Platform API Credentials

You'll need OAuth credentials from:

1. **YouTube Data API**:
   - Go to [Google Cloud Console](https://console.cloud.google.com/)
   - Create project → Enable YouTube Data API v3
   - Create OAuth 2.0 credentials (Web application)
   - Add redirect URI: `http://localhost:3000/auth/youtube/callback`

2. **Twitch API**:
   - Go to [Twitch Dev Console](https://dev.twitch.tv/console)
   - Register application
   - Add OAuth redirect URL: `http://localhost:3000/auth/twitch/callback`

---

## Setup Steps

### 1. Clone Repository

```bash
git clone <repository-url>
cd civictech-vpass
git checkout 001-channel-membership-verification
```

### 2. Database Setup

#### Option A: Docker (Recommended)

```bash
# Start PostgreSQL container
docker-compose up -d postgres

# Wait for PostgreSQL to be ready
docker-compose logs -f postgres  # Ctrl+C when you see "database system is ready"
```

#### Option B: Local PostgreSQL

```bash
# Create database
createdb vpass_dev

# (Optional) Create test database
createdb vpass_test
```

### 3. Environment Configuration

Create `.env` file in project root:

```bash
cp .env.example .env
```

Edit `.env` with your values:

```env
# Database
DATABASE_URL=postgresql://postgres:password@localhost:5432/vpass_dev
TEST_DATABASE_URL=postgresql://postgres:password@localhost:5432/vpass_test

# Server
BASE_URL=http://localhost:3000
PORT=3000

# YouTube OAuth
YOUTUBE_CLIENT_ID=your_youtube_client_id.apps.googleusercontent.com
YOUTUBE_CLIENT_SECRET=your_youtube_client_secret

# Twitch OAuth
TWITCH_CLIENT_ID=your_twitch_client_id
TWITCH_CLIENT_SECRET=your_twitch_client_secret

# Session Security
SESSION_SECRET=generate_random_string_here_at_least_32_chars
ENCRYPTION_KEY=generate_random_key_32_bytes_hex_encoded

# Cron Schedule (optional, default: every 6 hours)
SUBSCRIPTION_CHECK_SCHEDULE="0 */6 * * *"

# Logging
RUST_LOG=info,vpass=debug
```

**Generate secure secrets**:

```bash
# Generate SESSION_SECRET
openssl rand -hex 32

# Generate ENCRYPTION_KEY
openssl rand -hex 32
```

### 4. Run Database Migrations

```bash
# Run migrations
sqlx migrate run

# Verify migration status
sqlx migrate info
```

If migrations don't exist yet, you'll need to create them based on `data-model.md`.

### 5. Build and Run

```bash
# Build the project
cargo build

# Run the development server
cargo run

# Or use cargo-watch for auto-reload on file changes
cargo install cargo-watch
cargo watch -x run
```

The server will start on http://localhost:3000

---

## Verify Installation

### 1. Health Check

```bash
# Check if server is running
curl http://localhost:3000/

# Expected: HTML page or JSON response indicating service is up
```

### 2. Check Database Connection

```bash
# Attempt to list issuers (should return empty array initially)
curl http://localhost:3000/issuers

# Expected: {"issuers": []}
```

### 3. Test OAuth Flow

Open browser to:
- http://localhost:3000/auth/youtube/login?role=member
- Should redirect to Google OAuth consent screen

---

## Development Workflow

### Running Tests

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_card_issuance

# Run tests with output
cargo test -- --nocapture

# Run integration tests only
cargo test --test integration

# Run with test coverage (requires cargo-tarpaulin)
cargo install cargo-tarpaulin
cargo tarpaulin --out Html
```

### Database Migrations

#### Create a New Migration

```bash
sqlx migrate add create_users_table
# Edit the generated file in migrations/
```

#### Rollback Migration

```bash
sqlx migrate revert
```

#### Reset Database

```bash
sqlx database drop
sqlx database create
sqlx migrate run
```

### Code Formatting and Linting

```bash
# Format code
cargo fmt

# Check formatting
cargo fmt -- --check

# Run linter
cargo clippy

# Fix clippy warnings
cargo clippy --fix
```

### Viewing Logs

```bash
# Run with detailed logs
RUST_LOG=debug cargo run

# Filter specific module
RUST_LOG=vpass::services::oauth=trace cargo run
```

---

## Common Development Tasks

### Seed Test Data

Create a script `scripts/seed_dev_data.sh`:

```bash
#!/bin/bash

# Insert test issuer
psql $DATABASE_URL <<EOF
INSERT INTO card_issuers (id, issuer_type, platform, platform_channel_id, channel_name, is_verified)
VALUES (
    gen_random_uuid(),
    'official_channel',
    'youtube',
    'UCxxxxx_test_channel',
    'Test Gaming Channel',
    TRUE
);
EOF

echo "Test data seeded successfully"
```

Run it:

```bash
chmod +x scripts/seed_dev_data.sh
./scripts/seed_dev_data.sh
```

### View API Documentation

```bash
# Install swagger-ui-watcher
npm install -g swagger-ui-watcher

# View OpenAPI spec
cd specs/001-channel-membership-verification/contracts
swagger-ui-watcher openapi.yaml
```

Open http://localhost:8000

### Debugging

#### With VS Code

Create `.vscode/launch.json`:

```json
{
  "version": "0.2.0",
  "configurations": [
    {
      "type": "lldb",
      "request": "launch",
      "name": "Debug VPass",
      "cargo": {
        "args": ["build", "--bin=vpass"],
        "filter": {
          "name": "vpass",
          "kind": "bin"
        }
      },
      "args": [],
      "cwd": "${workspaceFolder}",
      "env": {
        "RUST_LOG": "debug"
      }
    }
  ]
}
```

Install "CodeLLDB" extension, then press F5 to debug.

#### With rust-lldb

```bash
rust-lldb target/debug/vpass
```

---

## Troubleshooting

### Issue: `sqlx::Error: PoolTimedOut`

**Solution**: PostgreSQL isn't running or DATABASE_URL is incorrect

```bash
# Check if PostgreSQL is running
docker-compose ps  # For Docker
pg_isready  # For local PostgreSQL

# Test connection
psql $DATABASE_URL -c "SELECT 1"
```

### Issue: OAuth redirect fails with "redirect_uri_mismatch"

**Solution**: Ensure redirect URI in platform console matches exactly

- YouTube: `http://localhost:3000/auth/youtube/callback`
- Twitch: `http://localhost:3000/auth/twitch/callback`

(No trailing slash, exact port number)

### Issue: Compilation errors about missing `sqlx-data.json`

**Solution**: SQLx compile-time verification needs database to be running

```bash
# Ensure database is up and migrated
sqlx migrate run

# Generate sqlx metadata
cargo sqlx prepare
```

Or use offline mode:

```toml
# Add to Cargo.toml
[dependencies]
sqlx = { version = "0.7", features = ["runtime-tokio-rustls", "postgres", "offline"] }
```

### Issue: "Address already in use" when starting server

**Solution**: Another process is using port 3000

```bash
# Find process using port 3000
lsof -ti:3000

# Kill it
kill -9 $(lsof -ti:3000)

# Or change PORT in .env
```

---

## Next Steps

1. **Read Architecture Docs**: Review `research.md` and `data-model.md`
2. **Review API Contracts**: See `contracts/openapi.yaml`
3. **Start Implementing**: Follow tasks in `tasks.md` (generated by `/speckit.tasks`)
4. **Write Tests First**: Follow TDD workflow (if required by project constitution)

---

## Useful Resources

- **Rust Documentation**: https://doc.rust-lang.org/
- **Axum Guide**: https://docs.rs/axum/latest/axum/
- **SQLx Guide**: https://docs.rs/sqlx/latest/sqlx/
- **YouTube Data API**: https://developers.google.com/youtube/v3
- **Twitch API**: https://dev.twitch.tv/docs/api/

---

## Getting Help

- Check existing issues: `<repository-issues-url>`
- Ask in project chat/Slack: `<team-chat-url>`
- Review spec documents: `specs/001-channel-membership-verification/`

---

**Ready to code!** 🚀
