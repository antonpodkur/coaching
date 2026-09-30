-- Client invites and coach sign-in both go through the bot. Codes in links are
-- stored hashed: until it is used, a code works like a password.

ALTER TABLE clients RENAME COLUMN invite_code TO invite_code_hash;

CREATE TYPE coach_login_status AS ENUM ('pending', 'approved', 'cancelled', 'used');

CREATE TABLE coach_logins (
    id               uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    -- Travels in the bot link: t.me/<bot>?start=login_<code>.
    code_hash        text NOT NULL UNIQUE,
    -- Known only to the browser that started the login; used to collect the token.
    poll_secret_hash text NOT NULL UNIQUE,
    -- Shown on the page and in the bot, so the coach only confirms her own login.
    display_code     text NOT NULL,
    -- Set when a coach opens the link in Telegram.
    coach_id         uuid REFERENCES coaches (id),
    status           coach_login_status NOT NULL DEFAULT 'pending',
    expires_at       timestamptz NOT NULL,
    created_at       timestamptz NOT NULL DEFAULT now()
);
