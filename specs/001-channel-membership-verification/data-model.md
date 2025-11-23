# Data Model: Channel Membership Verification System (MVP)

**Feature**: 001-channel-membership-verification  
**Date**: 2025-10-12  
**Database**: PostgreSQL + SQLx

## Overview

The extended MVP includes issuer configuration, OAuth sessions, member profiles, membership cards with integrated wallet data, event management, and OIDVP-based verification tracking. All identifiers are UUIDs, timestamps are UTC.

**Note**: The following schema reflects the actual implemented system, which includes features from spec-002 (event management, OIDVP verification) implemented alongside the original MVP.

**Architectural Simplifications (2025-11-10)**:
- Wallet QR fields merged into `membership_cards` (was separate 1:1 `wallet_qr_codes` table)
- Verification session state managed in frontend JavaScript (no `verification_sessions` table)
- Only successful verifications persisted in `verification_events` (audit log)

```
card_issuers ─┬── membership_cards (includes wallet QR data)
              │          │
              │          └── verification_events
              │
              └── events ─── verification_events

oauth_sessions ──┐
                 └── members
```

---

## Table: `card_issuers`

Represents YouTube channels authorized to issue cards.

| Column | Type | Constraints | Notes |
|--------|------|-------------|-------|
| id | UUID | PK | |
| platform | TEXT | NOT NULL, CHECK (platform = 'youtube') | Reserved for future multi-platform expansion |
| youtube_channel_id | TEXT | NOT NULL, UNIQUE | `channels.list` ID |
| channel_handle | TEXT | NULL | Optional `@handle` |
| channel_name | TEXT | NOT NULL | Display name |
| verification_video_id | TEXT | NOT NULL | Members-only video used for membership verification |
| default_membership_label | TEXT | NOT NULL | Label included on issued cards (e.g., "Channel Member") |
| is_active | BOOLEAN | NOT NULL DEFAULT TRUE | Allows soft disabling |
| created_at | TIMESTAMPTZ | NOT NULL DEFAULT NOW() | |
| updated_at | TIMESTAMPTZ | NOT NULL DEFAULT NOW() | Update trigger recommended |

Indexes:
- `idx_card_issuers_channel` ON (youtube_channel_id)
- `idx_card_issuers_active` ON (is_active) WHERE is_active = TRUE

---

## Table: `oauth_sessions`

Stores encrypted OAuth tokens for authenticated members.

| Column | Type | Constraints | Notes |
|--------|------|-------------|-------|
| id | UUID | PK | |
| member_id | UUID | NOT NULL REFERENCES members(id) ON DELETE CASCADE | |
| access_token | BYTEA | NOT NULL | AEAD-encrypted blob |
| refresh_token | BYTEA | NULL | AEAD-encrypted blob |
| token_scope | TEXT | NOT NULL | Space-delimited |
| token_expires_at | TIMESTAMPTZ | NOT NULL | |
| created_at | TIMESTAMPTZ | NOT NULL DEFAULT NOW() | |
| last_used_at | TIMESTAMPTZ | NOT NULL DEFAULT NOW() | |

Indexes:
- `idx_oauth_sessions_member` ON (member_id)
- `idx_oauth_sessions_expires` ON (token_expires_at)

Security:
- Encrypt tokens with AES-256-GCM; store nonce alongside ciphertext (prepend to BYTEA).
- Access tokens should be re-encrypted whenever keys rotate.

---

## Table: `members`

Cached metadata about a YouTube member at issuance time.

| Column | Type | Constraints | Notes |
|--------|------|-------------|-------|
| id | UUID | PK | Internal identifier |
| youtube_user_id | TEXT | NOT NULL, UNIQUE | Stable user identifier |
| default_display_name | TEXT | NOT NULL | Latest display name observed during membership verification |
| avatar_url | TEXT | NULL | Optional CDN URL |
| locale | TEXT | NULL | ISO language tag |
| created_at | TIMESTAMPTZ | NOT NULL DEFAULT NOW() | |
| updated_at | TIMESTAMPTZ | NOT NULL DEFAULT NOW() | |

Indexes:
- `idx_members_youtube_user_id` ON (youtube_user_id)

---

## Table: `membership_cards`

Record of cards issued to members. Includes integrated Taiwan Digital Wallet (數位皮夾) QR data.

| Column | Type | Constraints | Notes |
|--------|------|-------------|-------|
| id | UUID | PK | Used in QR payload |
| issuer_id | UUID | NOT NULL REFERENCES card_issuers(id) | |
| member_id | UUID | NOT NULL REFERENCES members(id) | |
| membership_level_label | TEXT | NOT NULL | Issuer-defined human-readable label (e.g., "Channel Member") |
| membership_confirmed_at | TIMESTAMPTZ | NOT NULL | Timestamp when membership access was verified |
| verification_comment_id | TEXT | NOT NULL | Membership verification token/marker (historically comment ID) |
| verification_video_id | TEXT | NOT NULL | Members-only verification video ID |
| snapshot_json | JSONB | NOT NULL | Raw payload snapshot for auditing (verification context) |
| status | ENUM(card_status) | NOT NULL DEFAULT 'active' | Card lifecycle state: active, expired, revoked, suspended, deleted |
| expires_at | TIMESTAMPTZ | NULL | Card expiration (30 days default) |
| last_verified_at | TIMESTAMPTZ | NULL | Last membership verification timestamp |
| verification_failures | INT | NOT NULL DEFAULT 0 | Counter for failed verification attempts |
| **wallet_transaction_id** | TEXT | NULL | Taiwan wallet API transaction ID |
| **wallet_qr_code** | TEXT | NULL | Base64-encoded QR code image (data URL) |
| **wallet_deep_link** | TEXT | NULL | Deep link to open in wallet app |
| **wallet_cid** | TEXT | NULL | Credential ID after wallet import |
| **wallet_scanned_at** | TIMESTAMPTZ | NULL | Timestamp when wallet scanned QR |
| deleted_at | TIMESTAMPTZ | NULL | Soft deletion timestamp; NULL = active card |
| issued_at | TIMESTAMPTZ | NOT NULL DEFAULT NOW() | |

**Note**: Wallet-related fields (bold) were moved from a separate `wallet_qr_codes` table (1:1 relationship, simplified 2025-11-10).

Unique Constraints:
- Cards use status-based uniqueness (one active card per issuer/member pair)

Indexes:
- `idx_membership_cards_member` ON (member_id, issued_at DESC)
- `idx_membership_cards_issuer` ON (issuer_id, issued_at DESC)
- `idx_membership_cards_transaction_id` ON (wallet_transaction_id)

---

## Removed Tables (Architectural Simplifications)

### `wallet_qr_codes` (removed 2025-11-10)
Previously a separate 1:1 table for Taiwan Digital Wallet QR data. Simplified by merging fields directly into `membership_cards` since:
- One card generates one QR code (1:1 relationship)
- Members don't need to re-scan
- Only one wallet format supported

### `verification_sessions` (removed 2025-11-10)
Previously tracked pending verification state in database. Simplified by managing state in frontend JavaScript since:
- Session state is ephemeral (5-minute timeout)
- Frontend already polls for status
- Only successful verifications need persistence (in `verification_events`)
- Reduced unnecessary database writes for temporary state
