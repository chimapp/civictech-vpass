# User flows

## Actors

1. Card issuer
   - Channel owner (direct owner of Youtube / Twitch owner)
   - Non-official / semi-official community
2. Channel member
3. Event organizer

## Channel member claims a card

1. Channel member visits our VPass
2. Channel member does certain actions according to platform-specific instructions to obtain a proof of membership (e.g. in Youtube comment on a specific member-only post)
3. Channel member submits proof of membership with other user defined fields to the web
4. VPass verifies the proof and issues the membership card as a QR code
5. Channel member scans the QR code with 數位皮夾 to obtain the card

## Event organizer verifies a card

1. Event organizer opens a page on VPass to verify card with certain issuer (what info Event organizer needs and how to obtain them?)
2. Event organizer starts a QR code scanner on VPass
3. Channel member shows QR code of membership card from 數位皮夾
4. Event organizer scans the QR code and checks result of the scan

---

## Card Features

- Membership level
- Subscription duration / since (15 個月訂閱！)
- Active member? (comments on Chat regularly / regular activities in the channel)
- 石油王 (Number of superchats / 小奇點 / 贈訂?)

### System Features

- Auto revocation of card if user cancels subscriptions
- Card info changes would requires revocation of old card and issue a card (e.g. membership level changes, member badge changes, 卡面 changes)
  - UX problem: How to notify Channel member to get the new card?
  - 數位皮夾 feature request: Support this flow in UI and model re-issue of the same card

## Other Card Use Cases

1. Integration with other platforms (e.g. Member-only discord, LINE, or etc)
