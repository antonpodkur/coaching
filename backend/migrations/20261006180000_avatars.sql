-- A client's avatar: a photo they added in their profile, or a copy of their
-- Telegram profile photo while they have not added one. Kept in the private
-- photo storage, shown through signed links.
ALTER TABLE clients
    ADD COLUMN avatar_path text,
    ADD COLUMN avatar_from_telegram boolean NOT NULL DEFAULT false,
    -- Telegram's id of the photo copied, so an unchanged one is not fetched again.
    ADD COLUMN telegram_photo_id text,
    -- When their Telegram photo was last looked at; it is looked at again weekly.
    ADD COLUMN telegram_photo_checked_at timestamptz;
