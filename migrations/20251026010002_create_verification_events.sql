-- Create verification_events table for spec-002
-- Records each QR scan verification attempt at an event

CREATE TABLE verification_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_id UUID NOT NULL REFERENCES events(id),
    card_id UUID REFERENCES membership_cards(id),  -- nullable: failed scans may not have valid card_id
    verification_result TEXT NOT NULL CHECK (
        verification_result IN (
            'success',
            'invalid_signature',
            'card_not_found',
            'invalid_payload'
        )
    ),
    verification_context JSONB,  -- extra metadata (IP, device, etc.)
    raw_payload TEXT,  -- original QR payload for debugging/forensics
    verified_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Create indexes for common queries
CREATE INDEX idx_verification_events_event ON verification_events(event_id, verified_at DESC);
CREATE INDEX idx_verification_events_card ON verification_events(card_id, verified_at DESC);
CREATE INDEX idx_verification_events_verified_at ON verification_events(verified_at DESC);
CREATE INDEX idx_verification_events_result ON verification_events(event_id, verification_result);
