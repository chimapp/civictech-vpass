-- Drop verification_sessions table
-- Rationale: Verification state should be managed in the browser (frontend JavaScript),
-- not in the database. Only successful verifications need to be persisted in verification_events.
--
-- Flow change:
-- BEFORE: request_qr → store in verification_sessions → poll → update verification_sessions → write verification_events
-- AFTER:  request_qr → return to frontend → frontend polls → success → write verification_events directly
--
-- Benefits:
-- - Reduced database writes (no pending state tracking)
-- - Simpler architecture (no ephemeral state in DB)
-- - Frontend has full control over verification UI state

DROP TABLE IF EXISTS verification_sessions;
