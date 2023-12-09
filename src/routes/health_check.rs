use crate::configuration::get_configuration;
use crate::startup::run;
use sqlx::{Connection, PgConnection};
use std::net::TcpListener;

#[tokio::test]
async fn subscribe_returns_a_200_for_valid_form_data() {
    let app_addr = spawn_app();
    let configuration = get_configuration().expect("Failed to read configuration");
    let connection_string = configuration.database.connection_string();

    let _connection = PgConnection::connect(&connection_string)
        .await
        .expect("Failed to connect to the Postgres.");
    let client = reqwest::Client::new();

    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";
    let response = client
        .post(format!("{}/subscriptions", app_addr))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("Failed to execute request.");

    assert_eq!(200, response.status().as_u16());
}

fn spawn_app() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind random port.");
    let addr = listener.local_addr().unwrap();
    let server = run(listener).expect("Failed to fireup server for test.");

    tokio::spawn(server);
    format!("http://{addr}")
}
