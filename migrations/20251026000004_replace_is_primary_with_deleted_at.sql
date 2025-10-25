-- Add 'deleted' value to card_status enum
-- This must be in a separate migration from usage due to PostgreSQL transaction limitations

ALTER TYPE card_status ADD VALUE IF NOT EXISTS 'deleted';
