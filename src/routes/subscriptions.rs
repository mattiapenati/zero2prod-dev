use axum::{extract::State, Form};
use chrono::Utc;
use hyper::StatusCode;
use rand::{distributions::Alphanumeric, Rng};
use serde::Deserialize;
use sqlx::PgPool;
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
) -> Result<(), StatusCode> {
    let new_subscriber = form
        .try_into()
        .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;

    let subscriber_id = insert_subscriber(&db_pool, &new_subscriber)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let subscription_token = generate_subscription_token();
    store_token(&db_pool, subscriber_id, &subscription_token)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    send_confirmation_email(&email_client, new_subscriber, base_url, &subscription_token)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(())
}

#[tracing::instrument(
    level = "info",
    name = "saving new subscriber details in the database",
    skip(db_pool, new_subscriber)
)]
async fn insert_subscriber(db_pool: &PgPool, new_subscriber: &NewSubscriber) -> sqlx::Result<Uuid> {
    let subscriber_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO subscriptions (id, email, name, subscribed_at, status)
        VALUES ($1, $2, $3, $4, 'pending_confirmation')",
        subscriber_id,
        new_subscriber.email.as_ref(),
        new_subscriber.name.as_ref(),
        Utc::now(),
    )
    .execute(db_pool)
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
    skip(db_pool, subscription_token)
)]
async fn store_token(
    db_pool: &PgPool,
    subscriber_id: Uuid,
    subscription_token: &str,
) -> sqlx::Result<()> {
    sqlx::query!(
        r#"INSERT INTO subscription_tokens (subscription_token, subscriber_id)
        VALUES ($1, $2)"#,
        subscription_token,
        subscriber_id
    )
    .execute(db_pool)
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
