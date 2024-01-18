use actix_web::{web, HttpResponse};
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    domain::{FormDataSubscriber, NewSubscriber, SubscriberEmail, SubscriberName},
    email_clients::EmailClient,
};

#[tracing::instrument(name="Adding a new subscriber", skip(form, connection_pool, email_client), fields(subscriber_email=%form.email, subscriber_name=%form.name))]
// The web::Form<> and web::Data annotations are telling the framework
// what to extract from the http request.
// After extraction, form and connection_pool will be of the type annotated inside the <>
pub async fn subsribe(
    form: web::Form<FormDataSubscriber>,
    connection_pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
) -> HttpResponse {
    let form: FormDataSubscriber = form.into_inner();

    let new_subscriber = match NewSubscriber::try_from(form) {
        Ok(subscriber) => subscriber,
        Err(_) => return HttpResponse::BadRequest().finish(),
    };
    if insert_subscriber(&new_subscriber, &connection_pool)
        .await
        .is_err()
    {
        return HttpResponse::InternalServerError().finish();
    }

    if send_confirmatioin_email(&email_client, new_subscriber)
        .await
        .is_err()
    {
        return HttpResponse::InternalServerError().finish();
    }

    HttpResponse::Ok().finish()
}

#[tracing::instrument(
    name = "Saving new subscriber details in the database",
    skip(new_subscriber, connection_pool)
)]
async fn insert_subscriber(
    new_subscriber: &NewSubscriber,
    connection_pool: &PgPool,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
    INSERT INTO subscriptions (id, email, name, subscribed_at, status)
    VALUES ($1, $2, $3, $4, $5)"#,
        Uuid::new_v4(),
        new_subscriber.email.as_ref(),
        new_subscriber.name.as_ref(),
        Utc::now(),
        "confirmed"
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

pub fn parse_subscriber(form: FormDataSubscriber) -> Result<NewSubscriber, String> {
    let name = SubscriberName::parse(form.name)?;
    let email = SubscriberEmail::parse(form.email)?;
    Ok(NewSubscriber { name, email })
}

#[tracing::instrument(name = "Sending confirmation email.", skip_all)]
pub async fn send_confirmatioin_email(
    email_client: &EmailClient,
    new_subscriber: NewSubscriber,
) -> Result<(), reqwest::Error> {
    let confirmation_link = "https://there-is-no-such-domain.com/subscriptions/confirm";

    email_client
        .send_email(
            new_subscriber.email,
            "Welcome",
            &format!(
                "Welcome to our newsletter!<br/>\
                Click <a href=\"{}\">here</a> to confirm your subscription.",
                confirmation_link
            ),
            &format!(
                "Welcome to our newsletter!<br/>\
                Visit {} to confirm your subscription.",
                confirmation_link
            ),
        )
        .await
}
