-- Initial schema for VPass MVP: Channel Membership Verification Card System
-- Based on specs/001-channel-membership-verification/data-model.md

-- Create card_issuers table
CREATE TABLE card_issuers (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    platform TEXT NOT NULL CHECK (platform = 'youtube'),
    youtube_channel_id TEXT NOT NULL UNIQUE,
    channel_handle TEXT,
    channel_name TEXT NOT NULL,
    verification_video_id TEXT NOT NULL,
    default_membership_label TEXT NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Create index for active issuers lookup
CREATE INDEX idx_card_issuers_channel ON card_issuers(youtube_channel_id);
CREATE INDEX idx_card_issuers_active ON card_issuers(is_active) WHERE is_active = TRUE;

-- Create members table
CREATE TABLE members (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    youtube_user_id TEXT NOT NULL UNIQUE,
    default_display_name TEXT NOT NULL,
    avatar_url TEXT,
    locale TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Create index for member lookup by YouTube user ID
CREATE INDEX idx_members_youtube_user_id ON members(youtube_user_id);

-- Create oauth_sessions table
CREATE TABLE oauth_sessions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    member_id UUID NOT NULL REFERENCES members(id) ON DELETE CASCADE,
    access_token BYTEA NOT NULL,
    refresh_token BYTEA,
    token_scope TEXT NOT NULL,
    token_expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_used_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Create indexes for session management
CREATE INDEX idx_oauth_sessions_member ON oauth_sessions(member_id);
CREATE INDEX idx_oauth_sessions_expires ON oauth_sessions(token_expires_at);

-- Create membership_cards table
CREATE TABLE membership_cards (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    issuer_id UUID NOT NULL REFERENCES card_issuers(id),
    member_id UUID NOT NULL REFERENCES members(id),
    membership_level_label TEXT NOT NULL,
    membership_confirmed_at TIMESTAMPTZ NOT NULL,
    verification_comment_id TEXT NOT NULL,
    verification_video_id TEXT NOT NULL,
    snapshot_json JSONB NOT NULL,
    qr_payload JSONB NOT NULL,
    qr_signature TEXT NOT NULL,
    is_primary BOOLEAN NOT NULL DEFAULT TRUE,
    issued_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Unique constraint: only one primary card per issuer/member pair
CREATE UNIQUE INDEX uniq_active_membership_card ON membership_cards(issuer_id, member_id) WHERE is_primary = TRUE;

-- Create indexes for card lookups
CREATE INDEX idx_membership_cards_member ON membership_cards(member_id, issued_at DESC);
CREATE INDEX idx_membership_cards_issuer ON membership_cards(issuer_id, issued_at DESC);

-- Create updated_at trigger function
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Add triggers for updated_at columns
CREATE TRIGGER update_card_issuers_updated_at
    BEFORE UPDATE ON card_issuers
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_members_updated_at
    BEFORE UPDATE ON members
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();
