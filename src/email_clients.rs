use crate::domain::SubscriberEmail;
use reqwest::Client;
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

        let result = self
            .http_client
            .post(&self.base_url)
            .json(&request_body)
            .header(
                "X-Chosen-Email-Server-Token",
                self.authorization_token.expose_secret(),
            )
            .send()
            .await?;

        println!("{:?}", result);
        Ok(())
    }

    pub fn new(base_url: String, sender: SubscriberEmail, auth_token: Secret<String>) -> Self {
        Self {
            sender,
            base_url,
            http_client: Client::new(),
            authorization_token: auth_token,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::SubscriberEmail;
    use fake::faker::lorem::en::{Paragraph, Sentence};
    use fake::{faker::internet::en::SafeEmail, Fake};
    use wiremock::matchers::{header, header_exists, method, path};
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
    #[tokio::test]
    async fn send_email_sends_the_expected_request() {
        let mock_auth_token = Secret::new("mock_auth_token".to_string());
        let mock_server = MockServer::start().await;
        let sender = SubscriberEmail::parse(SafeEmail().fake()).unwrap();
        let email_client = EmailClient::new(mock_server.uri(), sender, mock_auth_token);

        Mock::given(header_exists("X-Chosen-Email-Server-Token"))
            .and(header("Content-Type", "application/json"))
            .and(method("POST"))
            .and(path("/"))
            .and(SendEmailBodyMatcher)
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        let subscriber_email = SubscriberEmail::parse(SafeEmail().fake()).unwrap();
        let subject: String = Sentence(1..2).fake();
        let content: String = Paragraph(1..10).fake();

        let _ = email_client
            .send_email(subscriber_email, &subject, &content, &content)
            .await;
    }
}
