-- The coach's photo, which her clients see next to her workouts and comments.
-- Kept in the private photo storage like clients' avatars, shown through
-- signed links. Only the coach adds it; there is no Telegram copy.
ALTER TABLE coaches ADD COLUMN avatar_path text;
