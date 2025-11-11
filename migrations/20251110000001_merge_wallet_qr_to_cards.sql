-- Merge wallet_qr_codes into membership_cards
-- Rationale: 1:1 relationship (one card → one QR, no re-scanning needed, single format)

-- Step 1: Add wallet-related columns to membership_cards
ALTER TABLE membership_cards
ADD COLUMN wallet_transaction_id TEXT,
ADD COLUMN wallet_qr_code TEXT,
ADD COLUMN wallet_deep_link TEXT,
ADD COLUMN wallet_cid TEXT,
ADD COLUMN wallet_scanned_at TIMESTAMPTZ;

-- Step 2: Migrate data from wallet_qr_codes to membership_cards
-- Only migrate the active QR code (is_active = true)
UPDATE membership_cards mc
SET
    wallet_transaction_id = wqc.transaction_id,
    wallet_qr_code = wqc.qr_code,
    wallet_deep_link = wqc.deep_link,
    wallet_cid = wqc.cid,
    wallet_scanned_at = wqc.scanned_at
FROM wallet_qr_codes wqc
WHERE mc.id = wqc.card_id
  AND wqc.is_active = true;

-- Step 3: Create index on transaction_id for poll operations
CREATE INDEX idx_membership_cards_transaction_id ON membership_cards(wallet_transaction_id);

-- Step 4: Drop the wallet_qr_codes table
DROP TABLE wallet_qr_codes;

-- Note: This migration assumes no production data requires preserving inactive QR codes.
-- If historical QR tracking is needed, archive wallet_qr_codes data before running this.
