-- Exercise videos on Bunny Stream. The playable video and the newest upload are
-- kept apart, so replacing a video never leaves clients without one.
-- See docs/ARCHITECTURE.md, "Video pipeline".

CREATE TYPE upload_status AS ENUM ('uploading', 'processing', 'failed');

ALTER TABLE exercises
    DROP COLUMN video_status,
    ADD COLUMN video_length_secs integer,
    -- The newest upload: in flight, being encoded, or failed. Cleared once it
    -- becomes the playable `video_uid`.
    ADD COLUMN upload_video_uid  text UNIQUE,
    ADD COLUMN upload_status     upload_status,
    ADD COLUMN upload_started_at timestamptz,
    ADD CONSTRAINT exercises_upload_fields_together CHECK (
        (upload_video_uid IS NULL) = (upload_status IS NULL)
        AND (upload_video_uid IS NULL) = (upload_started_at IS NULL)
    );

DROP TYPE video_status;

COMMENT ON COLUMN exercises.video_uid IS
    'Bunny Stream video ID that clients play; NULL until the first upload is ready.';
