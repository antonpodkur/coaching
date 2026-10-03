-- How an exercise's sets are counted, set once in the library.
--   weight:     kg × reps.
--   bodyweight: reps; target_kg/actual_kg only for optional extra weight (a belt).
--   time:       the reps columns hold seconds (a plank, a bike); kg is optional extra weight.
-- Seconds share the reps columns, so saving, copying, "last time" and
-- set_differs() work unchanged. The API refuses switching an exercise to or
-- from 'time' once it has sets, so old numbers never change meaning.
CREATE TYPE exercise_measure AS ENUM ('weight', 'bodyweight', 'time');

ALTER TABLE exercises ADD COLUMN measure exercise_measure NOT NULL DEFAULT 'weight';
