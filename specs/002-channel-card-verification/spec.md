# Feature Specification: Channel Card Verification Experience

**Feature Branch**: `002-channel-card-verification`  
**Created**: 2025-10-12  
**Status**: Draft  
**Scope Linkage**: Builds on `001-channel-membership-verification` (YouTube card issuance MVP). Focuses on organizer-facing verification flows; does not modify issuance logic beyond what is necessary for verification.

## User Scenarios & Testing *(mandatory)*

### User Story 2 — Organizer Verifies Member Cards (Priority: P1)

An event organizer who owns a YouTube channel wants to validate attendee cards at the door. They authenticate as a channel owner, launch the verification interface, scan attendee QR codes, and receive immediate pass/fail feedback.

**Acceptance Scenarios**:
1. **Given** an organizer authenticates via YouTube OAuth, **When** VPass confirms channel ownership, **Then** the verification interface loads with the organizer’s issuer preselected.
2. **Given** the QR scanner is running, **When** the organizer scans a valid membership card, **Then** VPass confirms the card belongs to their channel and displays success with membership details.
3. **Given** the QR scanner is running, **When** the organizer scans an invalid or outdated card, **Then** VPass displays a failure state explaining the reason (revoked, wrong issuer, signature mismatch).
4. **Given** the organizer is mid-event, **When** they scan multiple cards in sequence, **Then** the scanner stays active without reloading or reauthenticating.
5. **Given** verification completes, **When** the organizer reviews history, **Then** they can see recent scans (timestamp, result, card/member info) for auditing.

**Independent Test**: Use a staging issuer with multiple test cards (valid, revoked, wrong issuer). Ensure QR scanner (desktop + mobile browser) successfully differentiates each scenario and logs results.

### User Story 2a — Organizer Manually Revokes a Card (Priority: P2)

Organizers occasionally need to manually revoke a card (e.g., abuse report). They should be able to search for a card and revoke it, marking the reason.

**Acceptance Scenarios**:
1. **Given** an organizer is authenticated, **When** they search for a member/card from their issuer, **Then** they can view card status and revoke history.
2. **Given** the organizer chooses to revoke, **When** they confirm action with a reason, **Then** the card is marked revoked, logged with a manual reason, and the member receives a message on next login.
3. **Given** a revoked card exists, **When** the organizer scans it, **Then** the scanner shows the manual revocation reason.

**Independent Test**: Revoke a card via the organizer UI; verify the card fails future scans and appears in revocation logs.

---

## Success Criteria

- **SC-201**: Organizer can authenticate and start scanning within 2 minutes.
- **SC-202**: Verification result latency is < 3 seconds per scan under typical network conditions.
- **SC-203**: At least 99% of forged or wrong-issuer cards are rejected.
- **SC-204**: Manual revocations propagate within 60 seconds of submission.
- **SC-205**: Verification logs retain at least 30 days of history per issuer.

## Functional Requirements

- **FR-201**: Support organizer authentication via YouTube OAuth with ownership validation.
- **FR-202**: Provide web-based QR scanner UI (desktop + mobile browsers).
- **FR-203**: Validate QR payload signatures server-side and confirm issuer ownership.
- **FR-204**: Surface card/member details (tier, issued date) on successful verification.
- **FR-205**: Record each verification event with timestamp, result, and operator metadata.
- **FR-206**: Offer manual revocation workflow limited to issuer-owned cards.
- **FR-207**: Serve verification APIs that respond within the defined latency budgets.
- **FR-208**: Present localized error messaging for scan failures.
- **FR-209**: Rate-limit verification requests to mitigate abuse.

## Assumptions

- Organizer already has cards issued through the MVP system.
- Organizer devices support modern browsers with camera access (WebRTC compliant).
- QR codes continue to use the payload/signature format defined in 001.
- Legal/privacy review approves storing verification logs for at least 30 days.

## Dependencies

- Issuer verification endpoints from YouTube API (channels.list with ownership scope).
- JavaScript QR scanning library (e.g., html5-qrcode) licensed for commercial use.
- Existing database schema extended with verification/revocation tables.
- Auth middleware capable of differentiating organizer vs. member roles.

## Out of Scope

- Offline verification or native mobile apps.
- Batch verification imports.
- Automated revocation/refresh logic (covered by spec 003).
- Support for non-YouTube issuers.

## Open Questions

1. Should verification logs include personally identifiable information beyond display name? (Compliance review.)
2. What notification method should inform members about manual revocations in this phase (email vs. in-app)?
3. Do organizers need exportable CSV logs in this phase or is UI-only acceptable?
