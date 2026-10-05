-- The questionnaire a client fills in for Dasha; every answer is optional.
CREATE TYPE client_sex AS ENUM ('female', 'male');

ALTER TABLE clients
    ADD COLUMN birth_year integer,
    ADD COLUMN sex client_sex,
    ADD COLUMN height_cm integer;

-- What a client shows Dasha of their gym. A photo sits in the private storage
-- zone and is ready at once; a video goes to the private client video library
-- and is encoded first, like a technique video.
CREATE TYPE gym_media_kind AS ENUM ('photo', 'video');

CREATE TABLE gym_media (
    id          uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    client_id   uuid NOT NULL REFERENCES clients (id) ON DELETE CASCADE,
    kind        gym_media_kind NOT NULL,
    -- A photo's path in the storage zone, or a video's ID in the library.
    object_key  text NOT NULL UNIQUE,
    status      form_video_status NOT NULL,
    -- Videos only, once encoded.
    length_secs integer,
    created_at  timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX gym_media_client ON gym_media (client_id, created_at);
