use axum::{extract::State, Form};
use chrono::Utc;
use hyper::StatusCode;
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

pub async fn subscribe(
    State(db_pool): State<PgPool>,
    Form(form): Form<FormData>,
) -> Result<(), StatusCode> {
    sqlx::query!(
        "INSERT INTO subscriptions (id, email, name, subscribed_at) VALUES ($1, $2, $3, $4)",
        Uuid::new_v4(),
        form.email,
        form.name,
        Utc::now(),
    )
    .execute(&db_pool)
    .await
    .map_err(|err| {
        eprintln!("failed to execute query: {}", err);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    Ok(())
}

#[derive(Deserialize)]
pub struct FormData {
    email: String,
    name: String,
}
