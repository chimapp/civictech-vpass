-- Fix mismatched timestamp types in wallet_qr_codes table
ALTER TABLE wallet_qr_codes
ALTER COLUMN created_at TYPE TIMESTAMPTZ USING created_at AT TIME ZONE 'UTC',
ALTER COLUMN updated_at TYPE TIMESTAMPTZ USING updated_at AT TIME ZONE 'UTC';

-- Create tower_sessions table
CREATE SCHEMA tower_sessions;
CREATE TABLE tower_sessions.session (
    id VARCHAR(255) PRIMARY KEY,
    expiry_date TIMESTAMPTZ NOT NULL,
    data BYTEA NOT NULL
);
