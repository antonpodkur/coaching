//! Timezone names as phones report them, stored under names Postgres knows.
//!
//! Phones report ICU's names, and ICU keeps some that the tz database has since
//! renamed: Chrome and Android still say `Europe/Kiev`. The tz database keeps
//! old names only as links in its `backward` file, which Debian now ships
//! separately (`tzdata-legacy`). Postgres without it, like the official
//! `postgres:16` image, refuses `AT TIME ZONE 'Europe/Kiev'`.

use sqlx::PgPool;

/// Old names that phones still report, with the current name of the same zone.
/// Migration `20261006090000_current_timezone_names.sql` rewrote the ones
/// stored before; a name added here needs a migration like it.
pub const RENAMED: &[(&str, &str)] = &[
    ("Africa/Asmera", "Africa/Asmara"),
    ("America/Buenos_Aires", "America/Argentina/Buenos_Aires"),
    ("America/Catamarca", "America/Argentina/Catamarca"),
    ("America/Cordoba", "America/Argentina/Cordoba"),
    ("America/Godthab", "America/Nuuk"),
    ("America/Indianapolis", "America/Indiana/Indianapolis"),
    ("America/Jujuy", "America/Argentina/Jujuy"),
    ("America/Louisville", "America/Kentucky/Louisville"),
    ("America/Mendoza", "America/Argentina/Mendoza"),
    ("Asia/Calcutta", "Asia/Kolkata"),
    ("Asia/Katmandu", "Asia/Kathmandu"),
    ("Asia/Rangoon", "Asia/Yangon"),
    ("Asia/Saigon", "Asia/Ho_Chi_Minh"),
    ("Atlantic/Faeroe", "Atlantic/Faroe"),
    ("Europe/Kiev", "Europe/Kyiv"),
    // Merged into Kyiv in 2022.
    ("Europe/Uzhgorod", "Europe/Kyiv"),
    ("Europe/Zaporozhye", "Europe/Kyiv"),
    ("Pacific/Enderbury", "Pacific/Kanton"),
    ("Pacific/Ponape", "Pacific/Pohnpei"),
    ("Pacific/Truk", "Pacific/Chuuk"),
];

/// The name to store for a timezone a phone reported: an IANA name, under its
/// current spelling, that this Postgres knows. `None` for anything else, so
/// `AT TIME ZONE` never fails on a stored name.
pub async fn storable(db: &PgPool, reported: &str) -> sqlx::Result<Option<&'static str>> {
    let Ok(timezone) = reported.parse::<chrono_tz::Tz>() else {
        return Ok(None);
    };
    let name = current_name(timezone.name());
    let known = sqlx::query_scalar!(
        r#"SELECT EXISTS (SELECT 1 FROM pg_timezone_names WHERE name = $1) AS "known!""#,
        name,
    )
    .fetch_one(db)
    .await?;
    Ok(known.then_some(name))
}

fn current_name(name: &'static str) -> &'static str {
    RENAMED
        .iter()
        .find(|(old, _)| *old == name)
        .map_or(name, |(_, current)| current)
}
