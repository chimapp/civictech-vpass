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

## Functional Requirements (MVP)

- **FR-001**: Members MUST authenticate with YouTube OAuth before claiming a card.
- **FR-002**: The system MUST validate membership status by confirming that the authenticated member has an active comment on the issuer’s designated members-only verification video.
- **FR-003**: The system MUST ensure the verified comment belongs to the authenticated member, targets the designated verification video, and was posted after the current claim session began.
- **FR-004**: Issued cards MUST include issuer channel name, an issuer-defined membership level label, membership confirmation timestamp, member display name, a signed QR payload, and metadata required by 數位皮夾.
- **FR-005**: The system MUST persist card issuance records for auditing and future verification.
- **FR-006**: Duplicate active cards for the same member/channel pair MUST be prevented.
- **FR-007**: OAuth access and refresh tokens MUST be stored encrypted at rest and never logged in plaintext.
- **FR-008**: The UI MUST offer QR download/import options compatible with 數位皮夾 expectations.
- **FR-009**: The system MUST return actionable, localized error messages when issuance fails.
- **FR-010**: The system MUST store the verified comment ID and verification timestamp to support auditing and detect replay or deletion.
- **FR-011**: Issuers MUST be able to configure the members-only verification video ID used for comment validation.

## Key Entities (MVP)

- **CardIssuer**: Represents a YouTube channel configured inside VPass. Stores channel metadata, verification video ID, default membership label, and whether YouTube ownership has been verified.
- **OAuthSession**: Captures the member’s authenticated session, encrypted tokens, scopes, and expiry.
- **MembershipCard**: Immutable record of an issued card including member identifiers, membership attributes, QR payload, and signature.
- **MemberProfile**: Cached snapshot of the member’s display name and relevant metadata captured during issuance (optional but keeps API round-trips low).

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

## Open Questions

1. Which verification metadata (comment ID, channel member handle, confirmation timestamp) must be preserved in the QR payload to satisfy 數位皮夾 (exact schema confirmation pending)?
2. Are localized strings for issuance success/failure required in this slice, or can English-only copy ship initially?
3. What minimum logging/observability requirements does the platform team expect before exposing this flow to real members?
