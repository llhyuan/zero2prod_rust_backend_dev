use reqwest::Client;
use std::net::TcpListener;

#[tokio::test]
async fn health_check_works() {
    let addr = spawn_app();

    let client = Client::new();

    let response = client
        .get(format!("{addr}/health_check"))
        .send()
        .await
        .expect("Request failed.");

    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}

fn spawn_app() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind random port.");
    let addr = listener.local_addr().unwrap();
    let server = zero2prod::run(listener).expect("Failed to fireup server for test.");

    tokio::spawn(server);
    format!("http://{addr}")
}
