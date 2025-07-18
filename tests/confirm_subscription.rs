mod utils;

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::{Arc, Mutex};
// use config::ValueKind::String;
use std::string::String;
use axum::routing::get;
use regex::Regex;
use serde_json::Value;
use email_sender::handlers::subscription::subscribe;
use crate::utils::spawn_app;

use tower::ServiceExt;


#[tokio::test]
async fn confirmation_without_token_are_rejected_with_a_400() {
    let app = spawn_app().await;

    let addr = format!("http://{}/subscription/confirm", app.unwrap().socket_addr);
    println!("sending request to {addr}");
    let response = reqwest::get(
       addr
    ).await.unwrap();

    assert_eq!(response.status(), 400);
}

#[tokio::test]
async fn valid_confirmation_process() {
    let mut app = spawn_app().await.unwrap();

    let mock_server = app.email_server.mock("POST", "/email")
        .with_status(200)
        .match_body(
            mockito::Matcher::Regex("http://127.0.0.1/subscription/confirm".to_string())
        )
        .expect(1)
        .create();
    
    let form_body = "username=username&email=username%40example.com";
    let resp = reqwest::Client::new()
        .post(format!("http://{}/subscription", app.socket_addr))
        .body(form_body)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .send().await.unwrap();
    
    let confirmation_token = sqlx::query!(
        "SELECT subscription_token, subscriber_uuid FROM subscription_tokens"
    )
        .fetch_one(&app.db_pool)
        .await
        .unwrap().subscription_token;
    
    let confirmation_url = format!("http://{}/subscription/confirm?token={}", app.socket_addr, confirmation_token);
    let resp = reqwest::Client::new()
        .get(&confirmation_url)
        .send()
        .await.unwrap();
    
    assert_eq!(resp.status(), 200);
    
    mock_server.assert_async().await;
}

#[tokio::test]
async fn the_link_returned_by_subscribe_returns_a_200_if_called() {
    // Arrange
    let mut app = spawn_app().await.unwrap();
    let body = "username=username&email=username%40example.com";
    
    let mut tmp = std::string::String::new();
    
    let mock = app.email_server.mock("POST", "/email")
        .with_status(200)
        .expect_at_least(1)
        .match_body(
            mockito::Matcher::Regex("http://127.0.0.1".to_string())
        )
        .create();
    
    let resp = reqwest::Client::new()
        .post(format!("http://{}/subscription", app.socket_addr))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await.unwrap();
    assert_eq!(resp.status().as_u16(), 200);
    
    mock.assert_async().await;
}
    