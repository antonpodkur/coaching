-- Phones report some timezones under old names, e.g. Europe/Kiev, and Postgres
-- without Debian's tzdata-legacy package does not know them: one such name
-- failed every background round. The API now stores current names (RENAMED in
-- src/timezone.rs); this rewrites the ones stored before.
WITH renamed (old_name, new_name) AS (
    VALUES
        ('Africa/Asmera', 'Africa/Asmara'),
        ('America/Buenos_Aires', 'America/Argentina/Buenos_Aires'),
        ('America/Catamarca', 'America/Argentina/Catamarca'),
        ('America/Cordoba', 'America/Argentina/Cordoba'),
        ('America/Godthab', 'America/Nuuk'),
        ('America/Indianapolis', 'America/Indiana/Indianapolis'),
        ('America/Jujuy', 'America/Argentina/Jujuy'),
        ('America/Louisville', 'America/Kentucky/Louisville'),
        ('America/Mendoza', 'America/Argentina/Mendoza'),
        ('Asia/Calcutta', 'Asia/Kolkata'),
        ('Asia/Katmandu', 'Asia/Kathmandu'),
        ('Asia/Rangoon', 'Asia/Yangon'),
        ('Asia/Saigon', 'Asia/Ho_Chi_Minh'),
        ('Atlantic/Faeroe', 'Atlantic/Faroe'),
        ('Europe/Kiev', 'Europe/Kyiv'),
        ('Europe/Uzhgorod', 'Europe/Kyiv'),
        ('Europe/Zaporozhye', 'Europe/Kyiv'),
        ('Pacific/Enderbury', 'Pacific/Kanton'),
        ('Pacific/Ponape', 'Pacific/Pohnpei'),
        ('Pacific/Truk', 'Pacific/Chuuk')
),
fixed_clients AS (
    UPDATE clients c SET timezone = r.new_name
    FROM renamed r
    WHERE c.timezone = r.old_name
)
UPDATE coaches co SET timezone = r.new_name
FROM renamed r
WHERE co.timezone = r.old_name;
