use actix_web::{web, HttpResponse};
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(serde::Deserialize)]
pub struct FormDataSubscriber {
    email: String,
    name: String,
}

#[tracing::instrument(name="Adding a new subscriber", skip(form, connection_pool), fields(subscriber_email=%form.email, subscriber_name=%form.name))]
// The web::Form<> and web::Data annotations are telling the framework
// what to extract from the http request.
// After extraction, form and connection_pool will be of the type annotated inside the <>
pub async fn subsribe(
    form: web::Form<FormDataSubscriber>,
    connection_pool: web::Data<PgPool>,
) -> HttpResponse {
    match insert_subscriber(&form, &connection_pool).await {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

#[tracing::instrument(
    name = "Saving new subscriber details in the database",
    skip(form, connection_pool)
)]
async fn insert_subscriber(
    form: &FormDataSubscriber,
    connection_pool: &PgPool,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
    INSERT INTO subscriptions (id, email, name, subscribed_at)
    VALUES ($1, $2, $3, $4)"#,
        Uuid::new_v4(),
        form.email,
        form.name,
        Utc::now(),
    )
    .execute(connection_pool)
    .await
    .map_err(|err| {
        // trace the query error without changing the flow
        tracing::error!("Failed to execute query: {:?}", err);
        err
    })?;
    Ok(())
}
