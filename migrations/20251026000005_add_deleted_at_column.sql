-- Replace is_primary boolean with deleted_at timestamp for soft deletion
-- This migration enables proper card deletion tracking with timestamps
-- Note: This runs AFTER 20251026000004 which adds 'deleted' to card_status enum

-- Add deleted_at column
ALTER TABLE membership_cards ADD COLUMN deleted_at TIMESTAMPTZ NULL;

-- Migrate existing data: non-active cards are marked as deleted
-- Set both status='deleted' and deleted_at for consistency
UPDATE membership_cards
SET status = 'deleted',
    deleted_at = issued_at
WHERE status != 'active';

-- Drop old unique index (if it exists from previous schema)
DROP INDEX IF EXISTS uniq_active_membership_card;

-- Create new unique index: only one active (non-deleted) card per issuer/member
-- A card is considered active if status = 'active' AND deleted_at IS NULL
CREATE UNIQUE INDEX uniq_active_membership_card
ON membership_cards(issuer_id, member_id)
WHERE status = 'active' AND deleted_at IS NULL;

-- Add comment for documentation
COMMENT ON COLUMN membership_cards.deleted_at IS 'Soft deletion timestamp - set when status becomes deleted';
