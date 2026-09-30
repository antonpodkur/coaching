-- v1 schema. See docs/ARCHITECTURE.md, "Data model".

CREATE TYPE workout_status AS ENUM ('draft', 'published', 'done');
CREATE TYPE workout_source AS ENUM ('builder', 'copy', 'template', 'import');
CREATE TYPE video_status AS ENUM ('none', 'uploading', 'ready', 'failed');
CREATE TYPE effort AS ENUM ('easy', 'ok', 'hard');

CREATE TABLE coaches (
    id          uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    telegram_id bigint NOT NULL UNIQUE,
    name        text NOT NULL,
    created_at  timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE clients (
    id                uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    coach_id          uuid NOT NULL REFERENCES coaches (id),
    name              text NOT NULL,
    telegram_id       bigint UNIQUE,
    invite_code       text UNIQUE,
    invite_expires_at timestamptz,
    paid_until        date,
    -- IANA name reported by the client's phone, e.g. 'Europe/Kyiv'.
    timezone          text,
    created_at        timestamptz NOT NULL DEFAULT now(),
    archived_at       timestamptz
);

CREATE TABLE exercises (
    id           uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    coach_id     uuid NOT NULL REFERENCES coaches (id),
    name         text NOT NULL,
    muscle_group text,
    -- Other spellings used to match imported Telegram plans.
    aliases      text[] NOT NULL DEFAULT '{}',
    video_uid    text,
    video_status video_status NOT NULL DEFAULT 'none',
    created_at   timestamptz NOT NULL DEFAULT now(),
    -- Exercises are archived, never deleted: old workouts refer to them.
    archived_at  timestamptz
);

CREATE UNIQUE INDEX exercises_coach_name_key
    ON exercises (coach_id, lower(name))
    WHERE archived_at IS NULL;

CREATE TABLE workouts (
    id             uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    coach_id       uuid NOT NULL REFERENCES coaches (id),
    -- NULL client means the workout is a template.
    client_id      uuid REFERENCES clients (id) ON DELETE CASCADE,
    -- Calendar date in the client's timezone.
    date           date,
    title          text NOT NULL DEFAULT '',
    status         workout_status NOT NULL DEFAULT 'draft',
    source         workout_source NOT NULL DEFAULT 'builder',
    copied_from_id uuid REFERENCES workouts (id) ON DELETE SET NULL,
    -- Optimistic locking for whole-document saves from the builder.
    version        integer NOT NULL DEFAULT 1,
    published_at   timestamptz,
    created_at     timestamptz NOT NULL DEFAULT now(),
    updated_at     timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT templates_stay_drafts CHECK (client_id IS NOT NULL OR status = 'draft')
);

CREATE INDEX workouts_client_date_idx ON workouts (client_id, date DESC);

CREATE TABLE workout_exercises (
    id             uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    workout_id     uuid NOT NULL REFERENCES workouts (id) ON DELETE CASCADE,
    exercise_id    uuid NOT NULL REFERENCES exercises (id),
    position       integer NOT NULL,
    per_side_label text,
    note           text,
    CONSTRAINT workout_exercises_position_key
        UNIQUE (workout_id, position) DEFERRABLE INITIALLY DEFERRED
);

-- Serves "last time" lookups: latest sets of an exercise for a client.
CREATE INDEX workout_exercises_exercise_idx ON workout_exercises (exercise_id, workout_id);

CREATE TABLE workout_sets (
    id                  uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    workout_exercise_id uuid NOT NULL REFERENCES workout_exercises (id) ON DELETE CASCADE,
    position            integer NOT NULL,
    -- Written by the coach. NULL kg means bodyweight; '8-10' is min 8, max 10.
    target_kg           numeric(6, 2),
    target_reps_min     integer NOT NULL,
    target_reps_max     integer NOT NULL,
    -- Written by the client.
    actual_kg           numeric(6, 2),
    actual_reps         integer,
    completed_at        timestamptz,
    -- Lets the server ignore stale offline writes that arrive late.
    client_updated_at   timestamptz,
    CONSTRAINT workout_sets_reps_range CHECK (target_reps_min > 0 AND target_reps_min <= target_reps_max),
    CONSTRAINT workout_sets_position_key
        UNIQUE (workout_exercise_id, position) DEFERRABLE INITIALLY DEFERRED
);

CREATE TABLE workout_reports (
    workout_id   uuid PRIMARY KEY REFERENCES workouts (id) ON DELETE CASCADE,
    effort       effort NOT NULL,
    comment      text NOT NULL DEFAULT '',
    finished_at  timestamptz NOT NULL DEFAULT now(),
    duration_min integer
);

-- One row per message the bot sends; the unique key makes job re-runs harmless.
CREATE TABLE notifications (
    id         uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    kind       text NOT NULL,
    entity_id  uuid NOT NULL,
    local_date date NOT NULL,
    sent_at    timestamptz,
    created_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT notifications_once_key UNIQUE (kind, entity_id, local_date)
);
