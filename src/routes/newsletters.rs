use anyhow::Context;
use axum::{extract::State, response::IntoResponse, Json};
use http::StatusCode;
use serde::Deserialize;
use sqlx::PgPool;

use crate::email_client::EmailClient;

#[tracing::instrument(
    level = "info",
    name = "publish a newsletter issue",
    skip(db_pool, email_client, body)
)]
pub async fn publish_newsletter(
    State(db_pool): State<PgPool>,
    State(email_client): State<EmailClient>,
    Json(body): Json<BodyData>,
) -> Result<(), PublishError> {
    let subscribers = get_confirmed_subscribers(&db_pool).await.context("")?;
    for subscriber in subscribers {
        email_client
            .send_email(
                subscriber.email,
                &body.title,
                &body.content.html,
                &body.content.text,
            )
            .await
            .map_err(PublishError::SendEmailError)
            .with_context(|| format!("failed to send newsletter issue to {}", subscriber.email))?;
    }
    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum PublishError {
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
    #[error("failed to send email: {0}")]
    SendEmailError(String),
}

impl IntoResponse for PublishError {
    fn into_response(self) -> axum::response::Response {
        let status = match self {
            Self::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::SendEmailError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };

        let mut response = status.into_response();
        let body = axum::body::boxed(super::error_chain_msg(&self).unwrap());
        *response.body_mut() = axum::body::boxed(body);
        response
    }
}

#[derive(Deserialize)]
pub struct BodyData {
    title: String,
    content: Content,
}

#[derive(Deserialize)]
struct Content {
    html: String,
    text: String,
}

struct ConfirmedSubscriber {
    email: String,
}

#[tracing::instrument(level = "info", name = "get confirmed subscribers", skip(db_pool))]
async fn get_confirmed_subscribers(
    db_pool: &PgPool,
) -> Result<Vec<ConfirmedSubscriber>, anyhow::Error> {
    let rows = sqlx::query_as!(
        ConfirmedSubscriber,
        r#"SELECT email FROM subscriptions WHERE status = 'confirmed'"#
    )
    .fetch_all(db_pool)
    .await?;
    Ok(rows)
}
