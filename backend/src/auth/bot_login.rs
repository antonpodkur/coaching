//! Coach sign-in confirmed through the bot.
//!
//! 1. The browser starts a login and gets a bot link, a poll secret and a display code.
//! 2. The coach opens the link; the bot checks she is a coach and asks her to confirm.
//! 3. Only the Confirm button approves; the browser then collects the token once.
//!
//! The confirm step and the matching display code stop an attacker from getting
//! their own browser signed in by sending the coach their link.

use chrono::{DateTime, Duration, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::codes;

pub const LOGIN_TTL: Duration = Duration::minutes(5);

/// Prefix of the `/start` payload that carries a login code.
pub const START_PREFIX: &str = "login_";

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "coach_login_status", rename_all = "lowercase")]
pub enum LoginStatus {
    Pending,
    Approved,
    Cancelled,
    Used,
}

pub struct StartedLogin {
    /// Goes into the bot link.
    pub code: String,
    /// Stays in the browser, which uses it to collect the token.
    pub poll_secret: String,
    pub display_code: String,
    pub expires_at: DateTime<Utc>,
}

pub async fn start(db: &PgPool) -> sqlx::Result<StartedLogin> {
    let (code, code_hash) = codes::new_secret();
    let (poll_secret, poll_secret_hash) = codes::new_secret();
    let display_code = codes::display_code();
    let expires_at = Utc::now() + LOGIN_TTL;
    sqlx::query!(
        "INSERT INTO coach_logins (code_hash, poll_secret_hash, display_code, expires_at)
         VALUES ($1, $2, $3, $4)",
        code_hash,
        poll_secret_hash,
        display_code,
        expires_at,
    )
    .execute(db)
    .await?;
    Ok(StartedLogin {
        code,
        poll_secret,
        display_code,
        expires_at,
    })
}

#[derive(Debug, PartialEq, Eq)]
pub enum Claimed {
    /// Ask the coach to confirm this login.
    Confirm {
        login_id: Uuid,
        display_code: String,
    },
    /// The Telegram account is not a coach.
    NotACoach,
    /// Unknown, expired, already decided, or opened by another coach.
    Invalid,
}

/// Called when someone opens the login link in Telegram.
pub async fn claim(db: &PgPool, code: &str, telegram_id: i64) -> sqlx::Result<Claimed> {
    let coach_id =
        sqlx::query_scalar!("SELECT id FROM coaches WHERE telegram_id = $1", telegram_id)
            .fetch_optional(db)
            .await?;
    let Some(coach_id) = coach_id else {
        return Ok(Claimed::NotACoach);
    };

    let login = sqlx::query!(
        "UPDATE coach_logins SET coach_id = $2
         WHERE code_hash = $1 AND status = 'pending' AND expires_at > now()
           AND (coach_id IS NULL OR coach_id = $2)
         RETURNING id, display_code",
        codes::hash(code),
        coach_id,
    )
    .fetch_optional(db)
    .await?;

    Ok(match login {
        Some(login) => Claimed::Confirm {
            login_id: login.id,
            display_code: login.display_code,
        },
        None => Claimed::Invalid,
    })
}

/// The coach pressed Confirm or Cancel. `false` if the login can no longer be
/// decided, or the button was pressed by someone other than that coach.
pub async fn decide(
    db: &PgPool,
    login_id: Uuid,
    telegram_id: i64,
    approve: bool,
) -> sqlx::Result<bool> {
    let decided = sqlx::query!(
        "UPDATE coach_logins l
         SET status = CASE WHEN $3 THEN 'approved'::coach_login_status
                           ELSE 'cancelled'::coach_login_status END
         FROM coaches c
         WHERE l.id = $1 AND l.coach_id = c.id AND c.telegram_id = $2
           AND l.status = 'pending' AND l.expires_at > now()",
        login_id,
        telegram_id,
        approve,
    )
    .execute(db)
    .await?;
    Ok(decided.rows_affected() == 1)
}

#[derive(Debug, PartialEq, Eq)]
pub enum Polled {
    Pending,
    Cancelled,
    /// Also covers unknown secrets and logins whose token was already collected.
    Expired,
    Approved {
        coach_id: Uuid,
    },
}

/// Called by the browser every few seconds. Hands out an approval only once.
pub async fn poll(db: &PgPool, poll_secret: &str) -> sqlx::Result<Polled> {
    let mut tx = db.begin().await?;
    let login = sqlx::query!(
        r#"SELECT id, status AS "status: LoginStatus", coach_id, expires_at
           FROM coach_logins WHERE poll_secret_hash = $1
           FOR UPDATE"#,
        codes::hash(poll_secret),
    )
    .fetch_optional(&mut *tx)
    .await?;
    let Some(login) = login else {
        return Ok(Polled::Expired);
    };

    let polled = match (login.status, login.coach_id) {
        _ if login.expires_at <= Utc::now() => Polled::Expired,
        (LoginStatus::Pending, _) => Polled::Pending,
        (LoginStatus::Cancelled, _) => Polled::Cancelled,
        (LoginStatus::Approved, Some(coach_id)) => {
            sqlx::query!(
                "UPDATE coach_logins SET status = 'used' WHERE id = $1",
                login.id
            )
            .execute(&mut *tx)
            .await?;
            Polled::Approved { coach_id }
        }
        (LoginStatus::Approved, None) | (LoginStatus::Used, _) => Polled::Expired,
    };
    tx.commit().await?;
    Ok(polled)
}
