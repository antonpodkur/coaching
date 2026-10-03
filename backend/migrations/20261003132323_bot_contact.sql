-- When the client let the bot message them: by pressing Start or writing to it,
-- or by allowing it in the Mini App. Joining through an app link skips Start,
-- so until then the bot cannot write first and sends them nothing.
ALTER TABLE clients ADD COLUMN bot_allowed_at timestamptz;
-- Everyone linked so far joined by pressing Start.
UPDATE clients SET bot_allowed_at = now() WHERE telegram_id IS NOT NULL;

-- Dasha's Telegram username, kept from her Mini App sign-in, so the welcome
-- can offer "Написати Даші".
ALTER TABLE coaches ADD COLUMN username text;
