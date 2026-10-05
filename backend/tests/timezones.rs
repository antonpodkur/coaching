//! Timezone names. Phones report some under old names (Chrome and Android say
//! `Europe/Kiev`) that Postgres without Debian's tzdata-legacy does not know.
//! The API stores current names, and a migration rewrote the old ones stored
//! before.

mod common;

use axum::http::StatusCode;
use coaching_backend::{
    auth::{Role, jwt::MINI_APP_TOKEN_TTL},
    timezone::RENAMED,
};
use common::{call, coach_token, seed_client, seed_coach, test_state};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

const COACH_TG: i64 = 555_000_222;
const CLIENT_TG: i64 = 777_000_111;
const OTHER_CLIENT_TG: i64 = 777_000_222;

const MIGRATION: &str = include_str!("../migrations/20261006090000_current_timezone_names.sql");

async fn postgres_knows(db: &PgPool, name: &str) -> bool {
    sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM pg_timezone_names WHERE name = $1)")
        .bind(name)
        .fetch_one(db)
        .await
        .unwrap()
}

async fn client_timezone(db: &PgPool, client_id: Uuid) -> Option<String> {
    sqlx::query_scalar("SELECT timezone FROM clients WHERE id = $1")
        .bind(client_id)
        .fetch_one(db)
        .await
        .unwrap()
}

async fn coach_timezone(db: &PgPool, coach_id: Uuid) -> String {
    sqlx::query_scalar("SELECT timezone FROM coaches WHERE id = $1")
        .bind(coach_id)
        .fetch_one(db)
        .await
        .unwrap()
}

#[sqlx::test]
async fn old_names_are_stored_under_current_ones(db: PgPool) {
    let coach_id = seed_coach(&db, COACH_TG).await;
    let client_id = seed_client(&db, coach_id, CLIENT_TG).await;
    let state = test_state(db.clone());
    let app = coaching_backend::router(state.clone());
    let client_token = state
        .jwt
        .issue(Role::Client, client_id, MINI_APP_TOKEN_TTL)
        .unwrap();

    // What Chrome on a Ukrainian phone reports.
    let (status, _) = call(
        &app,
        "PUT",
        "/me/timezone",
        Some(&client_token),
        Some(json!({ "timezone": "Europe/Kiev" })),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (_, me) = call(&app, "GET", "/me", Some(&client_token), None).await;
    assert_eq!(me["timezone"], "Europe/Kyiv");

    let (status, _) = call(
        &app,
        "PUT",
        "/coach/me/timezone",
        Some(&coach_token(&state, coach_id)),
        Some(json!({ "timezone": "Europe/Kiev" })),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(coach_timezone(&db, coach_id).await, "Europe/Kyiv");
}

#[sqlx::test]
async fn names_postgres_does_not_know_are_refused(db: PgPool) {
    let coach_id = seed_coach(&db, COACH_TG).await;
    let client_id = seed_client(&db, coach_id, CLIENT_TG).await;
    let state = test_state(db.clone());
    let app = coaching_backend::router(state.clone());
    let client_token = state
        .jwt
        .issue(Role::Client, client_id, MINI_APP_TOKEN_TTL)
        .unwrap();
    let coach_token = coach_token(&state, coach_id);

    // An old name with no mapping. Only Postgres with tzdata-legacy knows it;
    // the official postgres:16 image (CI and local) does not.
    let known = postgres_knows(&db, "US/Eastern").await;
    for (path, token) in [
        ("/me/timezone", &client_token),
        ("/coach/me/timezone", &coach_token),
    ] {
        let (status, body) = call(
            &app,
            "PUT",
            path,
            Some(token),
            Some(json!({ "timezone": "US/Eastern" })),
        )
        .await;
        if known {
            assert_eq!(status, StatusCode::NO_CONTENT, "{path}");
        } else {
            assert_eq!(status, StatusCode::BAD_REQUEST, "{path}");
            assert_eq!(body["error"], "unknown_timezone", "{path}");
        }
    }

    if known {
        assert_eq!(
            client_timezone(&db, client_id).await.as_deref(),
            Some("US/Eastern")
        );
        assert_eq!(coach_timezone(&db, coach_id).await, "US/Eastern");
    } else {
        assert_eq!(client_timezone(&db, client_id).await, None);
        assert_eq!(coach_timezone(&db, coach_id).await, "Europe/Kyiv");
    }
}

#[sqlx::test]
async fn renamed_zones_map_to_names_postgres_knows(db: PgPool) {
    for (old, current) in RENAMED {
        // An old name the parser refused would never reach the mapping.
        assert!(old.parse::<chrono_tz::Tz>().is_ok(), "{old}");
        assert!(postgres_knows(&db, current).await, "{current}");
    }
}

#[sqlx::test]
async fn the_migration_rewrites_old_names_stored_before(db: PgPool) {
    let coach_id = seed_coach(&db, COACH_TG).await;
    let client_id = seed_client(&db, coach_id, CLIENT_TG).await;
    let other_id = seed_client(&db, coach_id, OTHER_CLIENT_TG).await;
    sqlx::query("UPDATE clients SET timezone = 'Europe/Warsaw' WHERE id = $1")
        .bind(other_id)
        .execute(&db)
        .await
        .unwrap();

    // Every name the API maps; a name added to RENAMED needs a migration too.
    for (old, current) in RENAMED {
        sqlx::query("UPDATE clients SET timezone = $2 WHERE id = $1")
            .bind(client_id)
            .bind(old)
            .execute(&db)
            .await
            .unwrap();
        sqlx::query("UPDATE coaches SET timezone = $2 WHERE id = $1")
            .bind(coach_id)
            .bind(old)
            .execute(&db)
            .await
            .unwrap();
        sqlx::raw_sql(MIGRATION).execute(&db).await.unwrap();
        assert_eq!(
            client_timezone(&db, client_id).await.as_deref(),
            Some(*current),
            "{old}"
        );
        assert_eq!(coach_timezone(&db, coach_id).await, *current, "{old}");
    }

    // Current names and missing ones stay as they are.
    sqlx::query("UPDATE clients SET timezone = NULL WHERE id = $1")
        .bind(client_id)
        .execute(&db)
        .await
        .unwrap();
    sqlx::raw_sql(MIGRATION).execute(&db).await.unwrap();
    assert_eq!(client_timezone(&db, client_id).await, None);
    assert_eq!(
        client_timezone(&db, other_id).await.as_deref(),
        Some("Europe/Warsaw")
    );
}
