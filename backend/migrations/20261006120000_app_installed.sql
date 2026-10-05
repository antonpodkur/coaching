-- When someone first used the app installed on a phone's home screen. Until
-- then the Telegram version offers to install it; after that it stops asking,
-- on every device.
ALTER TABLE clients ADD COLUMN app_installed_at timestamptz;
ALTER TABLE coaches ADD COLUMN app_installed_at timestamptz;
