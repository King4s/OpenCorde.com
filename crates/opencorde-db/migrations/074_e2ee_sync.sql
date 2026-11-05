-- Public relay metadata and ciphertext only. No private MLS state.
CREATE TABLE e2ee_streams (
 channel_id BIGINT PRIMARY KEY REFERENCES channels(id) ON DELETE CASCADE,
 sequence BIGINT NOT NULL DEFAULT 0 CHECK(sequence >= 0),
 epoch BIGINT NOT NULL DEFAULT 0 CHECK(epoch >= 0)
);
CREATE TABLE e2ee_stream_members (
 channel_id BIGINT NOT NULL REFERENCES e2ee_streams(channel_id) ON DELETE CASCADE,
 user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
 joined_sequence BIGINT NOT NULL,
 active BOOLEAN NOT NULL DEFAULT TRUE,
 welcome_operation TEXT,
 welcome_sequence BIGINT,
 welcome_epoch BIGINT,
 welcome BYTEA,
 PRIMARY KEY(channel_id,user_id)
);
CREATE TABLE e2ee_events (
 channel_id BIGINT NOT NULL REFERENCES e2ee_streams(channel_id) ON DELETE CASCADE,
 sequence BIGINT NOT NULL,
 operation_id TEXT NOT NULL,
 sender_id BIGINT NOT NULL REFERENCES users(id),
 request JSONB NOT NULL,
 event JSONB NOT NULL,
 PRIMARY KEY(channel_id,sequence),
 UNIQUE(channel_id,operation_id)
);

-- One-time package publication remains idempotent even after logout deletes rows.
CREATE TABLE e2ee_key_package_publications (
 user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
 fingerprint TEXT NOT NULL,
 PRIMARY KEY(user_id,fingerprint)
);
INSERT INTO e2ee_key_package_publications(user_id,fingerprint)
 SELECT DISTINCT user_id,encode(sha256(key_package),'hex') FROM e2ee_key_packages;

-- Previously duplicated public packages must not be offered twice.
WITH packages AS (
 SELECT id, row_number() OVER(PARTITION BY user_id,key_package ORDER BY id) AS occurrence,
 bool_or(consumed_at IS NOT NULL) OVER(PARTITION BY user_id,key_package) AS already_used
 FROM e2ee_key_packages
)
UPDATE e2ee_key_packages k SET consumed_at=COALESCE(k.consumed_at,NOW())
FROM packages p WHERE k.id=p.id AND (p.occurrence>1 OR p.already_used);

-- Serialize legacy mutations with transition to the ordered stream. The bypass
-- is transaction-local and is set only by the authenticated v2 relay transaction.
CREATE FUNCTION guard_mls_stream_write() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 PERFORM id FROM channels WHERE id=NEW.channel_id FOR UPDATE;
 IF EXISTS(SELECT 1 FROM e2ee_streams WHERE channel_id=NEW.channel_id)
    AND (TG_TABLE_NAME <> 'messages' OR
         current_setting('opencorde.mls_channel',true) IS DISTINCT FROM NEW.channel_id::text)
 THEN
   RAISE EXCEPTION 'channel requires ordered MLS relay' USING ERRCODE='23514';
 END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER messages_mls_stream_guard BEFORE INSERT OR UPDATE OF content,attachments,channel_id ON messages
 FOR EACH ROW EXECUTE FUNCTION guard_mls_stream_write();
CREATE TRIGGER legacy_mls_state_guard BEFORE INSERT OR UPDATE ON e2ee_groups
 FOR EACH ROW EXECUTE FUNCTION guard_mls_stream_write();
