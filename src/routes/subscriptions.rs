use anyhow::Context;
use axum::{
    extract::State,
    response::{IntoResponse, Response},
    Form,
};
use chrono::Utc;
use hyper::StatusCode;
use rand::{distributions::Alphanumeric, Rng};
use serde::Deserialize;
use sqlx::{PgExecutor, PgPool};
use uuid::Uuid;

use crate::{
    app::BaseUrl,
    domain::{NewSubscriber, SubscriberEmail, SubscriberName},
    email_client::EmailClient,
};

#[tracing::instrument(
    level = "info",
    name = "adding a new subscriber",
    skip(db_pool, email_client, base_url, form),
    fields(
        subscriber_email = %form.email,
        subscriber_name = %form.name,
    ),
)]
pub async fn subscribe(
    State(db_pool): State<PgPool>,
    State(email_client): State<EmailClient>,
    State(base_url): State<BaseUrl>,
    Form(form): Form<FormData>,
) -> Result<(), SubscribeError> {
    let new_subscriber = form.try_into().map_err(SubscribeError::ValidationError)?;

    let mut transaction = db_pool
        .begin()
        .await
        .context("failed to acquire a Postgres connection from the pool")?;

    let subscriber_id = insert_subscriber(&mut transaction, &new_subscriber)
        .await
        .context("failed to insert new subscriber in the database")?;
    let subscription_token = generate_subscription_token();
    store_token(&mut transaction, subscriber_id, &subscription_token)
        .await
        .context("failed to store the confirmation token for a new subscriber")?;

    transaction
        .commit()
        .await
        .context("failed to commit SQL transaction to store a new subscriber")?;

    send_confirmation_email(&email_client, new_subscriber, base_url, &subscription_token)
        .await
        .map_err(SubscribeError::SendEmailError)?;

    Ok(())
}

#[tracing::instrument(
    level = "info",
    name = "saving new subscriber details in the database",
    skip(executor, new_subscriber)
)]
async fn insert_subscriber<'c, E>(executor: E, new_subscriber: &NewSubscriber) -> sqlx::Result<Uuid>
where
    E: PgExecutor<'c>,
{
    let subscriber_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO subscriptions (id, email, name, subscribed_at, status)
        VALUES ($1, $2, $3, $4, 'pending_confirmation')",
        subscriber_id,
        new_subscriber.email.as_ref(),
        new_subscriber.name.as_ref(),
        Utc::now(),
    )
    .execute(executor)
    .await
    .map_err(|err| {
        tracing::error!("failed to execute query: {:?}", err);
        err
    })?;
    Ok(subscriber_id)
}

#[tracing::instrument(
    level = "info",
    name = "send a confirmation email to a new subscriber"
    skip(email_client, new_subscriber, base_url)
)]
async fn send_confirmation_email(
    email_client: &EmailClient,
    new_subscriber: NewSubscriber,
    base_url: BaseUrl,
    subscription_token: &str,
) -> Result<(), String> {
    let confirmation_link = format!(
        "{}/subscriptions/confirm?subscription_token={}",
        base_url, subscription_token,
    );
    let html_body = format!(
        r#"Welcome to our newsletter!<br />Click <a href="{}">here</a> to confirm your subscription."#,
        confirmation_link
    );
    let text_body = format!(
        r#"Welcome to our newsletter!\nVisit {} to confirm your subscription."#,
        confirmation_link
    );

    email_client
        .send_email(new_subscriber.email, "Welcome!", &html_body, &text_body)
        .await
}

#[tracing::instrument(
    level = "info",
    name = "store subscription token in the database",
    skip(executor, subscription_token)
)]
async fn store_token<'c, E>(
    executor: E,
    subscriber_id: Uuid,
    subscription_token: &str,
) -> sqlx::Result<()>
where
    E: PgExecutor<'c>,
{
    sqlx::query!(
        r#"INSERT INTO subscription_tokens (subscription_token, subscriber_id)
        VALUES ($1, $2)"#,
        subscription_token,
        subscriber_id
    )
    .execute(executor)
    .await
    .map_err(|err| {
        tracing::error!("failed to execute query: {:?}", err);
        err
    })?;
    Ok(())
}

fn generate_subscription_token() -> String {
    rand::thread_rng()
        .sample_iter(Alphanumeric)
        .take(25)
        .map(char::from)
        .collect()
}

#[derive(Deserialize)]
pub struct FormData {
    email: String,
    name: String,
}

impl TryFrom<FormData> for NewSubscriber {
    type Error = String;

    fn try_from(value: FormData) -> Result<Self, Self::Error> {
        let name = SubscriberName::parse(value.name)?;
        let email = SubscriberEmail::parse(value.email)?;
        Ok(Self { name, email })
    }
}

#[derive(Debug, thiserror::Error)]
#[allow(clippy::enum_variant_names)]
pub enum SubscribeError {
    #[error("{0}")]
    ValidationError(String),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
    #[error("failed to send a confirmation email: {0}")]
    SendEmailError(String),
}

impl IntoResponse for SubscribeError {
    fn into_response(self) -> Response {
        let status = match self {
            Self::ValidationError(_) => StatusCode::UNPROCESSABLE_ENTITY,
            Self::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };

        let mut response = status.into_response();
        let body = axum::body::boxed(super::error_chain_msg(&self).unwrap());
        *response.body_mut() = axum::body::boxed(body);
        response
    }
}
