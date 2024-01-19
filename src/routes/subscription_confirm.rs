use actix_web::{web, HttpResponse};

#[derive(serde::Deserialize)]
pub struct Parameters {
    subscription_token: String,
}

#[tracing::instrument(name = "Confirming a pending subscription", skip_all)]
pub async fn subscription_confirm(parameters: web::Query<Parameters>) -> HttpResponse {
    println!("{}", parameters.subscription_token);
    HttpResponse::Ok().finish()
}

