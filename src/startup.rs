use crate::{
    configuration::{DatabaseSettings, Settings},
    email_clients::EmailClient,
    routes::subscription::subsribe,
};
use std::net::TcpListener;

use actix_web::{dev::Server, web, App, HttpRequest, HttpResponse, HttpServer, Responder};
use sqlx::{postgres::PgPoolOptions, PgPool};
use tracing_actix_web::TracingLogger;

pub fn run(
    listener: TcpListener,
    db_pool: PgPool,
    email_client: EmailClient,
) -> Result<Server, std::io::Error> {
    let connection_pool = web::Data::new(db_pool);
    let email_client = web::Data::new(email_client);

    // actix_web will create one server for each CPU core.
    // Wrapping shared data in web::Data, which is an arc<T> pointer,
    // making memory usage more efficient.
    let server = HttpServer::new(move || {
        // Pattern matching against the path happens in the order
        // in which the routes are registered in the app.
        App::new()
            // Tracing on the server level
            // The trace logger is monitoring incoming requests,
            // creating a seperate logging span for each request.
            // The default tracing logger will automaticaly
            // create an id for each request on request start.
            .wrap(TracingLogger::default())
            .route("/", web::get().to(greet))
            .route("/health_check", web::get().to(health_check))
            .route("/subscriptions", web::post().to(subsribe))
            .route("/{name}", web::get().to(greet))
            .app_data(connection_pool.clone())
            .app_data(email_client.clone())
    })
    .listen(listener)?
    .run();

    Ok(server)
}

pub fn get_connection_pool(configuration: &DatabaseSettings) -> PgPool {
    PgPoolOptions::new().connect_lazy_with(configuration.with_db())
}

async fn greet(req: HttpRequest) -> impl Responder {
    let name = req.match_info().get("name").unwrap_or("World");
    format!("Hello {}!", name)
}

async fn health_check(_req: HttpRequest) -> impl Responder {
    HttpResponse::Ok().finish()
}

pub struct Application {
    port: u16,
    server: Server,
}

impl Application {
    pub async fn build(configurations: Settings) -> Result<Self, std::io::Error> {
        let connection = get_connection_pool(&configurations.database);

        let addr_to_bind = format!(
            "{}:{}",
            configurations.application.host, configurations.application.port
        );

        let listener = TcpListener::bind(addr_to_bind).expect("Failed to bind random port.");

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

        Ok(Self {
            port: listener.local_addr()?.port(),
            server: run(listener, connection, email_client)?,
        })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub async fn run_until_stopped(self) -> Result<(), std::io::Error> {
        self.server.await
    }
}
