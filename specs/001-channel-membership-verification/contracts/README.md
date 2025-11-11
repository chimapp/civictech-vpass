# VPass API Documentation

VPass enables YouTube channel members to claim digital membership cards via OAuth authentication and comment verification. This document describes the REST API endpoints for card issuance, display, and verification.

**API Base URL**: `http://localhost:3000` (development)

**Performance Target**: Card issuance completes in <5 seconds (NFR-001)

---

## Table of Contents

- [Authentication](#authentication)
- [Card Management](#card-management)
- [Verification (OIDVP)](#verification-oidvp)
- [Issuers](#issuers)
- [Error Responses](#error-responses)
- [Performance Characteristics](#performance-characteristics)

---

## Authentication

All authenticated endpoints require a session cookie (`vpass_session`). Sessions are managed via `tower-sessions` with encrypted storage.

### Initiate OAuth Login

```http
GET /auth/youtube/login?role={role}&issuer_id={issuer_id}
```

**Query Parameters:**
- `role`: Either `member` (for claiming cards) or `organizer` (for verification)
- `issuer_id` (optional): UUID of card issuer for pre-filled flow

**Response:** 302 redirect to YouTube OAuth consent page

**Example:**
```bash
curl -i "http://localhost:3000/auth/youtube/login?role=member&issuer_id=550e8400-e29b-41d4-a716-446655440000"
```

---

### OAuth Callback

```http
GET /auth/youtube/callback?code={code}&state={state}
```

**Query Parameters:**
- `code`: OAuth authorization code from YouTube
- `state`: State parameter for CSRF protection

**Response:** 302 redirect to appropriate page based on role
- Member role → `/cards/claim/{issuer_id}` or `/cards/my-cards`
- Organizer role → `/verify` (event list)

**Sets Cookie:** `vpass_session` (HttpOnly, Secure, SameSite=Lax)

---

### Logout

```http
POST /auth/logout
```

**Authentication:** Required

**Response:** 302 redirect to `/`

---

## Card Management

### Claim Card Page

```http
GET /cards/claim/{issuer_id}
```

**Authentication:** Required (member role)

**Path Parameters:**
- `issuer_id`: UUID of the card issuer

**Response:** HTML page with claim form

---

### Issue Card

```http
POST /cards/issue
Content-Type: application/x-www-form-urlencoded
```

**Authentication:** Required (member role)

**Form Parameters:**
- `issuer_id`: UUID (required)
- `comment_link_or_id`: YouTube comment URL or comment ID (required)

**Response:** 302 redirect to `/cards/{card_id}` on success

**Error Responses:**
- `400 Bad Request`: Invalid comment URL/ID format
- `403 Forbidden`: Comment not found, wrong author, or wrong video
- `409 Conflict`: Active unexpired card already exists (message includes expiration date)
- `503 Service Unavailable`: Taiwan Digital Wallet service unavailable

**Performance:** Target <5 seconds (includes YouTube API + wallet API calls)

**Example:**
```bash
curl -X POST \
  -b "vpass_session=..." \
  -d "issuer_id=550e8400-e29b-41d4-a716-446655440000" \
  -d "comment_link_or_id=https://www.youtube.com/watch?v=dQw4w9WgXcQ&lc=UgxABC123" \
  "http://localhost:3000/cards/issue"
```

---

### Show Card

```http
GET /cards/{card_id}
```

**Authentication:** Required (must own the card)

**Path Parameters:**
- `card_id`: UUID of the membership card

**Response:** HTML page displaying card details including:
- Membership level label
- Expiration date (30 days from issuance)
- Taiwan Digital Wallet QR code (if not scanned)
- Credential issuance status
- Card ID

**Example:**
```bash
curl -b "vpass_session=..." \
  "http://localhost:3000/cards/c0ffee00-0000-0000-0000-000000000001"
```

---

### List My Cards

```http
GET /cards/my-cards
```

**Authentication:** Required

**Response:** HTML page listing all cards owned by the authenticated member

---

### Get Card QR Code

```http
GET /cards/{card_id}/qr
```

**Authentication:** Required (must own the card)

**Path Parameters:**
- `card_id`: UUID of the membership card

**Response:**
- `Content-Type: image/png` (QR code image)
- `404 Not Found` if card has no QR code

---

### Poll Credential Status

```http
GET /cards/{card_id}/poll-credential
```

**Authentication:** Required (must own the card)

**Path Parameters:**
- `card_id`: UUID of the membership card

**Response:**
```json
{
  "status": "ready",
  "cid": "a16187e9-755e-48ca-a9c0-622f76fe1360"
}
```

Or:
```json
{
  "status": "pending"
}
```

**Polling Strategy:** Frontend polls every 2 seconds, max 150 attempts (5 minutes)

---

### Delete Card

```http
DELETE /cards/{card_id}
```

**Authentication:** Required (must own the card)

**Path Parameters:**
- `card_id`: UUID of the membership card

**Response:** `200 OK` (soft-deletes the card)

---

## Verification (OIDVP)

VPass implements OpenID for Verifiable Presentations (OIDVP) protocol for event-based verification.

### List Events

```http
GET /verify
```

**Authentication:** Required (organizer role)

**Response:** HTML page listing active verification events

---

### Scanner Page

```http
GET /verify/{event_id}/scanner
```

**Authentication:** Required (organizer role, must own event)

**Path Parameters:**
- `event_id`: UUID of the verification event

**Response:** HTML page with QR scanner interface

---

### Request Verification QR

```http
POST /verify/{event_id}/request-qr
Content-Type: application/json
```

**Authentication:** Required (organizer role, must own event)

**Path Parameters:**
- `event_id`: UUID of the verification event

**Request Body:**
```json
{
  "purpose": "Event attendance verification"
}
```

**Response:**
```json
{
  "transaction_id": "txn_abc123",
  "qr_code": "data:image/png;base64,...",
  "expires_at": "2025-11-11T12:34:56Z"
}
```

---

### Check Verification Result

```http
GET /verify/{event_id}/check-result/{transaction_id}
```

**Authentication:** Required (organizer role, must own event)

**Path Parameters:**
- `event_id`: UUID of the verification event
- `transaction_id`: Transaction ID from request-qr

**Response (pending):**
```json
{
  "status": "pending"
}
```

**Response (success):**
```json
{
  "status": "verified",
  "member_name": "John Doe",
  "membership_level": "Member",
  "verified_at": "2025-11-11T12:34:56Z"
}
```

**Polling Strategy:** Poll every 500ms, max 60 seconds

---

### Verification History

```http
GET /verify/{event_id}/history
```

**Authentication:** Required (organizer role, must own event)

**Path Parameters:**
- `event_id`: UUID of the verification event

**Response:** HTML page showing audit log of all verifications for this event

---

## Issuers

### List Card Issuers

```http
GET /issuers
```

**Authentication:** Not required

**Response:**
```json
{
  "issuers": [
    {
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "channel_name": "Example Channel",
      "channel_handle": "@example",
      "verification_video_id": "dQw4w9WgXcQ",
      "default_membership_label": "Member"
    }
  ]
}
```

---

## Error Responses

VPass uses HTTP status codes and HTML error pages for browser requests. API endpoints may return JSON errors.

### HTTP Status Codes

- **200 OK**: Success
- **302 Found**: Redirect (OAuth flows, post-action redirects)
- **400 Bad Request**: Invalid input (malformed comment URL, missing parameters)
- **401 Unauthorized**: Not authenticated (missing or invalid session)
- **403 Forbidden**: Authenticated but not authorized (comment ownership mismatch, wrong video)
- **404 Not Found**: Resource not found (card, issuer, event)
- **409 Conflict**: Duplicate card exists (active unexpired card already issued)
- **500 Internal Server Error**: Server-side error
- **503 Service Unavailable**: External API unavailable (wallet API, YouTube API)

### Common Error Messages

**Duplicate Card:**
```
Active card already exists. Expires: 2025-12-10
```

**Wallet Service Unavailable:**
```
Taiwan Digital Wallet service unavailable. Please try again later.
```

**Comment Verification Failures:**
- `Comment not found`
- `Comment does not belong to authenticated user`
- `Comment is not on the verification video`

**YouTube API Errors:**
- `Rate limit exceeded after retries` (429, retried 3x with exponential backoff)
- `Service unavailable after retries` (503, retried 3x with exponential backoff)

---

## Performance Characteristics

### Card Issuance (NFR-001)

**Target:** <5 seconds end-to-end

**Breakdown:**
- YouTube API (comment verification): ~500-2000ms
- Wallet API (QR generation): ~500-1500ms
- Database operations: ~50-200ms
- Total overhead: ~100ms

**Logging:** Performance metrics logged with sub-step durations. Warnings logged if >5 seconds.

**Example Log Output:**
```
INFO Card issuance completed within target
  duration_secs=3.421
  youtube_api_ms=1823
  wallet_api_ms=1204
  card_id=c0ffee00-0000-0000-0000-000000000001
```

### YouTube API Retry Logic (FR-009a)

**Max Attempts:** 3
**Backoff:** Exponential (1s, 2s, 4s delays)
**Total Window:** ~7 seconds (within 30s limit)
**Retryable Errors:** 429 Too Many Requests, 503 Service Unavailable
**Non-retryable:** 400, 401, 404 (fail immediately)

### Wallet API Fail-Fast (FR-008a)

**Health Check:** HEAD request with 3-second timeout
**Timing:** Before YouTube API call (avoids wasted quota)
**Failure Behavior:** Immediate error, no partial card creation

---

## Implementation Notes

### Session Management

- **Cookie Name:** `vpass_session`
- **Storage:** PostgreSQL (encrypted)
- **Attributes:** HttpOnly, Secure (production), SameSite=Lax
- **Expiration:** 7 days idle timeout

### OAuth Tokens

- **Storage:** `oauth_sessions` table (AES-256-GCM encrypted)
- **Fields:** `access_token`, `refresh_token`, `expires_at`
- **Key:** Configured via `ENCRYPTION_KEY` environment variable

### Card Expiration

- **Default:** 30 days from issuance
- **Renewal:** Handled by cronjob (feature 003-card-lifecycle-automation)
- **Display:** Expiration date shown on card page
- **Expired UI:** QR code replaced with "卡片已過期" message

### Comment Verification (FR-003)

- **No Age Restriction:** Comments from any date accepted
- **Validation:** Author identity + video target only
- **API:** YouTube Data API v3 `comments.list`

---

## Example Flows

### Member Claims Card

1. `GET /auth/youtube/login?role=member&issuer_id={uuid}` → OAuth consent
2. YouTube redirects to `/auth/youtube/callback?code=...`
3. System creates session, redirects to `/cards/claim/{issuer_id}`
4. Member posts comment on members-only video
5. Member submits `POST /cards/issue` with comment URL
6. System verifies comment (YouTube API), generates QR (wallet API), creates card
7. Redirect to `/cards/{card_id}` showing QR code
8. Member scans QR in Taiwan Digital Wallet app
9. Frontend polls `/cards/{card_id}/poll-credential` until credential ready

### Organizer Verifies Member

1. `GET /auth/youtube/login?role=organizer` → OAuth consent
2. Redirects to `/verify` (event list)
3. `GET /verify/{event_id}/scanner` → Scanner interface
4. `POST /verify/{event_id}/request-qr` → Generate verification QR
5. Member scans QR in Taiwan Digital Wallet app (OIDVP presentation)
6. Frontend polls `/verify/{event_id}/check-result/{txn_id}`
7. System validates presentation, returns member info
8. Verification logged to `verification_events` table

---

## Testing Tools

### Browser Testing
- Use Chrome/Firefox for OAuth flows (requires browser redirect)
- Check Developer Console → Application → Cookies for session
- Network tab shows performance timing

### Command Line

```bash
# List issuers
curl "http://localhost:3000/issuers"

# Issue card (requires valid session cookie)
curl -X POST \
  -b "vpass_session=SESSION_VALUE" \
  -d "issuer_id=550e8400-e29b-41d4-a716-446655440000" \
  -d "comment_link_or_id=UgxDirect123" \
  "http://localhost:3000/cards/issue"

# Poll credential status
curl -b "vpass_session=SESSION_VALUE" \
  "http://localhost:3000/cards/c0ffee00-0000-0000-0000-000000000001/poll-credential"
```

### Environment Setup

See [quickstart.md](../quickstart.md) for:
- PostgreSQL setup
- Environment variables
- Database migrations
- Development server

---

## Future Enhancements

- Health check endpoint (`/health`) with dependency status
- Metrics endpoint for monitoring
- WebSocket support for real-time updates
- Batch card operations
- Admin API for issuer management
- GraphQL API as alternative to REST

---

**Last Updated:** 2025-11-11
**API Version:** 1.0.0
**Feature:** 001-channel-membership-verification
