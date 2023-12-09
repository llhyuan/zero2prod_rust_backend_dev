use std::net::TcpListener;

use sqlx::PgPool;
use zero2prod::{configuration::get_configuration, startup::run};

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let configurations = get_configuration().expect("Failed to read configuration.");

    let connection = PgPool::connect(&configurations.database.connection_string())
        .await
        .expect("Failed to connect to Postgres.");

    let addr_to_bind = format!("127.0.0.1:{}", configurations.application_port);

    let listener = TcpListener::bind(addr_to_bind).expect("Failed to bind random port.");

    run(listener, connection)?.await
}
