use actix_web::{web, HttpResponse};
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(serde::Deserialize)]
pub struct FormDataSubscriber {
    email: String,
    name: String,
}

pub async fn subsribe(
    form: web::Form<FormDataSubscriber>,
    connection_pool: web::Data<PgPool>,
) -> HttpResponse {
    let request_id = Uuid::new_v4();
    log::info!(
        "Request_id:{} - Adding email:{}, name:{} as a new subscriber.",
        request_id,
        form.name,
        form.email
    );

    log::info!(
        "Request_id: {} - Saving new subscriber details in the database",
        request_id
    );
    match sqlx::query!(
        r#"
    INSERT INTO subscriptions (id, email, name, subscribed_at)
    VALUES ($1, $2, $3, $4)"#,
        Uuid::new_v4(),
        form.email,
        form.name,
        Utc::now(),
    )
    .execute(connection_pool.get_ref())
    .await
    {
        Ok(_) => {
            log::info!(
                "Request_id: {} - New subscriber details have been saved",
                request_id
            );
            HttpResponse::Ok().finish()
        }
        Err(error_msg) => {
            log::error!(
                "Request_id: {} - Failed to execute query: {:?}",
                request_id,
                error_msg
            );
            HttpResponse::InternalServerError().finish()
        }
    }
}
