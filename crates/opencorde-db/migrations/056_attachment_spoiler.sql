-- Add spoiler flag to uploaded attachments

ALTER TABLE files ADD COLUMN spoiler BOOLEAN NOT NULL DEFAULT FALSE;
