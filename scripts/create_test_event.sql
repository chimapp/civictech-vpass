-- Script to create test data for event testing

-- 1. Ensure we have a test issuer
INSERT INTO card_issuers (
    platform,
    youtube_channel_id,
    channel_handle,
    channel_name,
    verification_video_id,
    default_membership_label,
    is_active
) VALUES (
    'youtube',
    'UC_test_channel_12345',
    '@testchannel',
    'Test Channel for Events',
    'test_video_id_12345',
    'Member',
    true
)
ON CONFLICT (youtube_channel_id) DO NOTHING
RETURNING id, channel_name;

-- 2. Get the issuer_id (you'll need this for the next step)
-- Copy the id from above result

-- 3. Create a test event (replace <issuer_id> with actual id)
-- INSERT INTO events (
--     issuer_id,
--     event_name,
--     event_description,
--     event_date,
--     event_location,
--     is_active
-- ) VALUES (
--     '<issuer_id>',
--     '2025 春季演唱会',
--     '会员专属演唱会，现场互动',
--     '2025-10-26',
--     '台北小巨蛋',
--     true
-- );

-- 4. Verify the event was created
SELECT e.id, e.event_name, e.event_date, e.event_location,
       ci.channel_name
FROM events e
JOIN card_issuers ci ON e.issuer_id = ci.id
ORDER BY e.created_at DESC
LIMIT 5;
