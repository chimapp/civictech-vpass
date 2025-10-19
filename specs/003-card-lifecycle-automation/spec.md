# Feature Specification: Membership Card Lifecycle Automation

**Feature Branch**: `003-card-lifecycle-automation`  
**Created**: 2025-10-12  
**Status**: Draft  
**Scope Linkage**: Extends `001-channel-membership-verification` (member issuance) and `002-channel-card-verification` (organizer tooling) by adding automated revocation and refresh capabilities for issued cards.

## User Scenarios & Testing *(mandatory)*

### User Story 3 — Automatic Revocation on Subscription Cancellation (Priority: P1)

As a channel issuer, I want cards to stop working automatically when memberships lapse, so that only active supporters retain access.

**Acceptance Scenarios**:
1. **Given** a member cancels their YouTube membership, **When** the scheduled lifecycle job runs, **Then** the system detects the cancellation and marks the member’s card as revoked.
2. **Given** a card is revoked automatically, **When** an organizer scans it, **Then** they see a failure with “subscription canceled” context.
3. **Given** a revocation occurs, **When** the member next logs in, **Then** they see messaging explaining the card is inactive and how to regain access.

**Independent Test**: Configure a test account to cancel a membership, trigger the lifecycle job (manual run), and verify the card status flips to revoked within SLA and fails scans.

### User Story 4 — Card Refresh on Membership Changes (Priority: P2)

Members should get updated cards when their membership level or perks change (e.g., upgraded tier). Cards should refresh automatically to reflect new data.

**Acceptance Scenarios**:
1. **Given** a member’s membership level changes, **When** the lifecycle job detects the update, **Then** the member’s existing card is marked “needs refresh” and a new card is issued.
2. **Given** a card is flagged “needs refresh,” **When** the member reauthenticates, **Then** they receive the refreshed card and the old card is retired.
3. **Given** an organizer scans an outdated card, **Then** the verification UI indicates the card is outdated and prompts for refresh.

**Independent Test**: Simulate a membership level upgrade in sandbox data, run the job, observe issuance of a replacement card, and validate the old card is flagged appropriately.

### User Story 3a — Lifecycle Operations Dashboard (Priority: P3)

Operational teams need visibility into job runs (stats, errors) and must be able to trigger replays when issues occur.

**Acceptance Scenarios**:
1. **Given** an operator opens the lifecycle dashboard, **When** they view job history, **Then** they see runs with duration, cards processed, revocations, refreshes, and errors.
2. **Given** a job fails mid-run, **When** the operator triggers a manual rerun, **Then** the system reprocesses outstanding cards without duplicating work.
3. **Given** the operator exports logs, **When** they download the report, **Then** it contains sufficient detail for audit (card id, user id, reason, timestamp).

**Independent Test**: Force an API outage, capture failed job metrics, trigger manual replay, and confirm the job completes successfully once connectivity is restored.

---

## Success Criteria

- **SC-301**: 95% of subscription cancellations result in revoked cards within 24 hours.
- **SC-302**: Membership upgrades trigger refreshed cards within 2 hours of detection.
- **SC-303**: Lifecycle job completion success rate ≥ 99% month over month.
- **SC-304**: Lifecycle dashboard exposes at least 90 days of job history with replay controls.
- **SC-305**: Organizer verification accuracy remains ≥ 99.5% post-automation.

## Functional Requirements

- **FR-301**: Implement scheduled job (cron or worker) to poll YouTube memberships for all active cards.
- **FR-302**: Detect status transitions (active → canceled) and flag cards for revocation automatically.
- **FR-303**: Detect membership attribute changes (tier, perks) and flag cards for refresh/reissue.
- **FR-304**: Persist lifecycle job runs with metrics, errors, and durations.
- **FR-305**: Provide admin interface/API to trigger ad-hoc job runs and view history.
- **FR-306**: Notify affected members (email or in-app messaging) when cards are revoked or refreshed.
- **FR-307**: Ensure job execution respects API rate limits and exponential backoff policies.
- **FR-308**: Guarantee idempotency—repeated runs should not double-issue or double-revoke cards.
- **FR-309**: Update verification logic to handle `needs_refresh` state distinctly from `revoked`.

## Assumptions

- YouTube APIs provide consistent membership data with change timestamps or equivalent.
- Infrastructure can schedule recurring jobs (cron scheduler, worker queue, or managed scheduler).
- Compliance approves storing job logs with member references.
- Notification channel (email, push, or in-app) is available; if not, the spec will define minimum messaging requirements on next member login.

## Dependencies

- Existing issuance + verification services from specs 001 and 002.
- Background job framework (e.g., `tokio-cron-scheduler`, external worker, or managed platform scheduler).
- Secret management process for storing service accounts/refresh tokens used by background job.
- Monitoring/alerting stack to capture job failures and API quota exhaustion.

## Out of Scope

- Support for platforms other than YouTube (handled in later multi-platform spec).
- Full member notification strategy across email/SMS (if not already available).
- Organizer-facing UI changes beyond indicating `needs_refresh` or `revoked` states.
- Real-time revocation (push-based) — polling cadence defines SLA.

## Open Questions

1. Should the lifecycle job run in the main web service or a dedicated worker deployment?
2. What is the preferred notification channel for automated revocations if email infrastructure is not yet integrated?
3. Do we need configurable schedules per issuer, or is a global cadence sufficient?
