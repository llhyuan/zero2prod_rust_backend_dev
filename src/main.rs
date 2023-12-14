use std::net::TcpListener;

use sqlx::PgPool;
use zero2prod::{
    configuration::get_configuration,
    startup::run,
    telemetry::{get_subscriber, init_subscriber},
};

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    // Instrumenting the app
    let subscriber = get_subscriber("zero2prod".into(), "info".into());
    init_subscriber(subscriber);

    // Config resources for the app: database connection and TCP listener
    let configurations = get_configuration().expect("Failed to read configuration.");

    let connection = PgPool::connect(&configurations.database.connection_string())
        .await
        .expect("Failed to connect to Postgres.");

    let addr_to_bind = format!("127.0.0.1:{}", configurations.application_port);

    let listener = TcpListener::bind(addr_to_bind).expect("Failed to bind random port.");

    // Startup the server.
    run(listener, connection)?.await
}
