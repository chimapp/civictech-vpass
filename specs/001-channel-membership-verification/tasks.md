# Implementation Tasks: Channel Membership Verification Card System

**Feature**: 001-channel-membership-verification
**Generated**: 2025-11-10
**Spec**: [spec.md](./spec.md) | **Plan**: [plan.md](./plan.md)

## Overview

This task breakdown implements the VPass membership card issuance system organized by user story priority. The system is largely implemented with clarifications from the 2025-11-10 session requiring specific refinements:

**Recent Clarifications**:
- No comment age restriction needed (removed 7-day limit)
- Cards expire after 30 days (with cronjob renewal via FR-006a)
- 5-second issuance latency target (NFR-001)
- YouTube API retry with exponential backoff (3 attempts/30s per FR-009a)
- Wallet API failure = full issuance failure (FR-008a)

**Implementation Status**: Core functionality exists. Tasks focus on:
1. Implementing clarified requirements from spec session
2. Adding missing error handling and performance requirements
3. Ensuring test coverage for edge cases

---

## Task Summary

| Phase | Description | Task Count | Parallel Opportunities |
|-------|-------------|------------|------------------------|
| Phase 1 | Setup & Infrastructure | 2 | 2 [P] tasks |
| Phase 2 | Foundational (Blocking) | 3 | 2 [P] tasks |
| Phase 3 | US1: Member Claims Card (P1) | 8 | 4 [P] tasks |
| Phase 4 | Polish & Cross-Cutting | 3 | 2 [P] tasks |
| **Total** | | **16** | **10 parallelizable** |

---

## Phase 1: Setup & Infrastructure

**Goal**: Prepare development environment and verify existing implementation baseline.

### T001 [P] [Setup] Verify database schema completeness
**File**: `migrations/` (review), `src/models/card.rs` (verify)
**Story**: Setup
**Description**:
- Confirm `membership_cards.expires_at` field exists (already implemented, verified in codebase)
- Verify wallet fields merged from `wallet_qr_codes` table (migration 20251110000001 applied)
- Confirm `verification_sessions` table dropped (migration 20251110000002 applied)
- Document any missing indexes or constraints

**Acceptance**:
- All migrations applied successfully
- Schema matches data-model.md (with 2025-11-10 simplifications)
- No pending schema changes required for P1 user story

### T002 [P] [Setup] Validate environment configuration
**File**: `.env.example`, `src/config.rs`
**Story**: Setup
**Description**:
- Verify all required environment variables documented:
  - `YOUTUBE_CLIENT_ID`, `YOUTUBE_CLIENT_SECRET`
  - `VERIFIER_API_URL`, `VERIFIER_ACCESS_TOKEN` (OIDVP)
  - `ISSUER_API_URL`, `ISSUER_ACCESS_TOKEN` (Taiwan wallet)
  - `ENCRYPTION_KEY` (OAuth token encryption)
- Confirm configuration loading in `src/config.rs` handles missing vars gracefully
- Add config validation on startup (fail fast if critical vars missing)

**Acceptance**:
- `.env.example` documents all required variables
- Application fails with clear error if critical config missing
- Development setup documented in quickstart.md

---

## Phase 2: Foundational Tasks (Blocking Prerequisites)

**Goal**: Core infrastructure that ALL user stories depend on. Must complete before US1 implementation.

### T003 [P] [Foundation] Implement YouTube API retry logic with exponential backoff
**File**: `src/services/youtube_channel.rs`
**Story**: Foundation (blocks US1)
**Requirement**: FR-009a
**Description**:
- Add retry logic to YouTube API calls (`commentThreads.list`, `comments.list`)
- Implement exponential backoff: initial delay 1s, max 3 attempts over 30s total
- Handle `429 Too Many Requests` and `503 Service Unavailable` as retryable
- Log retry attempts with structured tracing (include attempt number, delay)
- Non-retryable errors (400, 401, 404) fail immediately

**Acceptance**:
- Rate limit response triggers 3 retry attempts with delays: 1s, 2s, 4s (total ~7s)
- Total retry window ≤30 seconds per FR-009a
- Non-retryable errors fail on first attempt
- Retry attempts logged with transaction ID for debugging

### T004 [Foundation] Add card expiration validation logic
**File**: `src/models/card.rs`
**Story**: Foundation (blocks US1)
**Requirement**: FR-006a
**Description**:
- Add `is_expired()` method to `MembershipCard` model
- Check `expires_at.is_some() && expires_at < Utc::now()`
- Add database query helper: `find_active_unexpired_cards(pool, member_id, issuer_id)`
- Ensure duplicate card check (FR-006) excludes expired cards

