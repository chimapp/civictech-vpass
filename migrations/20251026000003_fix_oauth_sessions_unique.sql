-- Fix oauth_sessions duplicate records issue
-- Each member should only have one active OAuth session

-- Delete duplicate oauth_sessions, keeping only the most recent one per member
DELETE FROM oauth_sessions
WHERE id NOT IN (
    SELECT DISTINCT ON (member_id) id
    FROM oauth_sessions
    ORDER BY member_id, created_at DESC
);

-- Add unique constraint to prevent future duplicates
ALTER TABLE oauth_sessions
  ADD CONSTRAINT unique_member_oauth UNIQUE (member_id);

COMMENT ON CONSTRAINT unique_member_oauth ON oauth_sessions IS
  'Ensures each member has only one OAuth session. Old sessions are replaced on re-authentication.';
