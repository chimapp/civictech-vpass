-- Create events table for spec-002
-- Represents physical events/venues where cards are verified

CREATE TABLE events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    issuer_id UUID NOT NULL REFERENCES card_issuers(id),
    event_name TEXT NOT NULL,
    event_description TEXT,
    event_date DATE NOT NULL,
    event_location TEXT,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Create indexes for common queries
CREATE INDEX idx_events_issuer ON events(issuer_id, event_date DESC);
CREATE INDEX idx_events_active ON events(is_active) WHERE is_active = TRUE;
CREATE INDEX idx_events_date ON events(event_date);

-- Add trigger for updated_at
CREATE TRIGGER update_events_updated_at
    BEFORE UPDATE ON events
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();