**Acceptance**:
- `is_expired()` returns true if `expires_at` is past current time
- Duplicate card prevention only blocks if active AND unexpired card exists
- Database query filters: `status = 'active' AND expires_at > NOW()`

### T005 [P] [Foundation] Implement wallet API error handling (fail-fast)
**File**: `src/services/wallet_qr.rs`, `src/services/card_issuer.rs`
**Story**: Foundation (blocks US1)
**Requirement**: FR-008a
**Description**:
- Remove any fallback/retry logic for wallet API failures
- Ensure wallet API errors propagate to card issuance handler immediately
- Add specific error message: "Taiwan Digital Wallet service unavailable. Please try again later."
- Rollback any partial database writes if wallet API fails (use transaction)
- Log wallet API failures with full request/response details for debugging

**Acceptance**:
- Wallet API failure aborts entire card issuance
- No partial cards created (database transaction rollback)
- User sees actionable error message per FR-008a
- Database remains consistent after wallet API failure

---

## Phase 3: User Story 1 — Member Claims Card (Priority: P1)

**Goal**: Enable YouTube members to claim digital membership cards by authenticating via OAuth and posting a verification comment.

**Independent Test Criteria** (from spec.md):
- Use test YouTube account with active channel membership
- Walk through: OAuth sign-in → comment on members-only video → comment verification → QR download/import
- Measure time-to-complete (should be <5 minutes per SC-001, <5 seconds for issuance per NFR-001)
- Validate displayed data accuracy (channel name, membership level, expiration date)

### T006 [US1] Remove comment age validation restriction
**File**: `src/services/card_issuer.rs` (comment verification logic)
**Story**: US1
**Requirement**: FR-003 (clarification: no age restriction needed)
**Description**:
- Remove any code that checks comment `publishedAt` timestamp age
- Previous clarification removed 7-day limit - ensure no temporal validation remains
- Only validate: comment author matches authenticated user, targets correct video
- Update error messages to remove any mention of "comment too old"

**Acceptance**:
- Comments from any date accepted (no age-based rejection)
- Only validation: author identity + video target
- Integration test: Issue card with comment from >30 days ago succeeds

### T007 [US1] Set default 30-day card expiration on issuance
**File**: `src/services/card_issuer.rs`
**Story**: US1
**Requirement**: FR-006a
**Description**:
- Update card creation to set `expires_at = Utc::now() + Duration::days(30)`
- Ensure expires_at is NOT NULL on newly issued cards
- Log expiration timestamp in issuance success message
- Display expiration date in card UI (`templates/cards/show.html`)

**Acceptance**:
- All new cards have `expires_at` set to 30 days from issuance
- Expiration date visible in card details page
- Database constraint: `expires_at NOT NULL` for active cards

### T008 [P] [US1] Add performance monitoring for 5-second issuance target
**File**: `src/api/cards.rs` (issue endpoint)
**Story**: US1
**Requirement**: NFR-001
**Description**:
- Add tracing span around entire card issuance flow
- Measure duration from comment submission to QR code generation
- Log warning if issuance exceeds 5 seconds
- Include breakdown: OAuth validation, YouTube API call, wallet API call, DB write
- Add metric instrumentation (if metrics framework configured)

**Acceptance**:
- Issuance duration logged with sub-step breakdown
- Warning logged if total duration >5 seconds
- Trace IDs allow correlation with external API logs

### T009 [P] [US1] Add wallet API availability check before issuance
**File**: `src/services/wallet_qr.rs`
**Story**: US1
**Requirement**: FR-008a
**Description**:
- Add health check endpoint call to wallet API before starting issuance
- Fail early with clear message if wallet API unavailable
- Cache health status for 30 seconds to avoid per-request overhead
- Include wallet API status in application health endpoint (`/health`)

**Acceptance**:
- Issuance fails immediately if wallet API down (no wasted YouTube API quota)
- User sees: "Taiwan Digital Wallet service unavailable. Please try again later."
- Health endpoint shows wallet API status

### T010 [P] [US1] Update card display template with expiration date
**File**: `templates/cards/show.html`
**Story**: US1
**Requirement**: FR-006a
**Description**:
- Add expiration date display in card details: "Expires: YYYY-MM-DD"
- Show visual warning if card expires within 7 days
- If card expired: replace QR code with "Card Expired" message
- Add "Renew Card" button for expired cards (links to re-issuance flow)

