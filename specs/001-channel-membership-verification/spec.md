# Feature Specification: Channel Membership Verification Card System (MVP)

**Feature Branch**: `001-channel-membership-verification`  
**Created**: 2025-10-12  
**Status**: Draft  
**Scope**: Deliver the first member-facing slice that issues YouTube channel membership cards compatible with 數位皮夾.

## Overview

This MVP enables a YouTube channel member to authenticate with their Google account, have VPass confirm their active membership by validating their ability to comment on a members-only video, and receive a digitally signed QR card that can be imported into 數位皮夾. Organizer verification tooling, automated lifecycle management, and multi-platform support are intentionally deferred to follow-up specs so this slice stays shippable within a single iteration.

## User Scenarios & Testing *(mandatory)*

### User Story 1 — YouTube Member Claims a Card (Priority: P1)

- **Happy Path**: A user with an active YouTube channel membership signs in through OAuth, consents to required scopes, posts a new comment on the issuer’s members-only verification video, submits the comment link, VPass validates the comment, and the user receives a QR code that imports to 數位皮夾 with accurate channel information.
- **Failure Handling**: If the comment cannot be located (no active membership, incorrect video, deleted comment, or API error), the user receives a clear error message and the issuance is aborted.

**Independent Test**: Use a test YouTube account with an active channel membership. Walk through the flow end-to-end: OAuth sign-in → comment on members-only video → comment verification → QR download/import into 數位皮夾. Record time-to-complete and validation of displayed data.

### Future User Stories (Tracked Separately)

The following capabilities move to later specs:
- Organizer QR scanning and verification workflow (`002-card-verification`).
- Automatic revocation and refresh in response to membership changes (`003-card-lifecycle-automation`).
- Twitch or community issuer support (`00X-multi-platform`).

## Functional Requirements (Extended MVP)

### Core Issuance (Original MVP Scope)

- **FR-001**: Members MUST authenticate with YouTube OAuth before claiming a card.
- **FR-002**: The system MUST validate membership status by confirming that the authenticated member has an active comment on the issuer's designated members-only verification video.
- **FR-003**: The system MUST ensure the verified comment belongs to the authenticated member and targets the designated verification video (no age restriction on comments).
- **FR-004**: Issued cards MUST include issuer channel name, an issuer-defined membership level label, membership confirmation timestamp, member display name, a signed QR payload, and metadata required by 數位皮夾.
- **FR-005**: The system MUST persist card issuance records for auditing and future verification.
- **FR-006**: Duplicate active cards for the same member/channel pair MUST be prevented.
- **FR-006a**: Issued cards MUST include an expires_at timestamp set to 30 days from issuance; background jobs can extend expiration or revoke cards based on ongoing membership validation.
- **FR-007**: OAuth access and refresh tokens MUST be stored encrypted at rest and never logged in plaintext.
- **FR-008**: The UI MUST offer QR download/import options compatible with 數位皮夾 expectations.
- **FR-008a**: If the Taiwan Digital Wallet (數位皮夾) API is unavailable during card issuance, the entire issuance MUST fail with an actionable error message instructing the user to retry later.
- **FR-009**: The system MUST return actionable, localized error messages when issuance fails.
- **FR-009a**: When YouTube API rate limits are exceeded, the system MUST retry with exponential backoff (maximum 3 attempts over 30 seconds) before failing.
- **FR-010**: The system MUST store the verified comment ID and verification timestamp to support auditing and detect replay or deletion.
- **FR-011**: Issuers MUST be able to configure the members-only verification video ID used for comment validation.

### Event Management & Verification (Implemented Ahead of Schedule)

> **Note**: The following requirements were implemented alongside the core MVP to enable end-to-end verification workflows. Originally planned for spec-002, these features are documented here to reflect actual system capabilities.

- **FR-012**: Organizers MUST be able to create verification events with name, date, location, and required issuer association.
- **FR-013**: The system MUST support OIDVP (OpenID for Verifiable Presentations) protocol for card verification.
- **FR-014**: Organizers MUST be able to generate verification QR codes via external OIDVP verifier API.
- **FR-015**: The system MUST poll external verifier for verification results using transaction IDs (session state managed in frontend).
- **FR-016**: Successful verification results MUST be persisted in audit log with timestamp, member info, and event association.
- **FR-017**: Organizers MUST be able to view verification history for their events.
- **FR-018**: Event statistics MUST be available (total verifications, unique members, time distribution).
- **FR-019**: The system MUST integrate with Taiwan digital wallet (數位皮夾) for credential presentation via OIDVP.
- **FR-020**: Wallet QR codes and credential offer data MUST be stored on membership cards for wallet import functionality.
- **FR-021**: Card revocation MUST be supported with soft-delete semantics (deleted_at timestamp).
- **FR-022**: External OIDVP verifier API MUST be configurable via VERIFIER_API_URL environment variable.

## Non-Functional Requirements

### Performance

