-- An exercise Dasha adds to one workout without putting it in her library.
-- Workouts, reports and history treat it like any other exercise; the
-- library, its search and plan imports leave it out.
ALTER TABLE exercises ADD COLUMN in_library boolean NOT NULL DEFAULT true;

-- Names are unique within the library only, so a one-off may repeat a name.
DROP INDEX exercises_coach_name_key;
CREATE UNIQUE INDEX exercises_coach_name_key
    ON exercises (coach_id, lower(name))
    WHERE archived_at IS NULL AND in_library;