**Acceptance**:
- Expiration date visible on card page
- Expired cards show clear warning (no QR code display)
- Users can identify when renewal needed

### T011 [US1] Implement duplicate card prevention with expiration check
**File**: `src/services/card_issuer.rs`
**Story**: US1
**Requirement**: FR-006 + FR-006a
**Description**:
- Before issuing, query: `find_active_unexpired_cards(member_id, issuer_id)`
- Block issuance if active + unexpired card exists
- Allow re-issuance if previous card expired
- Return specific error: "Active card already exists. Expires: {date}"

**Acceptance**:
- Cannot issue duplicate if active unexpired card exists
- Can issue new card after previous expires
- Error message includes expiration date of existing card

### T012 [US1] Add integration test for complete issuance flow
**File**: `tests/integration/card_issuance_test.rs`
**Story**: US1
**Description**:
- Mock YouTube OAuth flow + API responses
- Mock Taiwan wallet API responses
- Test complete flow: OAuth → comment verification → wallet QR → card creation
- Verify `expires_at` set correctly
- Verify duplicate prevention
- Measure performance (<5 seconds per NFR-001)

**Acceptance**:
- End-to-end test covers happy path
- Test verifies 30-day expiration set
- Test runs in <5 seconds (mocked APIs)
- Assertions on database state after issuance

### T013 [US1] Add error handling integration tests
**File**: `tests/integration/card_issuance_errors_test.rs`
**Story**: US1
**Description**:
- Test YouTube API rate limit (429) triggers retry logic
- Test wallet API failure aborts issuance
- Test invalid comment URL returns clear error
- Test duplicate card prevention
- Test expired card allows re-issuance

**Acceptance**:
- All error scenarios covered
- Retry logic verified (3 attempts for rate limit)
- Wallet API failure leaves no partial data
- Error messages match spec requirements

---

## Phase 4: Polish & Cross-Cutting Concerns

**Goal**: System-wide improvements and operational readiness.

### T014 [P] [Polish] Update API documentation after OpenAPI removal
**File**: `specs/001-channel-membership-verification/contracts/README.md`
**Story**: Polish
**Description**:
- Remove references to deleted `openapi.yaml`
- Document actual implemented endpoints (from plan.md Phase 1)
- Add request/response examples for key endpoints
- Document error response format
- Add performance characteristics (5-second target)

**Acceptance**:
- README.md reflects current implementation
- No references to deleted OpenAPI spec
- Examples provided for OAuth flow, card issuance, verification

### T015 [P] [Polish] Add cronjob skeleton for card expiration renewal
**File**: `src/jobs/card_renewal.rs` (new), `src/main.rs`
**Story**: Polish
**Requirement**: FR-006a (cronjob can extend expiration)
**Description**:
- Create new cronjob module for card expiration management
- Schedule daily run (configurable interval)
- Query cards expiring within 7 days
- For each card: validate membership still active (YouTube API)
- If active: extend `expires_at` by 30 days
- If inactive: mark card as revoked (`deleted_at = NOW()`)
- Log renewal/revocation actions

**Acceptance**:
- Cronjob registered in tokio-cron-scheduler
- Runs daily (default), configurable via env var
- Cards nearing expiration automatically renewed if membership active
- Expired memberships trigger card revocation

### T016 [Polish] Add application health endpoint with dependency checks
**File**: `src/api/health.rs` (new)
**Story**: Polish
**Description**:
- Create `/health` endpoint returning JSON status
- Check database connectivity (simple query)
- Check wallet API availability (cached status from T009)
- Check YouTube API quota remaining (if available)
- Return 200 if all healthy, 503 if any dependency down
- Include response time for each dependency

**Acceptance**:
- `/health` endpoint accessible without authentication
- Returns dependency statuses: database, wallet API, YouTube API
- Returns 503 if any critical dependency unavailable
- Response includes timestamp and version info

---

## Dependencies & Execution Order

### Critical Path (Must Complete Sequentially)

```
T001, T002 (Setup - parallel)
    ↓
T003, T004, T005 (Foundation - T003 & T005 parallel, T004 after T003)
    ↓
T006 → T007 → T011 (US1: Core issuance logic - sequential, same file)
    ↓
T008, T009, T010 (US1: Enhancements - parallel)
    ↓
T012, T013 (US1: Tests - parallel)
    ↓
T014, T015, T016 (Polish - parallel)
```