- **NFR-001**: Card issuance flow (comment submission to QR code generation) MUST complete within 5 seconds under normal conditions (accounts for YouTube API and wallet API latency).

## Key Entities (Extended)

### Original MVP Entities

- **CardIssuer**: Represents a YouTube channel configured inside VPass. Stores channel metadata, verification video ID, default membership label, and whether YouTube ownership has been verified.
- **OAuthSession**: Captures the member's authenticated session, encrypted tokens, scopes, and expiry.
- **MembershipCard**: Immutable record of an issued card including member identifiers, membership attributes, QR payload, and signature.
- **MemberProfile**: Cached snapshot of the member's display name and relevant metadata captured during issuance (optional but keeps API round-trips low).

### Additional Entities (Implemented)

- **Event**: Verification event organized by channel owners. Stores event metadata (name, date, location), issuer requirements, and activation status. Enables organizers to manage multiple verification sessions.
- **VerificationEvent**: Audit log of successful verification attempts. Records verification success, member info, event association, and verification metadata for compliance and analytics.
- **MembershipCard (Extended)**: Includes Taiwan digital wallet (數位皮夾) integration fields: QR data, OIDVP credential offer URLs, deep links, credential IDs (cid), and scan timestamps. Simplified from previous separate wallet_qr_codes table (merged 2025-11-10).
- **Revocation**: Card revocation tracking implemented via soft-delete semantics (deleted_at timestamp on membership_cards). Enables auditable card lifecycle management.

**Architectural Note**: Verification session state (transaction IDs, pending status, timeouts) is managed in frontend JavaScript rather than database storage, reducing unnecessary writes for ephemeral state. Only successful verifications are persisted in VerificationEvent audit log.

## Success Criteria (MVP)

- **SC-001**: A member can finish the claim → import journey in under 5 minutes under nominal conditions.
- **SC-004**: At least 95% of successful OAuth authorizations that submit a valid members-only comment link result in a card being issued.
- **SC-007**: At least 90% of card claims finish without manual support intervention.

Metrics tied to organizer verification, revocation, or refresh cycles (previously SC-002, SC-003, SC-005, SC-006, SC-008) will be reinstated in their future specs.

## Assumptions

- 數位皮夾 already supports the documented QR import format.
- YouTube OAuth client credentials are available and reviewed for the required scopes.
- Channel organizers maintain at least one members-only verification video and moderate comments to keep the flow trustworthy.
- Members have internet-connected devices capable of importing QR codes into 數位皮夾.
- Membership checks only happen on user demand; ongoing polling is out of scope.

## Dependencies

- Google OAuth 2.0 client (client ID/secret, redirect URI approval).
- YouTube Data API quota for retrieving members-only video comments (`commentThreads.list` or `comments.list`).
- QR generation library that can emit payloads accepted by 數位皮夾.
- Secure secret storage (environment variables for development, infrastructure-managed secrets for production).

## Out of Scope

- Organizer-facing verification interfaces or history logs.
- Scheduled jobs for revocation or membership refresh.
- Twitch, Discord, or community issuer support.
- Bulk issuance, analytics dashboards, or wallet design customization.
- Offline verification or printable credentials.

## Clarifications

### Session 2025-11-10

- Q: What is the maximum allowed age for a comment to be considered valid for membership verification (防止舊留言重複使用)? → A: No age restriction needed
- Q: Should membership cards include an expires_at timestamp field, or rely solely on cronjob revocation when membership lapses? → A: Include expires_at field (30 days default, cronjob can extend/revoke)
- Q: What is the acceptable maximum response time for the card issuance flow (from comment submission to QR code generation)? → A: 5 seconds (reasonable for multi-API flow)
- Q: When YouTube API rate limits are exceeded during card issuance, what should the system do? → A: Retry with exponential backoff (max 3 attempts over 30s)
- Q: When the Taiwan Digital Wallet (數位皮夾) API is unavailable during card issuance, what should happen? → A: Fail entire issuance (user must retry later)

## Open Questions

### Resolved

1. ~~Which verification metadata must be preserved in the QR payload to satisfy 數位皮夾~~
   **RESOLVED**: QR payload includes card_id, issuer_id, member_id, membership_level_label, issued_at, and HMAC-SHA256 signature. Additional wallet integration uses OIDVP credential format with credential offer URLs stored directly on membership_cards table (wallet_transaction_id, wallet_qr_code, wallet_deep_link, wallet_cid, wallet_scanned_at fields).

2. ~~Are localized strings for issuance success/failure required in this slice~~
   **RESOLVED - DEFERRED**: English-only for extended MVP. Localization infrastructure and translated message catalogs planned for future iteration post-launch.

3. ~~What minimum logging/observability requirements does the platform team expect~~
   **RESOLVED**: Structured logging via `tracing` with request IDs, health endpoint for database connectivity checks, metrics instrumentation for issuance/verification flows covering success rates and latency percentiles.
