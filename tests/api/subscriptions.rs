use linkify::LinkFinder;
use wiremock::{
    matchers::{any, method},
    Mock, ResponseTemplate,
};

use crate::helpers::spawn_app;

#[tokio::test]
async fn subscribe_returns_a_400_when_data_is_missing() {
    let test_app = spawn_app().await;

    let test_cases = vec![
        ("name=le%20guin", "missing the email"),
        ("email=ursula_le_guin%40gmail.com", "missing the name"),
        ("", "missing both name and email"),
    ];

    for (invalid_body, error_msg) in test_cases {
        let response = test_app.post_subscription(invalid_body.to_string()).await;

        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not fail with 400 Bad Request when the payload was {}",
            error_msg
        );
    }
}

#[tokio::test]
async fn subscribe_returns_a_200_for_valid_form_data() {
    let test_app = spawn_app().await;

    let body = String::from("name=le%20guin&email=ursula_le_guin%40gmail.com");

    Mock::given(any())
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&test_app.email_server)
        .await;

    let response = test_app.post_subscription(body).await;

    assert_eq!(200, response.status().as_u16());

    let saved = sqlx::query!("SELECT email, name FROM subscriptions",)
        .fetch_one(&test_app.connection_pool)
        .await
        .expect("Failed to fetch saved subscription.");

    assert_eq!(saved.email, "ursula_le_guin@gmail.com");
    assert_eq!(saved.name, "le guin");
}

#[tokio::test]
async fn subsctibe_returns_a_400_when_fields_are_present_but_invalid() {
    let test_app = spawn_app().await;

    let test_cases = vec![
        ("name=&email=ursula_le_guin%40gmail.com", "empty name"),
        ("name=Ursula&email=", "empty email"),
        ("name=Ursula&email=definitely-not-an-email", "invalid email"),
    ];

    for (body, description) in test_cases {
        let response = test_app.post_subscription(body.to_string()).await;

        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not return a 400 OK when the payload was {}",
            description
        );
    }
}

#[tokio::test]
async fn subscribe_sends_a_confirmation_email_for_valid_data() {
    let test_app = spawn_app().await;

    let body = String::from("name=le%20guin&email=ursula_le_guin%40gmail.com");
    Mock::given(any())
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&test_app.email_server)
        .await;

    let response = test_app.post_subscription(body).await;

    assert_eq!(200, response.status().as_u16());
}

#[tokio::test]
async fn subscribe_sneds_a_confirmation_email_with_a_link() {
    let test_app = spawn_app().await;

    let body = String::from("name=le%20guin&email=ursula_le_guin%40gmail.com");
    Mock::given(any())
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&test_app.email_server)
        .await;

    let _ = test_app.post_subscription(body).await;

    let email_request = &test_app.email_server.received_requests().await.unwrap()[0];
    let request_body: serde_json::Value = serde_json::from_slice(&email_request.body).unwrap();

    fn get_link(s: &str) -> String {
        let links: Vec<_> = LinkFinder::new().links(s).collect();
        println!("{:?}", links);
        links[0].as_str().to_string()
    }
    println!("{:?}", request_body);

    let html_link = get_link(request_body["HtmlBody"].as_str().unwrap());
    let text_link = get_link(request_body["TextBody"].as_str().unwrap());

    assert_eq!(html_link, text_link);
}
