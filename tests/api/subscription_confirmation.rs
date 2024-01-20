use crate::helpers::{spawn_app, TestApp};
use reqwest::Response;
use wiremock::{
    matchers::{any, method},
    Mock, ResponseTemplate,
};

#[tokio::test]
async fn confirmations_without_token_are_rejected_with_a_400() {
    let test_app: TestApp = spawn_app().await;

    let response: Response = reqwest::get(format!("{}/subscriptions/confirm", test_app.address))
        .await
        .expect("Failedt to send request to the test app.");

    assert_eq!(400, response.status().as_u16());
}

#[tokio::test]
async fn the_link_returned_by_subscribe_returns_a_200_when_called() {
    let test_app = spawn_app().await;

    let body = String::from("name=le%20guin&email=ursula_le_guin%40gmail.com");

    Mock::given(any())
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&test_app.email_server)
        .await;

    test_app.post_subscription(body).await;

    let email_request = &test_app.email_server.received_requests().await.unwrap()[0];
    let confirmation_links = test_app.get_confirmation_link(email_request);
    let parsed_link = confirmation_links.html;

    let response = reqwest::get(parsed_link)
        .await
        .expect("Faield to follow the confirmation link.");

    assert_eq!(200, response.status().as_u16());
}

#[tokio::test]
async fn click_on_the_confirmation_link_confirms_a_subscriber() {
    let test_app = spawn_app().await;

    let body = String::from("name=le%20guin&email=ursula_le_guin%40gmail.com");

    Mock::given(any())
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&test_app.email_server)
        .await;

    test_app.post_subscription(body).await;

    let email_request = &test_app.email_server.received_requests().await.unwrap()[0];
    let confirmation_links = test_app.get_confirmation_link(email_request);
    let parsed_link = confirmation_links.html;

    reqwest::get(parsed_link)
        .await
        .expect("Faield to follow the confirmation link.");

    let saved = sqlx::query!("SELECT email, name, status FROM subscriptions",)
        .fetch_one(&test_app.connection_pool)
        .await
        .expect("Failed to fetch saved subscription.");

    assert_eq!(saved.email, "ursula_le_guin@gmail.com");
    assert_eq!(saved.name, "le guin");
    assert_eq!(saved.status, "confirmed");
}
