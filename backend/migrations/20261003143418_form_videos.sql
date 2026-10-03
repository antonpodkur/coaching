-- A client's own video of how they did an exercise, for Dasha to check their
-- technique. Kept in a separate, private Bunny library and played only through
-- signed links. Videos are kept until the exercise or the client is deleted.
CREATE TYPE form_video_status AS ENUM ('uploading', 'processing', 'ready', 'failed');

CREATE TABLE form_videos (
    id                  uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    workout_exercise_id uuid NOT NULL REFERENCES workout_exercises (id) ON DELETE CASCADE,
    client_id           uuid NOT NULL REFERENCES clients (id) ON DELETE CASCADE,
    -- The video in the client-videos library.
    video_uid           text NOT NULL UNIQUE,
    status              form_video_status NOT NULL DEFAULT 'uploading',
    length_secs         integer,
    created_at          timestamptz NOT NULL DEFAULT now(),
    ready_at            timestamptz,
    -- Dasha opened the workout's report after the video was ready.
    seen_at             timestamptz
);

CREATE INDEX form_videos_workout_exercise_idx ON form_videos (workout_exercise_id);
CREATE INDEX form_videos_processing_idx ON form_videos (created_at) WHERE status = 'processing';
