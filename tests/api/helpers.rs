use linkify::LinkFinder;
use once_cell::sync::Lazy;
use reqwest::Response;
use reqwest::Url;
use sqlx::{ConnectOptions, Executor, PgPool};
use uuid::Uuid;
use wiremock::{MockServer, Request};
use zero2prod::configuration::get_configuration;
use zero2prod::configuration::DatabaseSettings;
use zero2prod::startup::Application;
use zero2prod::telemetry::get_subscriber;
use zero2prod::telemetry::init_subscriber;

pub struct TestApp {
    pub address: String,
    pub port: u16,
    pub connection_pool: PgPool,
    pub email_server: MockServer,
}

pub struct ConfirmationLinks {
    pub html: reqwest::Url,
    pub plain_text: reqwest::Url,
}

impl TestApp {
    pub async fn post_subscription(&self, body: String) -> Response {
        let client = reqwest::Client::new();
        client
            .post(format!("{}/subscriptions", self.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("Failed to send subscription request.")
    }

    pub fn get_confirmation_link(&self, email_request: &Request) -> ConfirmationLinks {
        let request_body: serde_json::Value = serde_json::from_slice(&email_request.body).unwrap();

        fn get_link(s: &str) -> String {
            let links: Vec<_> = LinkFinder::new()
                .links(s)
                .filter(|link| *link.kind() == linkify::LinkKind::Url)
                .collect();
            links[0].as_str().to_string()
        }

        let confirmation_link = get_link(request_body["HtmlBody"].as_str().unwrap());
        let mut parsed_link = Url::parse(&confirmation_link).unwrap();

        parsed_link.set_port(Some(self.port)).unwrap();

        ConfirmationLinks {
            html: parsed_link.clone(),
            plain_text: parsed_link,
        }
    }
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

    let mut configurations = get_configuration().expect("Failed to read configuration");

    configurations.database.database_name = Uuid::new_v4().to_string();
    configurations.application.host = String::from("127.0.0.1");
    configurations.application.port = 0;

    let email_server = MockServer::start().await;
    configurations.email_client.base_url = email_server.uri();

    let connection_pool = configure_database(&configurations.database).await;

    let application = Application::build(configurations)
        .await
        .expect("Failed to build server application");
    let application_port = application.port();
    let addr = format!("127.0.0.1:{}", &application_port);

    tokio::spawn(application.run_until_stopped());

    // After spawning up a new instance,
    // only its address and port number are needed
    // to access/send requests to it.
    TestApp {
        address: format!("http://{}", addr),
        port: application_port,
        connection_pool,
        email_server,
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
