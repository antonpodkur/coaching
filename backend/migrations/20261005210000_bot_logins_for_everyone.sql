-- The bot-confirmed sign-in is no longer only the coach's: outside Telegram
-- (an installed app, a browser) everyone with an account signs in this way,
-- the coach or a client who joined. Who it is gets decided when the app
-- collects the session, as in the Mini App's sign-in.

ALTER TABLE coach_logins RENAME TO bot_logins;
ALTER TABLE bot_logins RENAME CONSTRAINT coach_logins_pkey TO bot_logins_pkey;
ALTER TABLE bot_logins RENAME CONSTRAINT coach_logins_code_hash_key TO bot_logins_code_hash_key;
ALTER TABLE bot_logins
    RENAME CONSTRAINT coach_logins_poll_secret_hash_key TO bot_logins_poll_secret_hash_key;

ALTER TYPE coach_login_status RENAME TO bot_login_status;
-- Opened by a Telegram account without access, so the app can say so at once.
ALTER TYPE bot_login_status ADD VALUE 'refused';

-- Set when someone with an account opens the link in Telegram; only they can confirm it.
ALTER TABLE bot_logins ADD COLUMN telegram_id bigint;
UPDATE bot_logins l SET telegram_id = c.telegram_id FROM coaches c WHERE c.id = l.coach_id;
ALTER TABLE bot_logins DROP COLUMN coach_id;
