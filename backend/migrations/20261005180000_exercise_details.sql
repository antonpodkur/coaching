-- How Dasha describes an exercise, and photos of it (the machine, the handle,
-- the starting position), shown to clients under her video.
ALTER TABLE exercises ADD COLUMN description text;

CREATE TABLE exercise_photos (
    id          uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    exercise_id uuid NOT NULL REFERENCES exercises (id) ON DELETE CASCADE,
    -- The photo's path in the storage zone.
    path        text NOT NULL UNIQUE,
    created_at  timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX exercise_photos_exercise ON exercise_photos (exercise_id, created_at);
