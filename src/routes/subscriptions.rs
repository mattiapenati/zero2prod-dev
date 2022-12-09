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
    let request_id = Uuid::new_v4();
    log::info!(
        "request_id {} - adding '{}' '{}' as a new subscriber",
        request_id,
        form.email,
        form.name,
    );
    log::info!(
        "request_id {} - saving new subscriber details in database",
        request_id
    );

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
        log::error!(
            "request_id {} - failed to execute query: {:?}",
            request_id,
            err
        );
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    log::info!(
        "request_id {} - new subscriber details have been saved",
        request_id
    );
    Ok(())
}

#[derive(Deserialize)]
pub struct FormData {
    email: String,
    name: String,
}
