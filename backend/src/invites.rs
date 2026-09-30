//! Client invites: a single-use link that connects a Telegram account to a client
//! profile Dasha created. See "Authentication and onboarding" in the architecture doc.

use chrono::{DateTime, Duration, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::codes;

pub const INVITE_TTL: Duration = Duration::days(7);

/// Prefix of the `/start` payload that carries an invite code.
pub const START_PREFIX: &str = "inv_";

pub struct Invite {
    pub code: String,
    pub expires_at: DateTime<Utc>,
}

/// Creates a fresh invite for a client, replacing any earlier unused one.
/// `None` if the client does not exist, belongs to another coach or is archived.
pub async fn issue(db: &PgPool, coach_id: Uuid, client_id: Uuid) -> sqlx::Result<Option<Invite>> {
    let (code, code_hash) = codes::new_secret();
    let expires_at = Utc::now() + INVITE_TTL;
    let updated = sqlx::query!(
        "UPDATE clients SET invite_code_hash = $3, invite_expires_at = $4
         WHERE id = $1 AND coach_id = $2 AND archived_at IS NULL",
        client_id,
        coach_id,
        code_hash,
        expires_at,
    )
    .execute(db)
    .await?;
    Ok((updated.rows_affected() == 1).then_some(Invite { code, expires_at }))
}

#[derive(Debug, PartialEq, Eq)]
pub enum Accepted {
    /// The Telegram account is now linked to the client.
    Joined {
        client_name: String,
        coach_telegram_id: i64,
    },
    /// This Telegram account already belongs to a different client profile.
    LinkedElsewhere,
    /// Unknown, used or expired code.
    Invalid,
}

/// Links `telegram_id` to the client the invite was issued for and spends the code.
///
/// A new invite for a client who already joined re-links the profile to whoever
/// uses it, e.g. after the client switched Telegram accounts.
pub async fn accept(db: &PgPool, code: &str, telegram_id: i64) -> sqlx::Result<Accepted> {
    let mut tx = db.begin().await?;
    let invite = sqlx::query!(
        r#"SELECT c.id, c.name, co.telegram_id AS coach_telegram_id
           FROM clients c JOIN coaches co ON co.id = c.coach_id
           WHERE c.invite_code_hash = $1 AND c.invite_expires_at > now() AND c.archived_at IS NULL
           FOR UPDATE OF c"#,
        codes::hash(code),
    )
    .fetch_optional(&mut *tx)
    .await?;
    let Some(invite) = invite else {
        return Ok(Accepted::Invalid);
    };

    let linked_elsewhere = sqlx::query_scalar!(
        "SELECT id FROM clients WHERE telegram_id = $1 AND id <> $2",
        telegram_id,
        invite.id,
    )
    .fetch_optional(&mut *tx)
    .await?
    .is_some();
    if linked_elsewhere {
        return Ok(Accepted::LinkedElsewhere);
    }

    sqlx::query!(
        "UPDATE clients SET telegram_id = $2, invite_code_hash = NULL, invite_expires_at = NULL
         WHERE id = $1",
        invite.id,
        telegram_id,
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    Ok(Accepted::Joined {
        client_name: invite.name,
        coach_telegram_id: invite.coach_telegram_id,
    })
}
