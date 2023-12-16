use std::net::TcpListener;

use secrecy::ExposeSecret;
use sqlx::PgPool;
use zero2prod::{
    configuration::get_configuration,
    startup::run,
    telemetry::{get_subscriber, init_subscriber},
};

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    // Initialize the logger
    let subscriber = get_subscriber("zero2prod".into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);

    let configurations = get_configuration().expect("Failed to read configuration.");

    let connection =
        PgPool::connect_lazy(configurations.database.connection_string().expose_secret())
            .expect("Failed to connect to Postgres.");

    let addr_to_bind = format!(
        "{}:{}",
        configurations.application.host, configurations.application.port
    );

    let listener = TcpListener::bind(addr_to_bind).expect("Failed to bind random port.");

    run(listener, connection)?.await
}
