use actix_web::{web, HttpResponse};
use chrono::Utc;
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use sqlx::{Executor, PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::{
    domain::{FormDataSubscriber, NewSubscriber, SubscriberEmail, SubscriberName},
    email_clients::EmailClient,
    startup::ApplicationBaseUrl,
};

#[tracing::instrument(name="Adding a new subscriber", skip_all, fields(subscriber_email=%form.email, subscriber_name=%form.name))]
// The web::Form<> and web::Data annotations are telling the framework
// what to extract from the http request.
// After extraction, form and connection_pool will be of the type annotated inside the <>
pub async fn subsribe(
    form: web::Form<FormDataSubscriber>,
    connection_pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
    base_url: web::Data<ApplicationBaseUrl>,
) -> HttpResponse {
    let form: FormDataSubscriber = form.into_inner();

    let new_subscriber = match NewSubscriber::try_from(form) {
        Ok(subscriber) => subscriber,
        Err(_) => return HttpResponse::BadRequest().finish(),
    };

    // Pick up a connection from the pool
    // for the upcoming transaction.
    let mut transaction = match connection_pool.begin().await {
        Ok(transaction) => transaction,
        Err(_) => {
            return HttpResponse::InternalServerError().finish();
        }
    };

    let subscription_token = generate_subscription_toke();
    let subscriber_id = match insert_subscriber(&new_subscriber, &mut transaction).await {
        Ok(subcriber_id) => subcriber_id,
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };

    if store_token(&mut transaction, &subscription_token, subscriber_id)
        .await
        .is_err()
    {
        return HttpResponse::InternalServerError().finish();
    }

    // End the transaction by explicitly calling commit
    // on the connection used for the transaction.
    if transaction.commit().await.is_err() {
        return HttpResponse::InternalServerError().finish();
    };

    if send_confirmatioin_email(
        &email_client,
        new_subscriber,
        &base_url.0,
        &subscription_token,
    )
    .await
    .is_err()
    {
        return HttpResponse::InternalServerError().finish();
    }

    HttpResponse::Ok().finish()
}

#[tracing::instrument(
    name = "Saving new subscriber details in the database",
    skip(new_subscriber, transaction)
)]
async fn insert_subscriber(
    new_subscriber: &NewSubscriber,
    transaction: &mut Transaction<'_, Postgres>,
) -> Result<Uuid, sqlx::Error> {
    let subscriber_id = Uuid::new_v4();
    let query = sqlx::query!(
        r#"
    INSERT INTO subscriptions (id, email, name, subscribed_at, status)
    VALUES ($1, $2, $3, $4, $5)"#,
        subscriber_id,
        new_subscriber.email.as_ref(),
        new_subscriber.name.as_ref(),
        Utc::now(),
        "pending_confirmation"
    );

    transaction.execute(query).await.map_err(|err| {
        // trace the query error without changing the flow
        tracing::error!("Failed to execute query: {:?}", err);
        err
    })?;
    Ok(subscriber_id)
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
    base_url: &str,
    subscription_token: &str,
) -> Result<(), reqwest::Error> {
    let confirmation_link = format!(
        "{}/subscriptions/confirm?subscription_token={}",
        base_url, subscription_token
    );

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

fn generate_subscription_toke() -> String {
    let mut rng = thread_rng();
    std::iter::repeat_with(|| rng.sample(Alphanumeric))
        .map(char::from)
        .take(25)
        .collect()
}

#[tracing::instrument(name = "Storing subscription token", skip_all)]
pub async fn store_token(
    transaction: &mut Transaction<'_, Postgres>,
    subscription_token: &str,
    subscriber_id: Uuid,
) -> Result<(), sqlx::Error> {
    let query = sqlx::query!(
        r#"INSERT INTO subscription_tokens (subscriber_id, subscription_token)
    VALUES ($1, $2)"#,
        subscriber_id,
        subscription_token
    );

    transaction.execute(query).await.map_err(|err| {
        tracing::error!("Failed to execute query: {:?}", err);
        err
    })?;
    Ok(())
}
