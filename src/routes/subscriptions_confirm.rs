use axum::extract::{Query, State};
use http::StatusCode;
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

#[tracing::instrument(
    level = "info",
    name = "confirm a pending subscriber",
    skip(db_pool, parameters)
)]
pub async fn confirm(
    State(db_pool): State<PgPool>,
    Query(parameters): Query<Parameters>,
) -> Result<(), StatusCode> {
    let subscriber_id = get_subscriber_id_from_token(&db_pool, &parameters.subscription_token)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::UNAUTHORIZED)?;

    confirm_subscriber(&db_pool, subscriber_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(())
}

#[tracing::instrument(
    level = "info",
    name = "mark subscriber as confirmed",
    skip(db_pool, subscriber_id)
)]
async fn confirm_subscriber(db_pool: &PgPool, subscriber_id: Uuid) -> sqlx::Result<()> {
    sqlx::query!(
        r#"UPDATE subscriptions SET status = 'confirmed' WHERE id = $1"#,
        subscriber_id,
    )
    .execute(db_pool)
    .await
    .map_err(|err| {
        tracing::error!("failed to execute query: {:?}", err);
        err
    })?;
    Ok(())
}

#[tracing::instrument(
    level = "info",
    name = "get subscriber_id from token",
    skip(db_pool, subscription_token)
)]
async fn get_subscriber_id_from_token(
    db_pool: &PgPool,
    subscription_token: &str,
) -> sqlx::Result<Option<Uuid>> {
    let result = sqlx::query!(
        r#"SELECT subscriber_id FROM subscription_tokens
            WHERE subscription_token = $1"#,
        subscription_token,
    )
    .fetch_optional(db_pool)
    .await
    .map_err(|err| {
        tracing::error!("failed to execute query: {:?}", err);
        err
    })?;
    Ok(result.map(|r| r.subscriber_id))
}

#[derive(Deserialize)]
pub struct Parameters {
    subscription_token: String,
}
