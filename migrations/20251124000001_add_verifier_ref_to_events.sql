-- Add verifier_ref to events table
-- Move VERIFIER_REF from global config to per-event configuration

-- Add column with default value for existing events
ALTER TABLE events ADD COLUMN verifier_ref TEXT NOT NULL DEFAULT '00000000_chimapp';

-- Remove default after backfill (new events must specify verifier_ref explicitly)
ALTER TABLE events ALTER COLUMN verifier_ref DROP DEFAULT;
