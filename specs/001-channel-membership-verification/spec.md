# Feature Specification: Channel Membership Verification Card System

**Feature Branch**: `001-channel-membership-verification`
**Created**: 2025-10-12
**Status**: Draft
**Input**: User description: "Channel membership verification card system"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Channel Member Claims Membership Card (Priority: P1)

A channel member (e.g., YouTube/Twitch subscriber) wants to obtain a digital membership card that proves their subscription status. They visit VPass, authenticate using their YouTube or Twitch account via OAuth, and the system automatically verifies their membership status through platform APIs. Upon successful verification, they receive a QR code that can be imported into their 數位皮夾 (digital wallet).

**Why this priority**: This is the core value proposition - enabling members to claim verifiable proof of their channel membership. Without this, no other functionality is possible.

**Independent Test**: Can be fully tested by having a test user claim a card for a real channel membership and successfully import it into 數位皮夾. Delivers immediate value by providing a digital credential.

**Acceptance Scenarios**:

1. **Given** a user is a valid YouTube channel member, **When** they visit VPass and initiate card claiming, **Then** they are prompted to authenticate via YouTube OAuth
2. **Given** a user has authenticated via platform OAuth, **When** the system retrieves their membership data from the platform API, **Then** the system verifies their active membership status and subscription details
3. **Given** membership verification succeeds, **When** the user optionally provides supplementary information, **Then** the system generates a QR code containing the complete membership card data
4. **Given** a user receives a valid QR code, **When** they scan it with 數位皮夾, **Then** the membership card is successfully added to their digital wallet
5. **Given** a user's membership verification fails (not subscribed, subscription expired), **When** the system detects invalid membership status, **Then** the user receives clear error messages explaining the issue

---

### User Story 2 - Event Organizer Verifies Membership Card (Priority: P2)

An event organizer wants to verify that attendees are legitimate channel members. They open VPass verification page, specify which channel issuer's cards they want to verify, start a QR code scanner, and scan cards presented by attendees from their 數位皮夾 to confirm membership status.

**Why this priority**: This enables the primary use case for issued cards - offline/online event access control. Without verification capability, cards have limited practical value.

**Independent Test**: Can be tested by having an event organizer scan test cards and verify that valid cards are accepted while invalid/expired cards are rejected. Delivers value by enabling access control.

**Acceptance Scenarios**:

1. **Given** an event organizer opens VPass verification page, **When** they authenticate their YouTube or Twitch channel account through platform OAuth, **Then** the system verifies their channel ownership and enables card verification for that channel
2. **Given** an organizer has authenticated their channel, **When** they access the verification interface, **Then** the system initializes a QR scanner for cards issued by their verified channel
3. **Given** the scanner is active, **When** an attendee presents a valid membership card QR code from 數位皮夾, **Then** the system displays verification success with card details (membership level, duration, status)
4. **Given** the scanner is active, **When** an attendee presents an expired or revoked card, **Then** the system displays verification failure with appropriate reason
5. **Given** verification is complete, **When** the organizer needs to verify another attendee, **Then** they can immediately scan the next QR code without reinitialization

---

### User Story 3 - Card Auto-Revocation on Subscription Cancellation (Priority: P3)

When a channel member cancels their subscription, the system automatically detects this change and revokes their membership card so it no longer passes verification checks.

**Why this priority**: Ensures card validity reflects current membership status. Important for security but can be implemented after basic issuance/verification flows work.

**Independent Test**: Can be tested by canceling a test subscription and verifying that the previously issued card fails verification. Delivers value by maintaining card integrity.

**Acceptance Scenarios**:

1. **Given** a member has an active membership card, **When** they cancel their channel subscription, **Then** the system detects the cancellation and marks the card as revoked
2. **Given** a card has been revoked, **When** an event organizer scans the revoked card, **Then** verification fails with status indicating the card is no longer valid
3. **Given** a member re-subscribes after cancellation, **When** they claim a new card, **Then** they receive a fresh card while the old revoked card remains invalid

---

### User Story 4 - Card Refresh on Membership Changes (Priority: P3)

When a channel member's subscription details change (e.g., membership level upgrade, badge changes, duration milestones), the system revokes the old card and issues a new card with updated information. Members authenticate through VPass using their YouTube or Twitch account to verify their updated membership status and claim the refreshed card.

**Why this priority**: Keeps cards current with actual membership status. Lower priority as it's an enhancement to keep data fresh rather than core functionality.

