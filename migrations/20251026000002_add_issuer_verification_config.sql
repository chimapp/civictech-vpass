-- Add issuer verification configuration for membership checking
-- Part of Spec 003: Card Lifecycle Automation

ALTER TABLE card_issuers
  ADD COLUMN members_only_video_id TEXT,
  ADD COLUMN verification_method VARCHAR(20) DEFAULT 'video'
    CHECK (verification_method IN ('video', 'comment'));

-- Add comments for documentation
COMMENT ON COLUMN card_issuers.members_only_video_id IS
  'YouTube members-only video ID for verifying membership status. Non-members get 403 when accessing.';
COMMENT ON COLUMN card_issuers.verification_method IS
  'Verification strategy: video = access members-only video, comment = access comment thread';
