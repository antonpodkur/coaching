//! Sign-in confirmed through the bot, for everyone outside Telegram: an
//! installed app or a browser.
//!
//! 1. The app starts a login and gets a bot link, a poll secret and a display code.
//! 2. The person opens the link; the bot checks they have an account (the coach,
//!    or a client who joined) and asks them to confirm.
//! 3. Only the Confirm button approves; the app then collects the session once.
//!
//! The confirm step and the matching display code stop an attacker from getting
//! their own app signed in by sending someone their link.

use chrono::{DateTime, Duration, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::codes;

pub const LOGIN_TTL: Duration = Duration::minutes(5);

/// Prefix of the `/start` payload that carries a login code.
pub const START_PREFIX: &str = "login_";

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "bot_login_status", rename_all = "lowercase")]
pub enum LoginStatus {
    Pending,
    Approved,
    Cancelled,
    /// Opened by a Telegram account without access.
    Refused,
    Used,
}

pub struct StartedLogin {
    /// Goes into the bot link.
    pub code: String,
    /// Stays in the app, which uses it to collect the session.
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
        "INSERT INTO bot_logins (code_hash, poll_secret_hash, display_code, expires_at)
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
    /// Ask them to confirm this login.
    Confirm {
        login_id: Uuid,
        display_code: String,
    },
    /// The Telegram account is neither the coach nor a client who joined.
    NoAccount,
    /// Unknown, expired, already decided, or opened by someone else first.
    Invalid,
}

/// Called when someone opens the login link in Telegram.
pub async fn claim(db: &PgPool, code: &str, telegram_id: i64) -> sqlx::Result<Claimed> {
    let has_account = sqlx::query_scalar!(
        r#"SELECT EXISTS (SELECT 1 FROM coaches WHERE telegram_id = $1)
               OR EXISTS (SELECT 1 FROM clients WHERE telegram_id = $1 AND archived_at IS NULL)
           AS "has_account!""#,
        telegram_id,
    )
    .fetch_one(db)
    .await?;
    if !has_account {
        // The app can tell them now instead of waiting for the code to run out.
        sqlx::query!(
            "UPDATE bot_logins SET status = 'refused'
             WHERE code_hash = $1 AND status = 'pending' AND expires_at > now()
               AND telegram_id IS NULL",
            codes::hash(code),
        )
        .execute(db)
        .await?;
        return Ok(Claimed::NoAccount);
    }

    let login = sqlx::query!(
        "UPDATE bot_logins SET telegram_id = $2
         WHERE code_hash = $1 AND status = 'pending' AND expires_at > now()
           AND (telegram_id IS NULL OR telegram_id = $2)
         RETURNING id, display_code",
        codes::hash(code),
        telegram_id,
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

/// They pressed Confirm or Cancel. `false` if the login can no longer be
/// decided, or the button was pressed by someone other than who opened it.
pub async fn decide(
    db: &PgPool,
    login_id: Uuid,
    telegram_id: i64,
    approve: bool,
) -> sqlx::Result<bool> {
    let decided = sqlx::query!(
        "UPDATE bot_logins
         SET status = CASE WHEN $3 THEN 'approved'::bot_login_status
                           ELSE 'cancelled'::bot_login_status END
         WHERE id = $1 AND telegram_id = $2 AND status = 'pending' AND expires_at > now()",
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
    Refused,
    /// Also covers unknown secrets and logins whose session was already collected.
    Expired,
    /// Who confirmed it; the session is theirs.
    Approved {
        telegram_id: i64,
    },
}

/// Called by the app every few seconds. Hands out an approval only once.
pub async fn poll(db: &PgPool, poll_secret: &str) -> sqlx::Result<Polled> {
    let mut tx = db.begin().await?;
    let login = sqlx::query!(
        r#"SELECT id, status AS "status: LoginStatus", telegram_id, expires_at
           FROM bot_logins WHERE poll_secret_hash = $1
           FOR UPDATE"#,
        codes::hash(poll_secret),
    )
    .fetch_optional(&mut *tx)
    .await?;
    let Some(login) = login else {
        return Ok(Polled::Expired);
    };

    let polled = match (login.status, login.telegram_id) {
        _ if login.expires_at <= Utc::now() => Polled::Expired,
        (LoginStatus::Pending, _) => Polled::Pending,
        (LoginStatus::Cancelled, _) => Polled::Cancelled,
        (LoginStatus::Refused, _) => Polled::Refused,
        (LoginStatus::Approved, Some(telegram_id)) => {
            sqlx::query!(
                "UPDATE bot_logins SET status = 'used' WHERE id = $1",
                login.id
            )
            .execute(&mut *tx)
            .await?;
            Polled::Approved { telegram_id }
        }
        (LoginStatus::Approved, None) | (LoginStatus::Used, _) => Polled::Expired,
    };
    tx.commit().await?;
    Ok(polled)
}
