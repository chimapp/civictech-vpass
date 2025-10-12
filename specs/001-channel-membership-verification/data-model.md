# Data Model: Channel Membership Verification System

**Feature**: 001-channel-membership-verification
**Date**: 2025-10-12
**Database**: PostgreSQL with SQLx

## Overview

This document defines the database schema for the membership card system. All tables use UUID primary keys for security and distributed system compatibility. Timestamps are stored in UTC.

---

## Schema Diagram

```
┌─────────────────┐          ┌──────────────────┐
│  card_issuers   │◄─────────│ membership_cards │
└─────────────────┘          └──────────────────┘
         │                            │
         │                            │
         │                            ▼
         │                   ┌──────────────────┐
         │                   │   revocations    │
         │                   └──────────────────┘
         │                            │
         │                            │
         ▼                            ▼
┌─────────────────┐          ┌──────────────────┐
│ oauth_sessions  │          │verification_events│
└─────────────────┘          └──────────────────┘
```

---

## Table: `card_issuers`

Represents channel owners or communities who can issue membership cards.

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| id | UUID | PRIMARY KEY | Unique issuer identifier |
| issuer_type | VARCHAR(50) | NOT NULL | `official_channel` or `community` |
| platform | VARCHAR(50) | NOT NULL | `youtube` or `twitch` |
| platform_channel_id | VARCHAR(255) | NOT NULL, UNIQUE | Platform-specific channel/broadcaster ID |
| channel_name | VARCHAR(255) | NOT NULL | Display name of the channel |
| is_verified | BOOLEAN | NOT NULL, DEFAULT FALSE | Whether issuer has completed OAuth verification |
| created_at | TIMESTAMPTZ | NOT NULL, DEFAULT NOW() | Creation timestamp |
| updated_at | TIMESTAMPTZ | NOT NULL, DEFAULT NOW() | Last update timestamp |

**Indexes**:
- `idx_issuers_platform_channel` ON (platform, platform_channel_id)
- `idx_issuers_verified` ON (is_verified) WHERE is_verified = TRUE

**Validation Rules**:
- `platform` must be in (`youtube`, `twitch`)
- `issuer_type` must be in (`official_channel`, `community`)
- `platform_channel_id` format validated per platform

---

## Table: `oauth_sessions`

Stores OAuth authentication sessions for both members and organizers.

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| id | UUID | PRIMARY KEY | Unique session identifier |
| user_role | VARCHAR(50) | NOT NULL | `member` or `organizer` |
| platform | VARCHAR(50) | NOT NULL | `youtube` or `twitch` |
| platform_user_id | VARCHAR(255) | NOT NULL | Platform-specific user ID |
| access_token | TEXT | NOT NULL | Encrypted OAuth access token |
| refresh_token | TEXT | NULL | Encrypted OAuth refresh token (if provided) |
| token_expires_at | TIMESTAMPTZ | NOT NULL | Access token expiration time |
| scope | TEXT | NOT NULL | OAuth scopes granted |
| issuer_id | UUID | NULL, FOREIGN KEY | References `card_issuers(id)` if user is organizer |
| created_at | TIMESTAMPTZ | NOT NULL, DEFAULT NOW() | Session creation timestamp |
| last_used_at | TIMESTAMPTZ | NOT NULL, DEFAULT NOW() | Last activity timestamp |

**Indexes**:
- `idx_sessions_platform_user` ON (platform, platform_user_id)
- `idx_sessions_issuer` ON (issuer_id) WHERE issuer_id IS NOT NULL
- `idx_sessions_expiry` ON (token_expires_at)

**Validation Rules**:
- `user_role` must be in (`member`, `organizer`)
- `platform` must be in (`youtube`, `twitch`)
- `issuer_id` must be NULL if `user_role` is `member`
- `issuer_id` must be NOT NULL if `user_role` is `organizer`

**Security**:
- `access_token` and `refresh_token` are encrypted at rest using AES-256-GCM
- Tokens are decrypted only when needed for API calls
- Use `secrecy::Secret` in Rust code to prevent logging

---

## Table: `membership_cards`

