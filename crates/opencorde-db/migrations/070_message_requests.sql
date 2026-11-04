-- Migration: 070_message_requests
-- Message request state for DM channels and privacy settings.

-- Per-user message request status in DM channels.
-- accepted = regular DM (default for existing channels, friend DMs)
-- pending  = incoming message request awaiting recipient action
-- ignored  = recipient dismissed the request
-- spam     = recipient marked as spam
CREATE TYPE message_request_status AS ENUM ('accepted', 'pending', 'ignored', 'spam');

ALTER TABLE dm_channel_members
    ADD COLUMN message_request_status message_request_status NOT NULL DEFAULT 'accepted';

COMMENT ON COLUMN dm_channel_members.message_request_status IS
    'Per-recipient message request state: accepted (regular DM), pending (awaiting action), ignored, or spam';

CREATE INDEX idx_dm_channel_members_request_status
    ON dm_channel_members (user_id, message_request_status);

-- Privacy setting: who can send DMs to this user
ALTER TABLE users
    ADD COLUMN who_can_dm VARCHAR(20) NOT NULL DEFAULT 'everyone';

COMMENT ON COLUMN users.who_can_dm IS
    'DM privacy: everyone, friends_only';
