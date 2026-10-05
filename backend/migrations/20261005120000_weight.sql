-- Measurements a client logs over time. Weight for now; the kind leaves room
-- for others (waist, hips) without another table.
CREATE TYPE measurement_kind AS ENUM ('weight');

CREATE TABLE body_measurements (
    id          uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    client_id   uuid NOT NULL REFERENCES clients (id) ON DELETE CASCADE,
    kind        measurement_kind NOT NULL,
    -- Kilograms for weight.
    value       numeric(6, 2) NOT NULL,
    -- The client's own calendar date. One entry a day: weighing again replaces it.
    measured_on date NOT NULL,
    created_at  timestamptz NOT NULL DEFAULT now(),
    UNIQUE (client_id, kind, measured_on)
);
