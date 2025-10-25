-- Add card status and expiration tracking for automated lifecycle management
-- Part of Spec 003: Card Lifecycle Automation

-- Create card status enum
CREATE TYPE card_status AS ENUM ('active', 'expired', 'revoked', 'suspended');

-- Drop the old is_primary boolean field (replaced by status)
ALTER TABLE membership_cards DROP COLUMN is_primary;

-- Add new status and expiration fields
ALTER TABLE membership_cards
  ADD COLUMN status card_status DEFAULT 'active' NOT NULL,
  ADD COLUMN expires_at TIMESTAMPTZ,
  ADD COLUMN last_verified_at TIMESTAMPTZ,
  ADD COLUMN verification_failures INT DEFAULT 0 NOT NULL;

-- Set initial expires_at for existing cards (30 days from issued_at)
UPDATE membership_cards
SET expires_at = issued_at + INTERVAL '30 days'
WHERE expires_at IS NULL;

-- Create indexes for background job queries
CREATE INDEX idx_cards_status_expires ON membership_cards(status, expires_at);
CREATE INDEX idx_cards_next_verification ON membership_cards(last_verified_at)
  WHERE status = 'active';

-- Add comments for documentation
COMMENT ON COLUMN membership_cards.status IS 'Card lifecycle status: active, expired, revoked, suspended';
COMMENT ON COLUMN membership_cards.expires_at IS 'Card expiration timestamp, extended on successful verification';
COMMENT ON COLUMN membership_cards.last_verified_at IS 'Last time membership was verified via YouTube API';
COMMENT ON COLUMN membership_cards.verification_failures IS 'Consecutive verification failures (reset on success)';
