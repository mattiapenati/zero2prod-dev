use http::Request;
use wiremock::{
    matchers::{method, path},
    Mock, ResponseTemplate,
};

use crate::helpers::spawn_app;

#[tokio::test]
async fn confirmations_without_token_are_rejected_with_a_400() {
    let app = spawn_app().await;

    let client = hyper::Client::new();
    let request = Request::get(format!("{}/subscriptions/confirm", app.address))
        .body(Default::default())
        .unwrap();
    let response = client
        .request(request)
        .await
        .expect("failed to execute request");

    assert_eq!(400, response.status());
}

#[tokio::test]
async fn the_link_returned_by_subscribe_returns_a_200_if_called() {
    let app = spawn_app().await;

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";
    let _ = app.post_subscriptions(body.into()).await;

    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    let confirmation_links = app.get_confirmation_links(&email_request);

    let client = hyper::Client::new();
    let request = Request::get(confirmation_links.html)
        .body(Default::default())
        .unwrap();
    let response = client
        .request(request)
        .await
        .expect("failed to execute request");

    assert_eq!(200, response.status());
}

#[tokio::test]
async fn clicking_on_the_confirmation_link_confirms_a_subscriber() {
    let app = spawn_app().await;

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";
    let _ = app.post_subscriptions(body.into()).await;

    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    let confirmation_links = app.get_confirmation_links(&email_request);

    let client = hyper::Client::new();
    let request = Request::get(confirmation_links.html)
        .body(Default::default())
        .unwrap();
    let response = client
        .request(request)
        .await
        .expect("failed to execute request");
    assert!(response.status().is_success());
}
