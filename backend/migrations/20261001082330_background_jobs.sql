-- Background jobs (src/jobs.rs): retrying messages the bot could not send,
-- Dasha's evening summary in her timezone, and which workouts were opened.

ALTER TABLE notifications
    -- Delivery attempts so far. Unsent rows are retried a few times.
    ADD COLUMN attempts        integer NOT NULL DEFAULT 0,
    ADD COLUMN last_attempt_at timestamptz,
    -- Nothing to send after all, e.g. the workout was deleted meanwhile.
    ADD COLUMN skipped         boolean NOT NULL DEFAULT false;

-- What the retry job scans.
CREATE INDEX notifications_unsent_idx ON notifications (created_at) WHERE sent_at IS NULL;

-- When the evening summary goes out. The app can set it later; Kyiv for now.
ALTER TABLE coaches ADD COLUMN timezone text NOT NULL DEFAULT 'Europe/Kyiv';

-- First time the client opened the workout in the Mini App.
ALTER TABLE workouts ADD COLUMN opened_at timestamptz;
