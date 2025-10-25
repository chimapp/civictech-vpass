# Data Model: Channel Membership Verification System (MVP)

**Feature**: 001-channel-membership-verification  
**Date**: 2025-10-12  
**Database**: PostgreSQL + SQLx

## Overview

The MVP requires persistent storage for issuer configuration, OAuth sessions, cached member metadata, and issued membership cards. Revocation logs, verification events, and non-YouTube platform data are intentionally deferred to future specs. All identifiers are UUIDs, timestamps are UTC.

```
card_issuers ─┐
              ├── membership_cards
oauth_sessions┘             ▲
      │                     │
      └──── members ────────┘
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
| verification_video_id | TEXT | NOT NULL | Members-only video used for comment verification |
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
| default_display_name | TEXT | NOT NULL | Pulled from YouTube comment author metadata |
| avatar_url | TEXT | NULL | Optional CDN URL |
| locale | TEXT | NULL | ISO language tag |
| created_at | TIMESTAMPTZ | NOT NULL DEFAULT NOW() | |
| updated_at | TIMESTAMPTZ | NOT NULL DEFAULT NOW() | |

Indexes:
- `idx_members_youtube_user_id` ON (youtube_user_id)

---

## Table: `membership_cards`

Immutable record of cards issued to members.

| Column | Type | Constraints | Notes |
|--------|------|-------------|-------|
| id | UUID | PK | Used in QR payload |
| issuer_id | UUID | NOT NULL REFERENCES card_issuers(id) | |
| member_id | UUID | NOT NULL REFERENCES members(id) | |
| membership_level_label | TEXT | NOT NULL | Issuer-defined human-readable label (e.g., "Channel Member") |
| membership_confirmed_at | TIMESTAMPTZ | NOT NULL | Comment publication timestamp used for verification |
| verification_comment_id | TEXT | NOT NULL | `comments.list` ID |
| verification_video_id | TEXT | NOT NULL | Members-only verification video ID |
| snapshot_json | JSONB | NOT NULL | Raw payload snapshot for auditing (comment + verification context) |
| qr_payload | JSONB | NOT NULL | Data encoded into QR |
| qr_signature | TEXT | NOT NULL | Hex-encoded HMAC-SHA256 |
| deleted_at | TIMESTAMPTZ | NULL | Soft deletion timestamp; NULL = active card |
| issued_at | TIMESTAMPTZ | NOT NULL DEFAULT NOW() | |

Unique Constraints:
- `uniq_active_membership_card` UNIQUE (issuer_id, member_id) WHERE deleted_at IS NULL

Indexes:
- `idx_membership_cards_member` ON (member_id, issued_at DESC)
- `idx_membership_cards_issuer` ON (issuer_id, issued_at DESC)

---

## Supporting Views / Future Expansion

- Add `revocations`, `verification_events`, and multi-platform columns in follow-up specs.
- Consider a materialized view or cached table for frequently accessed QR payloads once organizer verification exists.
