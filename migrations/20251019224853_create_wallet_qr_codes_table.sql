-- Create wallet_qr_codes table for Taiwan Digital Wallet integration
CREATE TABLE wallet_qr_codes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    card_id UUID NOT NULL REFERENCES membership_cards(id) ON DELETE CASCADE,

    -- Taiwan Digital Wallet API response fields
    transaction_id TEXT NOT NULL UNIQUE,
    qr_code TEXT NOT NULL,
    deep_link TEXT,

    -- Scan result (populated after user scans)
    cid TEXT,
    scanned_at TIMESTAMPTZ,

    -- Lifecycle management
    is_active BOOLEAN NOT NULL DEFAULT TRUE,

    -- Metadata
    provider TEXT NOT NULL DEFAULT 'tw_digital_wallet',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes
CREATE INDEX idx_wallet_qr_codes_card_id ON wallet_qr_codes(card_id);
CREATE INDEX idx_wallet_qr_codes_transaction_id ON wallet_qr_codes(transaction_id);
CREATE INDEX idx_wallet_qr_codes_active ON wallet_qr_codes(card_id, is_active) WHERE is_active = TRUE;

-- Unique constraint: only one active QR code per card
CREATE UNIQUE INDEX uniq_active_wallet_qr_code ON wallet_qr_codes(card_id) WHERE is_active = TRUE;

-- Updated_at trigger
CREATE TRIGGER update_wallet_qr_codes_updated_at
    BEFORE UPDATE ON wallet_qr_codes
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- Remove wallet_qr_data column from membership_cards
ALTER TABLE membership_cards DROP COLUMN wallet_qr_data;