Represents issued membership cards.

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| id | UUID | PRIMARY KEY | Unique card identifier (used in QR code) |
| issuer_id | UUID | NOT NULL, FOREIGN KEY | References `card_issuers(id)` |
| platform | VARCHAR(50) | NOT NULL | `youtube` or `twitch` |
| member_platform_id | VARCHAR(255) | NOT NULL | Platform-specific member user ID |
| member_display_name | VARCHAR(255) | NOT NULL | Member's display name |
| membership_level | VARCHAR(100) | NULL | Membership tier (e.g., "Member", "Sponsor") |
| subscription_start_date | DATE | NOT NULL | When subscription started |
| subscription_duration_months | INTEGER | NULL | Total months subscribed |
| is_active_member | BOOLEAN | NOT NULL, DEFAULT TRUE | Based on engagement metrics |
| supporter_metrics | JSONB | NULL | Superchats, gift subs, etc. |
| supplementary_data | JSONB | NULL | User-provided additional fields |
| qr_code_payload | TEXT | NOT NULL | JSON string encoded in QR code |
| qr_code_signature | VARCHAR(64) | NOT NULL | HMAC-SHA256 signature of payload |
| is_revoked | BOOLEAN | NOT NULL, DEFAULT FALSE | Whether card is revoked |
| needs_refresh | BOOLEAN | NOT NULL, DEFAULT FALSE | Whether card needs re-issuance |
| issued_at | TIMESTAMPTZ | NOT NULL, DEFAULT NOW() | Card issuance timestamp |
| revoked_at | TIMESTAMPTZ | NULL | When card was revoked |

**Indexes**:
- `idx_cards_issuer` ON (issuer_id)
- `idx_cards_member` ON (platform, member_platform_id)
- `idx_cards_active` ON (is_revoked, needs_refresh) WHERE is_revoked = FALSE
- `idx_cards_issued_at` ON (issued_at DESC)

**Unique Constraint**:
- `unique_active_card_per_member` UNIQUE (issuer_id, platform, member_platform_id) WHERE is_revoked = FALSE AND needs_refresh = FALSE

**Validation Rules**:
- `platform` must be in (`youtube`, `twitch`)
- `subscription_duration_months` >= 0 if not NULL
- `qr_code_signature` must be 64 hex characters
- Cannot set `revoked_at` without `is_revoked = TRUE`

**State Transitions**:
```
[New Card]
   ↓
[Active] (is_revoked=false, needs_refresh=false)
   ↓
[Needs Refresh] (is_revoked=false, needs_refresh=true)
   OR
[Revoked] (is_revoked=true, revoked_at=NOW)
```

---

## Table: `revocations`

Audit log for card revocations.

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| id | UUID | PRIMARY KEY | Unique revocation record ID |
| card_id | UUID | NOT NULL, FOREIGN KEY | References `membership_cards(id)` |
| reason | VARCHAR(100) | NOT NULL | Revocation reason code |
| reason_detail | TEXT | NULL | Detailed explanation |
| new_card_id | UUID | NULL, FOREIGN KEY | References `membership_cards(id)` if refreshed |
| revoked_by | VARCHAR(50) | NOT NULL | `system` or `manual` |
| revoked_at | TIMESTAMPTZ | NOT NULL, DEFAULT NOW() | Revocation timestamp |

**Indexes**:
- `idx_revocations_card` ON (card_id)
- `idx_revocations_timestamp` ON (revoked_at DESC)

**Validation Rules**:
- `reason` must be in (`subscription_canceled`, `membership_changed`, `manual_revocation`, `security_issue`)
- `revoked_by` must be in (`system`, `manual`)
- `new_card_id` must reference a valid card if not NULL

---

## Table: `verification_events`

Log of card verification attempts by organizers.

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| id | UUID | PRIMARY KEY | Unique event identifier |
| card_id | UUID | NOT NULL, FOREIGN KEY | References `membership_cards(id)` |
| verifier_issuer_id | UUID | NOT NULL, FOREIGN KEY | References `card_issuers(id)` |
| verification_result | VARCHAR(50) | NOT NULL | `success`, `revoked`, `expired`, `invalid_signature` |
| verification_context | JSONB | NULL | Additional context (event name, location, etc.) |
| verified_at | TIMESTAMPTZ | NOT NULL, DEFAULT NOW() | Verification timestamp |

**Indexes**:
- `idx_verifications_card` ON (card_id)
- `idx_verifications_issuer` ON (verifier_issuer_id)
- `idx_verifications_timestamp` ON (verified_at DESC)
- `idx_verifications_result` ON (verification_result)

**Validation Rules**:
- `verification_result` must be in (`success`, `revoked`, `expired`, `invalid_signature`, `wrong_issuer`)

---

## Table: `subscription_check_jobs`

Tracks cron job runs for subscription status checking.

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| id | UUID | PRIMARY KEY | Unique job run identifier |
| job_started_at | TIMESTAMPTZ | NOT NULL, DEFAULT NOW() | When job started |
| job_completed_at | TIMESTAMPTZ | NULL | When job completed |
| cards_checked | INTEGER | NOT NULL, DEFAULT 0 | Number of cards checked |
| cards_revoked | INTEGER | NOT NULL, DEFAULT 0 | Number of cards revoked |
| errors_count | INTEGER | NOT NULL, DEFAULT 0 | Number of errors encountered |
| error_details | JSONB | NULL | Details of any errors |
| job_status | VARCHAR(50) | NOT NULL, DEFAULT 'running' | `running`, `completed`, `failed` |