**Independent Test**: Can be tested by changing membership level and verifying that a new card reflects updated information. Delivers value by ensuring cards show current status.

**Acceptance Scenarios**:

1. **Given** a member's subscription changes (level, duration milestone, badge update), **When** the system detects the change through periodic platform API checks, **Then** the old card is revoked and marked for refresh
2. **Given** a card has been marked for refresh, **When** the member authenticates via VPass using their YouTube or Twitch OAuth, **Then** the system verifies their current membership status and generates a new card
3. **Given** the system generates a new card, **When** the member receives the QR code, **Then** they can import it to 數位皮夾 and the new card replaces or coexists with the old card (depending on 數位皮夾 behavior)
4. **Given** a member has not re-authenticated after membership changes, **When** an event organizer scans their old card, **Then** verification shows the card is outdated and prompts the member to refresh via VPass

---

### Edge Cases

- What happens when a member tries to claim a card but their subscription status cannot be verified (e.g., platform API unavailable)?
- How does the system handle members who have subscriptions on multiple platforms (YouTube and Twitch) for the same channel?
- What happens if a member tries to claim multiple cards simultaneously?
- How does verification work when the event organizer's device loses internet connectivity?
- What happens when a card issuer's channel is deleted or suspended by the platform?
- How does the system handle time zone differences for subscription duration calculations?
- What happens if 數位皮夾 is unavailable or changes its QR code format?

## Requirements *(mandatory)*

### Functional Requirements

**Authentication & Authorization**:
- **FR-001**: System MUST allow channel members to authenticate using YouTube OAuth
- **FR-002**: System MUST allow channel members to authenticate using Twitch OAuth
- **FR-003**: System MUST allow event organizers to authenticate as channel owners using YouTube OAuth
- **FR-004**: System MUST allow event organizers to authenticate as channel owners using Twitch OAuth
- **FR-005**: System MUST verify channel ownership through authenticated platform APIs

**Card Claiming Flow**:
- **FR-006**: System MUST retrieve membership status directly from platform APIs after user authenticates
- **FR-007**: System MUST verify membership validity (active subscription, membership level, duration) through platform API responses
- **FR-008**: System MUST generate QR code containing membership card data upon successful verification
- **FR-009**: QR code MUST be compatible with 數位皮夾 import format and standards
- **FR-010**: System MUST allow members to include user-defined supplementary fields during card claiming

**Card Content**:
- **FR-011**: Membership card MUST contain membership level information
- **FR-012**: Membership card MUST contain subscription duration or start date
- **FR-013**: Membership card MUST indicate whether member is active (based on recent engagement metrics like comments, chat participation)
- **FR-014**: Membership card MUST contain high-value supporter indicators (superchats count, gift subscriptions, platform-specific engagement metrics)
- **FR-015**: Membership card MUST include unique card identifier and issuer information

**Card Verification Flow**:
- **FR-016**: System MUST provide verification interface for authenticated event organizers to scan membership cards
- **FR-017**: System MUST restrict verification to cards issued by the organizer's authenticated channel
- **FR-018**: System MUST validate scanned QR codes against issued card database
- **FR-019**: System MUST display verification result showing card validity and key membership details
- **FR-020**: System MUST check card revocation status during verification

**Card Lifecycle Management**:
- **FR-021**: System MUST periodically check subscription status through platform APIs to detect cancellations
- **FR-022**: System MUST automatically revoke cards when associated subscriptions are canceled
- **FR-023**: System MUST revoke old cards when membership information changes (level, badge, card design)
- **FR-024**: System MUST mark cards for refresh when membership information changes
- **FR-025**: System MUST allow members to re-authenticate and claim refreshed cards with updated information
- **FR-026**: System MUST maintain revocation history for audit purposes

**Multi-Platform Support**:
- **FR-027**: System MUST support YouTube channel memberships as a card issuer platform
- **FR-028**: System MUST support Twitch channel subscriptions as a card issuer platform
- **FR-029**: System MUST support non-official/semi-official community-issued cards with appropriate distinction from official channel cards

**Security & Data Protection**:
- **FR-030**: System MUST securely store OAuth tokens according to platform requirements
- **FR-031**: System MUST request minimum necessary permissions from platform OAuth scopes
- **FR-032**: System MUST support multiple card issuers (channel owners, community organizers)
- **FR-033**: System MUST prevent duplicate card issuance for the same membership at the same time
- **FR-034**: System MUST provide error messages when verification fails, indicating the reason

