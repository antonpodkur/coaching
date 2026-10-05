-- Dasha's daily nutrition targets for a client: grams of protein, fat and
-- carbohydrates, the same every day until she changes them. A change is a new
-- row, so the history stays and the latest row is the current target.
CREATE TABLE nutrition_targets (
    id         uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    client_id  uuid NOT NULL REFERENCES clients (id) ON DELETE CASCADE,
    protein_g  integer NOT NULL,
    fat_g      integer NOT NULL,
    carbs_g    integer NOT NULL,
    note       text,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX nutrition_targets_client ON nutrition_targets (client_id, created_at DESC);
