use axum::{extract::State, Form};
use chrono::Utc;
use hyper::StatusCode;
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

#[tracing::instrument(
    level = "info",
    name = "adding a new subscriber",
    skip(db_pool, form),
    fields(
        subscriber_email = %form.email,
        subscriber_name = %form.name,
    ),
)]
pub async fn subscribe(
    State(db_pool): State<PgPool>,
    Form(form): Form<FormData>,
) -> Result<(), StatusCode> {
    insert_subscriber(&db_pool, &form)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(())
}

#[tracing::instrument(
    level = "info",
    name = "saving new subscriber details in the database",
    skip(db_pool, form)
)]
async fn insert_subscriber(db_pool: &PgPool, form: &FormData) -> sqlx::Result<()> {
    sqlx::query!(
        "INSERT INTO subscriptions (id, email, name, subscribed_at) VALUES ($1, $2, $3, $4)",
        Uuid::new_v4(),
        form.email,
        form.name,
        Utc::now(),
    )
    .execute(db_pool)
    .await
    .map_err(|err| {
        tracing::error!("failed to execute query: {:?}", err);
        err
    })?;
    Ok(())
}

#[derive(Deserialize)]
pub struct FormData {
    email: String,
    name: String,
}