**Indexes**:
- `idx_jobs_started` ON (job_started_at DESC)
- `idx_jobs_status` ON (job_status)

**Validation Rules**:
- `job_status` must be in (`running`, `completed`, `failed`)
- `job_completed_at` must be >= `job_started_at` if not NULL

---

## Migration Strategy

### Initial Migration

Create file: `migrations/00001_initial_schema.sql`

```sql
-- Enable UUID extension
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Create tables in order (respecting foreign keys)
CREATE TABLE card_issuers (...);
CREATE TABLE oauth_sessions (...);
CREATE TABLE membership_cards (...);
CREATE TABLE revocations (...);
CREATE TABLE verification_events (...);
CREATE TABLE subscription_check_jobs (...);

-- Create indexes
-- Create constraints
```

### Future Migrations

- Use timestamp prefixes: `YYYYMMDDHHMMSS_description.sql`
- Always include both `up` and `down` migrations
- Test migrations on staging before production
- Document breaking changes in migration comments

---

## Sample Data Queries

### Create a new card issuer
```sql
INSERT INTO card_issuers (id, issuer_type, platform, platform_channel_id, channel_name, is_verified)
VALUES (
    uuid_generate_v4(),
    'official_channel',
    'youtube',
    'UCxxxxxxxxxxxxx',
    'Example Gaming Channel',
    TRUE
);
```

### Issue a membership card
```sql
INSERT INTO membership_cards (
    id, issuer_id, platform, member_platform_id, member_display_name,
    membership_level, subscription_start_date, qr_code_payload, qr_code_signature
)
VALUES (
    uuid_generate_v4(),
    '...issuer_id...',
    'youtube',
    'UCyyyyyyyyyy',
    'MemberUsername',
    'Sponsor',
    '2024-01-15',
    '{"card_id":"...","issuer_id":"..."}',
    'abc123...'
);
```

### Find active cards for a member
```sql
SELECT * FROM membership_cards
WHERE platform = 'youtube'
  AND member_platform_id = 'UCyyyyyyyyyy'
  AND is_revoked = FALSE
  AND needs_refresh = FALSE;
```

### Log a verification event
```sql
INSERT INTO verification_events (id, card_id, verifier_issuer_id, verification_result)
VALUES (uuid_generate_v4(), '...card_id...', '...issuer_id...', 'success');
```

---

## Entity Relationships

### One-to-Many Relationships

- `card_issuers` → `membership_cards`: One issuer can issue many cards
- `card_issuers` → `oauth_sessions`: One issuer can have many sessions (different devices)
- `membership_cards` → `revocations`: One card can have multiple revocation records (audit trail)
- `membership_cards` → `verification_events`: One card can be verified many times

### Optional Relationships

- `oauth_sessions.issuer_id` → `card_issuers`: Only set for organizer sessions
- `revocations.new_card_id` → `membership_cards`: Only set when card is refreshed

---

## Data Integrity Rules

1. **Card Uniqueness**: At most one active card per (issuer, platform, member) combination
2. **Revocation Consistency**: If `membership_cards.is_revoked = TRUE`, must have corresponding `revocations` record
3. **OAuth Session Cleanup**: Delete sessions older than 90 days with no activity
4. **Verification Authorization**: Verifier can only verify cards issued by their own channel (enforced in application logic)

---

## Performance Considerations

1. **Partitioning**: Consider partitioning `verification_events` by timestamp if table grows large (> 10M rows)
2. **Archival**: Archive revoked cards older than 1 year to separate table
3. **Caching**: Cache frequently accessed issuer data in application memory
4. **Connection Pooling**: Use `sqlx::PgPool` with 10-50 connections depending on load

---

## Security Considerations

1. **Token Encryption**: OAuth tokens encrypted with AES-256-GCM using key from environment
2. **QR Signature**: HMAC-SHA256 signature prevents QR code tampering
3. **No PII Logging**: Never log member platform IDs or tokens
4. **Access Control**: Use database roles with minimal privileges
5. **Audit Trail**: `revocations` and `verification_events` tables provide complete audit log

---

## Next Steps

- **API Contracts**: Define REST endpoints that interact with this data model
- **Quickstart Guide**: Document how to run migrations and seed test data
- **Implementation Tasks**: Break down into implementation tasks in `tasks.md`
