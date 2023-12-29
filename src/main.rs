use std::net::TcpListener;

use sqlx::postgres::PgPoolOptions;
use zero2prod::{
    configuration::get_configuration,
    email_clients::EmailClient,
    startup::run,
    telemetry::{get_subscriber, init_subscriber},
};

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    // Initialize the logger
    // On the application level
    let subscriber = get_subscriber("zero2prod".into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);

    let configurations = get_configuration().expect("Failed to read configuration.");

    let connection = PgPoolOptions::new().connect_lazy_with(configurations.database.with_db());

    let addr_to_bind = format!(
        "{}:{}",
        configurations.application.host, configurations.application.port
    );

    let listener = TcpListener::bind(addr_to_bind).expect("Failed to bind random port.");

    let sender_email = configurations
        .email_client
        .sender()
        .expect("Invalide sender email.");

    let auth_token = configurations.email_client.auth_token;

    let email_client = EmailClient::new(
        configurations.email_client.base_url,
        sender_email,
        auth_token,
    );

    run(listener, connection, email_client)?.await
}
