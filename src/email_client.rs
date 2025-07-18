use crate::errors::AppError;
use crate::validation::ValidatedEmail;
use reqwest::Client;
use serde::Serialize;
use std::time::Duration;
use anyhow::Context;

#[derive(Clone, Debug)]

pub struct EmailClient {
    http_client: Client,
    email_server_url: String,
    my_domain_email: ValidatedEmail, // 自身のドメインのメアド
    authorization_token: String,
}
#[derive(Serialize, Debug)]
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
        recipient: &str, // todo : email checking..
        subject: &str,
        html_content: &str,
        text_content: &str,
    ) -> anyhow::Result<()> {
        let url = format!("{}/email", self.email_server_url);
        let request_body = SendEmailRequest {
            from: self.as_str(),
            to: recipient,
            subject,
            html_body: html_content,
            text_body: text_content,
        };
        let builder = self
            .http_client
            .post(&url)
            .header("X-Postmark-Server-Token", self.authorization_token.clone())
            .timeout(Duration::from_secs(10))
            .json(&request_body);
        let resp = builder
            .send()
            .await?
            .error_for_status()
            .context("server returned error")?;
        Ok(())
    }

    pub fn new(
        base_url: &str,
        sender: ValidatedEmail,
        authorization_token: &str,
        timeout: std::time::Duration,
    ) -> Self {
        let http_client = Client::builder().timeout(timeout).build().unwrap();
        Self {
            http_client,
            email_server_url: base_url.to_string(),
            my_domain_email: sender,
            authorization_token: authorization_token.to_string()
        }
    }
    
    pub fn url(&self) -> &str {
        &self.email_server_url
    }
 
    fn as_str(&self) -> &str {
        &self.my_domain_email.0
    }
}

#[cfg(test)]
mod tests {
    use crate::email_client::EmailClient;
    use crate::validation::ValidatedEmail;
    use fake::faker::internet::en::SafeEmail;
    use fake::faker::lorem::ja_jp::{Paragraph, Sentence};
    use fake::{Fake, Faker};
    use std::thread;
    use std::time::Duration;
    use tracing::Instrument;
    use tracing_subscriber::fmt::format::json;

    /// Generate a random email subject
    fn subject() -> String {
        Sentence(1..2).fake()
    }
    /// Generate a random email content
    fn content() -> String {
        Paragraph(1..10).fake()
    }
    /// Generate a random subscriber email
    fn email() -> String {
        ValidatedEmail::parse(&SafeEmail().fake::<String>()).unwrap().0
    }

    /// Get a test instance of `EmailClient`.
    fn email_client(base_url: &str) -> EmailClient {
        let tmp_token: String = Faker.fake();
        EmailClient::new(
            base_url,
            ValidatedEmail::parse(&SafeEmail().fake::<String>()).unwrap(),
            &tmp_token,
            Duration::from_secs(10),
        )
    }

    #[tokio::test]
    async fn send_email_with_expected_request() {
        let mut server = mockito::Server::new_async().await;
        
        let sender = ValidatedEmail::parse(&SafeEmail().fake::<String>()).unwrap();
        let tmp_token: String = Faker.fake();
        let email_client = EmailClient::new(
            &server.url(),
            sender,
            &tmp_token,
            Duration::from_secs(10),
        );

        let mock = server
            .mock("POST", "/email")
            .match_header("content-type", "application/json")
            .expect(1) // get at least 1 request...
            .match_header("X-Postmark-Server-Token", tmp_token.as_str())
            .create();

        // let subscriber_email = ValidatedEmail::parse(SafeEmail().fake()).unwrap();
        // let subject: String = Sentence(1..2).fake();
        // let content: String = Paragraph(1..10).fake();

        email_client
            .send_email(&email(), &subject(), &content(), &content())
            .await
            .expect("error sending email");
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn send_email_with_status_200() {
        let mut server = mockito::Server::new_async().await;
        
        let sender = ValidatedEmail::parse(&SafeEmail().fake::<String>()).unwrap();
        let tmp_token: String = Faker.fake();
        let email_client = EmailClient::new(
            &server.url(),
            sender,
            &tmp_token,
            std::time::Duration::from_secs(10),
        );

        let mock = server
            .mock("POST", "/email")
            .with_status(200)
            .match_header("content-type", "application/json")
            .expect(1) // get at least 1 request...
            .match_header("X-Postmark-Server-Token", tmp_token.as_str())
            .create();

        // let subscriber_email = ValidatedEmail::parse(SafeEmail().fake()).unwrap();
        // let subject: String = Sentence(1..2).fake();
        // let content: String = Paragraph(1..10).fake();

        let res = email_client
            .send_email(&email(), &subject(), &content(), &content())
            .await;
        assert!(res.is_ok());
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn send_email_with_status_500() {
        let mut server = mockito::Server::new_async().await;

        let sender = ValidatedEmail::parse(&SafeEmail().fake::<String>()).unwrap();
        let tmp_token: String = Faker.fake();
        let email_client = EmailClient::new(
            &server.url(),
            sender,
            &tmp_token,
            std::time::Duration::from_secs(10),
        );

        let mock = server
            .mock("POST", "/email")
            .with_status(500)
            .match_header("content-type", "application/json")
            .expect(1) // get at least 1 request...
            .match_header("X-Postmark-Server-Token", tmp_token.as_str())
            .create();

        let res = email_client
            .send_email(&email(), &subject(), &content(), &content())
            .await;
        println!("res : {res:?}");
        assert!(res.is_err());
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn send_email_with_too_slow_response() {
        let mut server = mockito::Server::new_async().await;

        let sender = ValidatedEmail::parse(&SafeEmail().fake::<String>()).unwrap();
        let tmp_token: String = Faker.fake();
        let email_client = EmailClient::new(
            &server.url(),
            sender,
            &tmp_token,
            std::time::Duration::from_secs(10),
        );

        let mock = server
            .mock("POST", "/email")
            .with_status(200)
            .match_header("content-type", "application/json")
            .match_request(|_| {
                thread::sleep(Duration::from_secs(11));
                true
            })
            .expect(1) // get at least 1 request...
            .match_header("X-Postmark-Server-Token", tmp_token.as_str())
            .create();

        let res = email_client
            .send_email(&email(), &subject(), &content(), &content())
            .await;
        assert!(res.is_err());
        mock.assert_async().await;
    }
}
