-- Dasha's report view: which reports she has looked at, and one definition of
-- "done differently from the plan" for every query that counts it.

ALTER TABLE workout_reports ADD COLUMN seen_at timestamptz;

-- A ticked set whose logged numbers are not what was planned: another weight
-- (bodyweight counts as NULL), or reps missing or outside the target range.
CREATE FUNCTION set_differs(s workout_sets) RETURNS boolean
LANGUAGE sql IMMUTABLE AS $$
    SELECT s.completed_at IS NOT NULL
       AND (s.actual_kg IS DISTINCT FROM s.target_kg
            OR s.actual_reps IS NULL
            OR s.actual_reps NOT BETWEEN s.target_reps_min AND s.target_reps_max)
$$;