### Key Entities

- **Card Issuer**: Represents the authority that issues membership cards. Can be an official channel owner (YouTube/Twitch) or a non-official/semi-official community. Has unique identifier, issuer type, associated platform channel ID, and OAuth authentication status.

- **Channel Member**: Represents a person who holds a subscription or membership to a channel. Has platform-specific user identifiers (YouTube user ID, Twitch user ID), OAuth authentication tokens, membership start date, membership level, engagement metrics, and support history.

- **Membership Card**: Digital credential proving channel membership. Contains unique card ID, issuer reference, member platform identifier, issuance timestamp, membership level, subscription duration, activity status, supporter metrics, revocation status, and refresh status.

- **Platform Authentication Session**: OAuth authentication session linking VPass users to their platform accounts. Contains platform type (YouTube/Twitch), platform user ID, OAuth tokens, token expiration, and session creation timestamp.

- **Verification Event**: Record of a card verification attempt by an event organizer. Contains verification timestamp, scanned card reference, organizer's authenticated channel reference, verification result, and context information.

- **Card Revocation**: Record of a card being invalidated. Contains revocation timestamp, reason (subscription canceled, card updated, manual revocation), old card reference, new card reference (if applicable), and refresh flag.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Channel members can complete card claiming process from proof submission to 數位皮夾 import in under 5 minutes for standard cases
- **SC-002**: Card verification by event organizers completes within 3 seconds per scan under normal conditions
- **SC-003**: System automatically revokes cards within 24 hours of subscription cancellation detection
- **SC-004**: 95% of valid proof submissions result in successful card issuance
- **SC-005**: Card verification accuracy reaches 99.5% (correct acceptance of valid cards and rejection of invalid/revoked cards)
- **SC-006**: System supports simultaneous verification by at least 10 event organizers without performance degradation
- **SC-007**: 90% of card claiming attempts succeed on first try without requiring support intervention
- **SC-008**: Card re-issuance on membership changes completes within 1 hour of change detection

## Assumptions

- **AS-001**: 數位皮夾 (digital wallet) already exists and has a documented QR code import format
- **AS-002**: YouTube and Twitch provide OAuth 2.0 authentication endpoints for user authorization
- **AS-003**: YouTube and Twitch APIs provide endpoints to retrieve membership/subscription data for authenticated users
- **AS-004**: Platform APIs reliably return membership status, level, duration, and engagement metrics
- **AS-005**: Event organizers have devices capable of scanning QR codes (smartphones, tablets)
- **AS-006**: Members have access to 數位皮夾 on their mobile devices
- **AS-007**: OAuth tokens from platforms have sufficient validity period for periodic membership status checks
- **AS-008**: Internet connectivity is required for both card claiming and verification processes
- **AS-009**: Channel owners can authenticate as themselves through platform OAuth to enable verification
- **AS-010**: Platform rate limits for API calls are sufficient for periodic subscription status checks

## Dependencies

- **DEP-001**: 數位皮夾 must support QR code import functionality
- **DEP-002**: YouTube OAuth 2.0 API access (requires OAuth client ID/secret registration)
- **DEP-003**: YouTube Data API access for membership verification (requires API keys and compliance with usage limits)
- **DEP-004**: Twitch OAuth 2.0 API access (requires OAuth client ID/secret registration)
- **DEP-005**: Twitch API access for subscription verification (requires API keys and compliance with usage limits)
- **DEP-006**: QR code generation library compatible with 數位皮夾 format requirements
- **DEP-007**: Coordination with 數位皮夾 team for card re-issuance UX improvements (mentioned in user-flows.md)
- **DEP-008**: Documentation of OAuth scopes required for YouTube membership data access
- **DEP-009**: Documentation of OAuth scopes required for Twitch subscription data access

## Out of Scope

- **OOS-001**: Integration with third-party platforms beyond YouTube and Twitch for the initial version
- **OOS-002**: Automated integration with Discord, LINE, or other community platforms (noted as future use case)
- **OOS-003**: Physical card printing or generation
- **OOS-004**: Payment processing for card issuance fees (assumed to be free initially)
- **OOS-005**: Advanced analytics dashboard for card issuers to track card distribution and usage
- **OOS-006**: Customizable card design templates for issuers (using default design initially)
- **OOS-007**: Bulk card issuance for multiple members simultaneously
- **OOS-008**: Offline verification mode (verification requires internet connectivity)
