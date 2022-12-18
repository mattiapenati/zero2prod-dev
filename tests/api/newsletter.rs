use http::{header, Request};
use serde_json::json;
use wiremock::{
    matchers::{any, method, path},
    Mock, ResponseTemplate,
};

use crate::helpers::{spawn_app, ConfirmationLinks, TestApp};

#[tokio::test]
async fn newsletters_are_not_delivered_to_uncontfirmed_subscribers() {
    let app = spawn_app().await;
    create_unconfirmed_subscriber(&app).await;

    Mock::given(any())
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&app.email_server)
        .await;

    let client = hyper::Client::new();
    let newsletter_request_body = json!({
        "title": "Newsletter title",
        "content": {
            "text": "Newsletter body as plain text",
            "html": "<p>Newsletter body as HTML</p>",
        }
    });
    let request = Request::post(format!("{}/newsletters", &app.address))
        .header(header::CONTENT_TYPE, "application/json")
        .body(
            serde_json::to_string(&newsletter_request_body)
                .unwrap()
                .into(),
        )
        .expect("failed to build request for sending newsletter");
    let response = client
        .request(request)
        .await
        .expect("failed to execute request");

    assert_eq!(200, response.status());
}

#[tokio::test]
async fn newsletters_are_delivered_to_contfirmed_subscribers() {
    let app = spawn_app().await;
    create_confirmed_subscriber(&app).await;

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    let client = hyper::Client::new();
    let newsletter_request_body = json!({
        "title": "Newsletter title",
        "content": {
            "text": "Newsletter body as plain text",
            "html": "<p>Newsletter body as HTML</p>",
        }
    });
    let request = Request::post(format!("{}/newsletters", &app.address))
        .header(header::CONTENT_TYPE, "application/json")
        .body(
            serde_json::to_string(&newsletter_request_body)
                .unwrap()
                .into(),
        )
        .expect("failed to build request for sending newsletter");
    let response = client
        .request(request)
        .await
        .expect("failed to execute request");

    assert_eq!(200, response.status());
}

async fn create_unconfirmed_subscriber(app: &TestApp) -> ConfirmationLinks {
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";
    let _guard = Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .named("create unconfirmed subscriber")
        .expect(1)
        .mount_as_scoped(&app.email_server)
        .await;

    let response = app.post_subscriptions(body.into()).await;
    assert!(response.status().is_success());

    let email_request = &app
        .email_server
        .received_requests()
        .await
        .unwrap()
        .pop()
        .unwrap();
    app.get_confirmation_links(&email_request)
}

async fn create_confirmed_subscriber(app: &TestApp) {
    let confirmation_links = create_unconfirmed_subscriber(app).await;
    let request = Request::get(confirmation_links.html)
        .body(Default::default())
        .expect("failed to build request for subscription confirmation");
    let response = hyper::Client::new()
        .request(request)
        .await
        .expect("failed to execute request");
    assert!(response.status().is_success());
}
