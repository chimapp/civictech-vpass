-- Create verification_sessions table for OIDVP verification flow
-- This tracks each QR code verification session created by an event organizer

CREATE TABLE verification_sessions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_id UUID NOT NULL REFERENCES events(id) ON DELETE CASCADE,
    transaction_id VARCHAR(50) NOT NULL UNIQUE,
    qrcode_image TEXT NOT NULL,        -- base64 encoded PNG image
    auth_uri TEXT NOT NULL,             -- deep link for Taiwan Digital Wallet app
    status VARCHAR(20) NOT NULL DEFAULT 'pending',  -- 'pending', 'completed', 'expired', 'failed'
    verify_result BOOLEAN,              -- null if not completed, true/false after verification
    result_description TEXT,            -- description from OIDVP API
    result_data JSONB,                  -- full credential data from verifiable presentation
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ,           -- when verification was completed
    expires_at TIMESTAMPTZ NOT NULL,    -- QR code expires after 5 minutes
    CONSTRAINT valid_status CHECK (status IN ('pending', 'completed', 'expired', 'failed'))
);

-- Index for finding sessions by event
CREATE INDEX idx_verification_sessions_event_id ON verification_sessions(event_id);

-- Index for finding by transaction_id (for polling)
CREATE INDEX idx_verification_sessions_transaction_id ON verification_sessions(transaction_id);

-- Index for finding pending/expired sessions
CREATE INDEX idx_verification_sessions_status ON verification_sessions(status);

-- Index for cleanup of old sessions
CREATE INDEX idx_verification_sessions_created_at ON verification_sessions(created_at);
