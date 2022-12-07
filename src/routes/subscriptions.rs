use axum::Form;
use hyper::StatusCode;
use serde::Deserialize;

pub async fn subscribe(Form(_form): Form<FormData>) -> StatusCode {
    StatusCode::OK
}

#[derive(Deserialize)]
pub struct FormData {
    email: String,
    name: String,
}
