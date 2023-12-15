use crate::routes::subscription::subsribe;
use std::net::TcpListener;

use actix_web::{dev::Server, web, App, HttpRequest, HttpResponse, HttpServer, Responder};
use sqlx::PgPool;
use tracing_actix_web::TracingLogger;

pub fn run(listener: TcpListener, db_pool: PgPool) -> Result<Server, std::io::Error> {
    let connection_pool = web::Data::new(db_pool);
    let server = HttpServer::new(move || {
        // Pattern matching against the path happens in the order
        // in which the routes are registered in the app.
        App::new()
            // The default tracing logger will automaticaly
            // create an id for each request on request start.
            .wrap(TracingLogger::default())
            .route("/", web::get().to(greet))
            .route("/health_check", web::get().to(health_check))
            .route("/subscriptions", web::post().to(subsribe))
            .route("/{name}", web::get().to(greet))
            .app_data(connection_pool.clone())
    })
    .listen(listener)?
    .run();

    Ok(server)
}

async fn greet(req: HttpRequest) -> impl Responder {
    let name = req.match_info().get("name").unwrap_or("World");
    format!("Hello {}!", name)
}

async fn health_check(_req: HttpRequest) -> impl Responder {
    HttpResponse::Ok().finish()
}
