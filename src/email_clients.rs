use std::time::Duration;

use crate::domain::SubscriberEmail;
use reqwest::{Client, ClientBuilder};
use secrecy::{ExposeSecret, Secret};

pub struct EmailClient {
    sender: SubscriberEmail,
    http_client: Client,
    // the API url that we want to call and have it send the email for us
    base_url: String,
    authorization_token: Secret<String>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "PascalCase")]
struct SendEmailRequest<'a> {
    from: &'a str,
    to: &'a str,
    subject: &'a str,
    html_body: &'a str,
    text_body: &'a str,
}

impl EmailClient {
    pub async fn send_email(
        &self,
        recipient: SubscriberEmail,
        subject: &str,
        html_content: &str,
        text_content: &str,
    ) -> Result<(), reqwest::Error> {
        let request_body = SendEmailRequest {
            from: self.sender.as_ref(),
            to: recipient.as_ref(),
            subject,
            html_body: html_content,
            text_body: text_content,
        };

        let _result = self
            .http_client
            .post(&self.base_url)
            .json(&request_body)
            .header(
                "X-Chosen-Email-Server-Token",
                self.authorization_token.expose_secret(),
            )
            .send()
            .await?
            // the error returned here includes error while sending the request_body
            // it DOES NOT include the error response from the server.
            // .error_for_status() examines the response,
            // and Expose the error response from the server.
            .error_for_status()?;

        Ok(())
    }

    pub fn new(
        base_url: String,
        sender: SubscriberEmail,
        auth_token: Secret<String>,
        timeout_millis: Duration,
    ) -> Self {
        let http_client = ClientBuilder::new()
            .timeout(timeout_millis)
            .build()
            .expect("Failed to create test client");
        Self {
            sender,
            base_url,
            http_client,
            authorization_token: auth_token,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::domain::SubscriberEmail;
    use claims::{assert_err, assert_ok};
    use fake::faker::lorem::en::{Paragraph, Sentence};
    use fake::{faker::internet::en::SafeEmail, Fake};
    use wiremock::matchers::{any, header, header_exists, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use wiremock::Request;

    struct SendEmailBodyMatcher;
    impl wiremock::Match for SendEmailBodyMatcher {
        fn matches(&self, request: &Request) -> bool {
            let parsed_body: Result<serde_json::Value, _> = serde_json::from_slice(&request.body);
            match parsed_body {
                Ok(json_body) => {
                    json_body.get("From").is_some()
                        && json_body.get("To").is_some()
                        && json_body.get("Subject").is_some()
                }
                Err(_) => false,
            }
        }
    }

    // helper functions

    fn mock_auth_token() -> Secret<String> {
        Secret::new("mock_auth_token".to_string())
    }

    async fn mock_server() -> MockServer {
        MockServer::start().await
    }

    fn sender() -> SubscriberEmail {
        SubscriberEmail::parse(SafeEmail().fake()).unwrap()
    }

    fn subscriber() -> SubscriberEmail {
        SubscriberEmail::parse(SafeEmail().fake()).unwrap()
    }

    fn subject() -> String {
        Sentence(1..2).fake()
    }

    fn content() -> String {
        Paragraph(1..10).fake()
    }

    fn email_client(base_url: String) -> EmailClient {
        EmailClient::new(
            base_url,
            sender(),
            mock_auth_token(),
            Duration::from_millis(200),
        )
    }

    // test section

    #[tokio::test]
    async fn send_email_sends_the_expected_request() {
        let mock_server = mock_server().await;
        let email_client = email_client(mock_server.uri());

        Mock::given(header_exists("X-Chosen-Email-Server-Token"))
            .and(header("Content-Type", "application/json"))
            .and(method("POST"))
            .and(path("/"))
            .and(SendEmailBodyMatcher)
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        let outcome = email_client
            .send_email(subscriber(), &subject(), &content(), &content())
            .await;

        assert_ok!(outcome);
    }

    #[tokio::test]
    async fn send_email_times_out_if_the_server_takes_too_long() {
        let mock_server = mock_server().await;
        let email_client = email_client(mock_server.uri());

        Mock::given(any())
            .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(120)))
            .expect(1)
            .mount(&mock_server)
            .await;

        let outcome = email_client
            .send_email(subscriber(), &subject(), &content(), &content())
            .await;

        assert_err!(outcome);
    }
}
