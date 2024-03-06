use actix_web::{web, HttpResponse, ResponseError};
use anyhow::Context;
use reqwest::StatusCode;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(serde::Deserialize)]
pub struct Parameters {
    subscription_token: String,
}

#[tracing::instrument(name = "Confirming a pending subscription", skip_all)]
pub async fn subscription_confirm(
    parameters: web::Query<Parameters>,
    connection_pool: web::Data<PgPool>,
) -> Result<HttpResponse, ConfirmationError> {
    let id = get_subscriber_id(&connection_pool, &parameters.subscription_token)
        .await
        .context("No database connection.")?;

    match id {
        None => Err(ConfirmationError::NoRecordError(
            "Record does not exit in the database.".to_string(),
        )),
        Some(subscriber_id) => {
            update_status(&connection_pool, subscriber_id)
                .await
                .context("Failed to update confirmation status in the database.")?;
            Ok(HttpResponse::Ok().finish())
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfirmationError {
    #[error("Failed to confirm subscription. You need to subscrib to our newsletter first.")]
    NoRecordError(String),
    #[error("Weak Internet connection. Try again later.")]
    DatabaseError(#[from] anyhow::Error),
}

impl ResponseError for ConfirmationError {
    fn status_code(&self) -> reqwest::StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }
}

async fn get_subscriber_id(
    connection_pool: &PgPool,
    subscription_token: &str,
) -> Result<Option<Uuid>, sqlx::Error> {
    let result = sqlx::query!(
        "SELECT subscriber_id FROM subscription_tokens WHERE subscription_token = $1",
        subscription_token,
    )
    .fetch_optional(connection_pool)
    .await
    .map_err(|err| {
        tracing::error!("Failed to execute query: {:?}", err);
        err
    })?;

    Ok(result.map(|r| r.subscriber_id))
}

async fn update_status(connection_pool: &PgPool, subscriber_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE subscriptions SET status = 'confirmed' WHERE id = $1",
        subscriber_id
    )
    .execute(connection_pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;

    Ok(())
}
