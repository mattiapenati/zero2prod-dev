use anyhow::Context;
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use axum::{extract::State, response::IntoResponse, Json};
use http::{header, HeaderMap, HeaderValue, StatusCode};
use secrecy::{ExposeSecret, Secret};
use serde::Deserialize;
use sqlx::PgPool;

use crate::{
    domain::SubscriberEmail, email_client::EmailClient, trace::spawn_blocking_with_tracing,
};

#[tracing::instrument(
    level = "info",
    name = "publish a newsletter issue",
    skip(db_pool, email_client, body)
)]
pub async fn publish_newsletter(
    State(db_pool): State<PgPool>,
    State(email_client): State<EmailClient>,
    headers: HeaderMap,
    Json(body): Json<BodyData>,
) -> Result<(), PublishError> {
    let credentials = basic_authentication(&headers).map_err(PublishError::AuthError)?;
    tracing::Span::current().record("username", &tracing::field::display(&credentials.username));
    let user_id = validate_credentials(credentials, &db_pool).await?;
    tracing::Span::current().record("user_id", &tracing::field::display(&user_id));

    let subscribers = get_confirmed_subscribers(&db_pool).await.context("")?;
    for subscriber in subscribers {
        match subscriber {
            Ok(subscriber) => {
                email_client
                    .send_email(
                        &subscriber.email,
                        &body.title,
                        &body.content.html,
                        &body.content.text,
                    )
                    .await
                    .map_err(PublishError::SendEmailError)
                    .with_context(|| {
                        format!("failed to send newsletter issue to {}", subscriber.email)
                    })?;
            }
            Err(err) => {
                tracing::warn!(
                    error.cause_chain = ?err,
                    "skipping a confirmed subscriber, \
                    their stores contact details are invalid"
                );
            }
        }
    }
    Ok(())
}

#[derive(Debug, thiserror::Error)]
#[allow(clippy::enum_variant_names)]
pub enum PublishError {
    #[error("authentication failed")]
    AuthError(#[source] anyhow::Error),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
    #[error("failed to send email: {0}")]
    SendEmailError(String),
}

impl IntoResponse for PublishError {
    fn into_response(self) -> axum::response::Response {
        let status = match self {
            Self::AuthError(_) => StatusCode::UNAUTHORIZED,
            Self::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::SendEmailError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };

        let mut response = status.into_response();
        let body = axum::body::boxed(super::error_chain_msg(&self).unwrap());
        *response.body_mut() = axum::body::boxed(body);
        if let Self::AuthError(_) = self {
            let header_value = HeaderValue::from_static(r#"Basic realm="publish""#);
            response
                .headers_mut()
                .insert(header::WWW_AUTHENTICATE, header_value);
        }
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
    email: SubscriberEmail,
}

#[tracing::instrument(level = "info", name = "get confirmed subscribers", skip(db_pool))]
async fn get_confirmed_subscribers(
    db_pool: &PgPool,
) -> anyhow::Result<Vec<anyhow::Result<ConfirmedSubscriber>>> {
    let rows = sqlx::query!(r#"SELECT email FROM subscriptions WHERE status = 'confirmed'"#)
        .fetch_all(db_pool)
        .await?
        .into_iter()
        .map(|row| match SubscriberEmail::parse(row.email) {
            Ok(email) => Ok(ConfirmedSubscriber { email }),
            Err(err) => Err(anyhow::anyhow!(err)),
        })
        .collect();
    Ok(rows)
}

struct Credentials {
    username: String,
    password: Secret<String>,
}

fn basic_authentication(headers: &HeaderMap) -> anyhow::Result<Credentials> {
    let header_value = headers
        .get(header::AUTHORIZATION)
        .context("the 'authorization' header was missing")?
        .to_str()
        .context("the 'authorization' header was not a valid utf8 string")?;
    let base64encoded_segment = header_value
        .strip_prefix("Basic ")
        .context("the authorization scheme was not 'Basic'")?;
    let decoded_bytes = base64::decode(base64encoded_segment)
        .context("failed to base64-decode 'Basic' credentials")?;
    let decoded_credentials = String::from_utf8(decoded_bytes)
        .context("the decoded credential string is not valid utf8")?;

    let mut credentials = decoded_credentials.splitn(2, ':');
    let username = credentials
        .next()
        .ok_or_else(|| anyhow::anyhow!("a username must be provided in 'Basic' authentication"))?
        .to_string();
    let password = credentials
        .next()
        .ok_or_else(|| anyhow::anyhow!("a password must be provided in 'Basic' authentication"))?
        .to_string();

    Ok(Credentials {
        username,
        password: Secret::new(password),
    })
}

#[tracing::instrument(
    level = "info",
    name = "validate credentials",
    skip(credentials, db_pool)
)]
async fn validate_credentials(
    credentials: Credentials,
    db_pool: &PgPool,
) -> Result<uuid::Uuid, PublishError> {
    let mut user_id = None;
    let mut expected_password_hash = Secret::new("$argon2id$v=19$m=15000,t=2,p=1$gZiV/M1gPc22ElAH/Jh1Hw$CWOrkoo7oJBQ/iyh7uJ0LO2aLEfrHwTWllSAxT0zRno".to_string());

    if let Some((stored_user_id, stored_expected_password_hash)) =
        get_stored_credentials(&credentials.username, db_pool)
            .await
            .map_err(PublishError::UnexpectedError)?
    {
        user_id = Some(stored_user_id);
        expected_password_hash = stored_expected_password_hash;
    }

    spawn_blocking_with_tracing(move || {
        verify_password_hash(expected_password_hash, credentials.password)
    })
    .await
    .context("failed to spawn blocking task")
    .map_err(PublishError::UnexpectedError)??;

    user_id.ok_or_else(|| PublishError::AuthError(anyhow::anyhow!("unknown username")))
}

#[tracing::instrument(level = "info", name = "get stored credentials", skip_all)]
async fn get_stored_credentials(
    username: &str,
    db_pool: &PgPool,
) -> anyhow::Result<Option<(uuid::Uuid, Secret<String>)>> {
    let row = sqlx::query!(
        "SELECT user_id, password_hash FROM users where username = $1",
        username,
    )
    .fetch_optional(db_pool)
    .await
    .context("failed to perform a query to retrieve stored credentials")?
    .map(|row| (row.user_id, Secret::new(row.password_hash)));
    Ok(row)
}

#[tracing::instrument(level = "info", name = "verify password hash", skip_all)]
fn verify_password_hash(
    expected_password_hash: Secret<String>,
    password_candidate: Secret<String>,
) -> Result<(), PublishError> {
    let expected_password_hash = PasswordHash::new(expected_password_hash.expose_secret())
        .context("failed to parse hash in PHC string format")
        .map_err(PublishError::UnexpectedError)?;

    Argon2::default()
        .verify_password(
            password_candidate.expose_secret().as_bytes(),
            &expected_password_hash,
        )
        .context("invalid password")
        .map_err(PublishError::AuthError)?;

    Ok(())
}
