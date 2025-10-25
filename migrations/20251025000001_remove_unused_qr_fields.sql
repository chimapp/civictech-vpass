-- Remove unused internal QR signature fields
-- These fields were never used - the system only uses Taiwan Digital Wallet QR codes
-- stored in the wallet_qr_codes table.

ALTER TABLE membership_cards
DROP COLUMN qr_payload,
DROP COLUMN qr_signature;