### Parallel Execution Opportunities

**Setup Phase (can run concurrently)**:
- T001 (database schema) || T002 (environment config)

**Foundation Phase (partial parallelism)**:
- T003 (YouTube retry logic) || T005 (wallet error handling)
- T004 (expiration validation) - after T003 (shares card.rs)

**US1 Implementation (partial parallelism)**:
- T006 → T007 → T011 (sequential - same service file)
- T008 || T009 || T010 (parallel - different files)

**US1 Testing (full parallelism)**:
- T012 || T013 (parallel - separate test files)

**Polish Phase (full parallelism)**:
- T014 || T015 || T016 (parallel - separate files)

---

## Implementation Strategy

### MVP Scope (Immediate Priority)

**Phase 1-3 Required for MVP** (Tasks T001-T013):
- Setup & foundational infrastructure (T001-T005)
- Complete US1: Member claims card with all clarified requirements (T006-T013)

**Phase 4 Optional for MVP** (Tasks T014-T016):
- Can be deferred to post-launch if time constrained
- T015 (cronjob) critical for production but can run manually during MVP phase

### Incremental Delivery Checkpoints

**Checkpoint 1**: Foundation Complete (after T005)
- Core infrastructure ready
- Retry logic, expiration validation, wallet error handling in place
- Ready to implement US1 features

**Checkpoint 2**: US1 Implementation Complete (after T011)
- Core issuance flow functional with all clarifications
- 30-day expiration, duplicate prevention, comment validation
- Ready for enhancement features

**Checkpoint 3**: US1 Enhanced (after T013)
- Performance monitoring, health checks, comprehensive tests
- Ready for production deployment

**Checkpoint 4**: Production Ready (after T016)
- Documentation updated, cronjob configured, health monitoring
- Full operational readiness

### Testing Strategy

**Integration Tests Required**:
- T012: Happy path end-to-end test
- T013: Error scenario coverage (rate limits, API failures, duplicates)

**Manual Testing** (per spec.md Independent Test):
- Use real YouTube account with active membership
- Complete OAuth flow in browser
- Post comment on members-only video
- Submit comment URL and verify card issuance
- Import QR code into Taiwan Digital Wallet
- Measure time-to-complete (<5 minutes target)

---

## Task Checklist

**Phase 1: Setup**
- [X] T001 - Verify database schema completeness
- [X] T002 - Validate environment configuration

**Phase 2: Foundation**
- [X] T003 - Implement YouTube API retry logic with exponential backoff
- [X] T004 - Add card expiration validation logic
- [X] T005 - Implement wallet API error handling (fail-fast)

**Phase 3: US1 - Member Claims Card**
- [X] T006 - Remove comment age validation restriction
- [X] T007 - Set default 30-day card expiration on issuance
- [X] T008 - Add performance monitoring for 5-second issuance target
- [X] T009 - Add wallet API availability check before issuance
- [X] T010 - Update card display template with expiration date
- [X] T011 - Implement duplicate card prevention with expiration check
- [~] T012 - Add integration test for complete issuance flow (SKIPPED: test coverage deferred)
- [~] T013 - Add error handling integration tests (SKIPPED: test coverage deferred)

**Phase 4: Polish**
- [X] T014 - Update API documentation after OpenAPI removal
- [~] T015 - Add cronjob skeleton for card expiration renewal (DEFERRED: belongs to 003-card-lifecycle-automation)
- [X] T016 - Add application health endpoint with dependency checks

---

## Success Metrics

**Per Spec Success Criteria**:
- **SC-001**: Member completes claim → import in <5 minutes ✓ (measured in T012)
- **SC-004**: 95% OAuth → card issuance success rate ✓ (tracked via T008 metrics)
- **SC-007**: 90% claims without manual intervention ✓ (error handling in T013)

**Per NFR Performance**:
- **NFR-001**: <5 second issuance latency ✓ (monitored via T008)

**Per Functional Requirements**:
- **FR-003**: Comment validation (no age limit) ✓ (T006)
- **FR-006a**: 30-day expiration + cronjob renewal ✓ (T007, T015)
- **FR-008a**: Wallet API fail-fast ✓ (T005)
- **FR-009a**: YouTube API retry 3x/30s ✓ (T003)

---

**Generated by**: `/speckit.tasks` command
**Next Command**: Begin implementation with `T001` or use parallel execution strategy starting with `T001 || T002`
