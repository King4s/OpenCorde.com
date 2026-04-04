-- Split forum channels off from categories.
-- Existing forum channels were historically stored as channel_type = 2.
-- We identify them by forum posts and migrate them to channel_type = 5.

UPDATE channels c
SET channel_type = 5
WHERE c.channel_type = 2
  AND EXISTS (
    SELECT 1
    FROM forum_posts fp
    WHERE fp.channel_id = c.id
  );
