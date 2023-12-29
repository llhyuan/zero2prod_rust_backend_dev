use once_cell::sync::Lazy;
use sqlx::{ConnectOptions, Executor, PgPool};
use std::net::TcpListener;
use uuid::Uuid;
use zero2prod::configuration::get_configuration;
use zero2prod::configuration::DatabaseSettings;
use zero2prod::email_clients::EmailClient;
use zero2prod::startup::run;
use zero2prod::telemetry::get_subscriber;
use zero2prod::telemetry::init_subscriber;

use reqwest::Client;

pub struct TestApp {
    pub address: String,
    pub connection_pool: PgPool,
}

static TRACING: Lazy<()> = Lazy::new(|| {
    let default_filter_level = "info".to_string();
    let subscriber_name = "zero2prod_test".to_string();
    // set up logging for test app
    if std::env::var("TEST_LOG").is_ok() {
        let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::stdout);
        init_subscriber(subscriber);
    } else {
        // use std::io::sink to consume the log data silently
        // ie. send them into void
        let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::sink);
        init_subscriber(subscriber);
    };
});

pub async fn spawn_app() -> TestApp {
    Lazy::force(&TRACING);
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind random port.");
    let addr = listener.local_addr().unwrap();

    let mut configurations = get_configuration().expect("Failed to read configuration");

    configurations.database.database_name = Uuid::new_v4().to_string();

    let connection_pool = configure_database(&configurations.database).await;

    let sender_email = configurations
        .email_client
        .sender()
        .expect("Invalide sender email.");

    let email_client = EmailClient::new(
        configurations.email_client.base_url,
        sender_email,
        configurations.email_client.auth_token,
    );

    let server = run(listener, connection_pool.clone(), email_client)
        .expect("Failed to fireup server for test.");

    tokio::spawn(server);
    TestApp {
        address: format!("http://{addr}"),
        connection_pool,
    }
}

pub async fn configure_database(config: &DatabaseSettings) -> PgPool {
    // First, connect to the admin database - postgres
    let mut connection = config
        .without_db()
        .connect()
        .await
        .expect("Fail to connect to test database.");

    // Then, use the established connection to create a new database for testing.
    // Here, the query result is disgarded.
    connection
        .execute(format!(r#"CREATE DATABASE "{}";"#, config.database_name).as_str())
        .await
        .expect("Failed to create test database.");

    // Then, establish a connection pool to the newly created test database.
    let connection_pool = PgPool::connect_with(config.with_db())
        .await
        .expect("Failed to connect to test database.");

    // Run database migration operations to populate the database with test data.
    sqlx::migrate!("./migrations")
        .run(&connection_pool)
        .await
        .expect("Failed to migrate test database.");

    connection_pool
}

#[tokio::test]
async fn health_check_works() {
    let test_app = spawn_app().await;

    let client = Client::new();

    let response = client
        .get(format!("{}/health_check", &test_app.address))
        .send()
        .await
        .expect("Request failed.");

    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}

#[tokio::test]
async fn subscribe_returns_a_400_when_data_is_missing() {
    let test_app = spawn_app().await;
    let client = Client::new();

    let test_cases = vec![
        ("name=le%20guin", "missing the email"),
        ("email=ursula_le_guin%40gmail.com", "missing the name"),
        ("", "missing both name and email"),
    ];

    for (invalid_body, error_msg) in test_cases {
        let response = client
            .post(format!("{}/subscriptions", &test_app.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(invalid_body)
            .send()
            .await
            .expect("Failed to send subscription form test data.");

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

    let client = reqwest::Client::new();
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    let response = client
        .post(format!("{}/subscriptions", &test_app.address))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("Failed to send subscription request.");

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

    let client = reqwest::Client::new();
    let test_cases = vec![
        ("name=&email=ursula_le_guin%40gmail.com", "empty name"),
        ("name=Ursula&email=", "empty email"),
        ("name=Ursula&email=definitely-not-an-email", "invalid email"),
    ];

    for (body, description) in test_cases {
        let response = client
            .post(format!("{}/subscriptions", &test_app.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("Failed to send subscription request.");

        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not return a 400 OK when the payload was {}",
            description
        );
    }
}
