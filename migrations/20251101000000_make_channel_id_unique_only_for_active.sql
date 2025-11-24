-- Allow reusing a YouTube channel ID after the issuer has been deactivated
-- by making the unique constraint conditional on is_active = TRUE.

-- Drop the original unique constraint on the column
ALTER TABLE card_issuers
DROP CONSTRAINT IF EXISTS card_issuers_youtube_channel_id_key;

-- Enforce uniqueness only for active issuers
CREATE UNIQUE INDEX IF NOT EXISTS uniq_active_card_issuer_channel
ON card_issuers(youtube_channel_id)
WHERE is_active = TRUE;
