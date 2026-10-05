-- Phones (and computers) that get notifications from the installed app: one
-- row per browser that allowed them, belonging to a client or to the coach.
CREATE TABLE push_subscriptions (
    id         uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    client_id  uuid REFERENCES clients (id) ON DELETE CASCADE,
    coach_id   uuid REFERENCES coaches (id) ON DELETE CASCADE,
    -- The browser's address at its push service. Signing in as someone else
    -- on the same phone moves it to them.
    endpoint   text NOT NULL UNIQUE,
    -- The browser's keys: the message is encrypted for it alone.
    p256dh     text NOT NULL,
    auth       text NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT push_subscriptions_one_owner CHECK ((client_id IS NULL) <> (coach_id IS NULL))
);
CREATE INDEX push_subscriptions_client_id_idx ON push_subscriptions (client_id);
CREATE INDEX push_subscriptions_coach_id_idx ON push_subscriptions (coach_id);

-- A notification is pushed once; when the bot's message fails and is retried,
-- the push is not sent again.
ALTER TABLE notifications ADD COLUMN pushed_at timestamptz;
