mod utils;

use axum::http;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use std::env::set_var;
use utils::spawn_app;

#[tokio::test]
async fn subscribe_returns_200_with_valid_data() {
    let mut test_info = spawn_app().await.expect("error spawning app...");
    let url = format!("http://{}/subscription", test_info.socket_addr);
    let client = reqwest::Client::new();

    let pg_option = test_info.db_pool.connect_options();

    let mut con = PgConnection::connect_with(&pg_option)
        .await
        .expect("error connecting to postgres");
    
    let mock_server = test_info.email_server.mock("POST", "/email")
        .with_status(200)
        .expect(1)
        .create();

    let body = "username=username&email=username%40example.com";
    let res = client
        .post(url)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("failed to execute request.");

    assert_eq!(res.status(), 200);

    let saved = sqlx::query!("select email, name from subscriptions")
        .fetch_one(&mut con)
        .await
        .expect("Failed to fetch saved subscription.");
    println!("{saved:?}");

    assert_eq!(saved.email, "username@example.com");
    assert_eq!(saved.name, "username");
    
    mock_server.assert_async().await;
}

#[tokio::test]
async fn subscribe_returns_400_with_invalid_data() {
    let test_info = spawn_app().await.expect("error spawning app...");
    let url = format!("http://{}/subscription", test_info.socket_addr);
    let client = reqwest::Client::new();

    let pg_option = test_info.db_pool.connect_options();

    let con = PgConnection::connect_with(&pg_option)
        .await
        .expect("error connecting to postgres");

    let test_cases = vec![
        ("username=username", "need email address"),
        ("email=username@example.com", "need username"),
        ("", "missing form information"),
    ];

    for (invalid_body, _) in test_cases {
        let response = client
            .post(&url)
            .header(
                http::header::CONTENT_TYPE,
                mime::APPLICATION_WWW_FORM_URLENCODED.as_ref(),
            )
            .body(invalid_body)
            .send()
            .await
            .expect("error receiving response..");

        assert_eq!(
            response.status(),
            http::StatusCode::UNPROCESSABLE_ENTITY, // 422
            "status is not 422 when whole data is `{invalid_body}`"
        )
    }
}

#[tokio::test]
async fn subscribe_returns_400_with_invalid_individual_data() {
    let test_info = spawn_app().await.expect("error spawning app...");
    let url = format!("http://{}/subscription", test_info.socket_addr);
    let client = reqwest::Client::new();

    let pg_option = test_info.db_pool.connect_options();

    let con = PgConnection::connect_with(&pg_option)
        .await
        .expect("error connecting to postgres");

    let test_cases = vec![
        (
            "username=username&email=testdataexample.com",
            "need valid email",
        ),
        (
            "username=user__&email=testdata@example.com",
            "invalid username",
        ),
        (
            "username=おかひょう&email=hello",
            "missing form information",
        ),
    ];

    for (invalid_body, _) in test_cases {
        let response = client
            .post(&url)
            .header(
                http::header::CONTENT_TYPE,
                mime::APPLICATION_WWW_FORM_URLENCODED.as_ref(),
            )
            .body(invalid_body)
            .send()
            .await
            .expect("error receiving response..");

        assert_eq!(
            response.status(),
            http::StatusCode::BAD_REQUEST, // 400
            "status is not 400 when whole data is `{invalid_body}`"
        )
    }
}

#[tokio::test]
async fn subscribe_sends_a_confirmation_email() {
    let mut app_info = spawn_app().await.unwrap();
    let body = "username=username&email=username%40example.com";

    let mock = app_info
        .email_server
        .mock("POST", "/email")
        .with_status(200)
        .expect(1)
        .create();

    let client = reqwest::Client::new()
        .post(format!("http://{}/subscription", app_info.socket_addr))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await;

    mock.assert_async().await;
}
