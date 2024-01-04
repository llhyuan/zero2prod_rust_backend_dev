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

    let timeout = configurations.email_client.timeout();
    let email_client = EmailClient::new(
        configurations.email_client.base_url,
        sender_email,
        configurations.email_client.auth_token,
        timeout,
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
