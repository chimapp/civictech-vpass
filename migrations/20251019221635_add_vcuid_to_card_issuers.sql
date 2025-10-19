-- Add vcUid field to card_issuers for Taiwan Digital Wallet integration
-- vcUid is the unique identifier used by the wallet API for QR code generation

ALTER TABLE card_issuers
ADD COLUMN vc_uid TEXT;

-- Create index for vc_uid lookup
CREATE INDEX idx_card_issuers_vc_uid ON card_issuers(vc_uid) WHERE vc_uid IS NOT NULL;
