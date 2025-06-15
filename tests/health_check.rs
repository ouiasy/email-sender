use axum::http;
use email_sender::configuration::{DatabaseSettings, get_configuration};
use email_sender::{app_internal, errors::AppError};
use sqlx::{Connection, Executor, PgConnection, PgPool};
use std::net::SocketAddr;
use uuid::Uuid;
// `tokio::test` is the testing equivalent of `tokio::main`.
// It also spares you from having to specify the `#[test]` attribute.
//
// You can inspect what code gets generated using
// `cargo expand --test health_check` (<- name of the test file)

#[tokio::test]
async fn health_check_works() {
    println!("hello");
    let test_info = spawn_app().await.expect("error spawning app...");
    let url = format!("http://{}/health/mee", test_info.socket_addr);

    let client = reqwest::Client::new();

    let res = client
        .get(url)
        .send()
        .await
        .expect("error waiting response from /health");

    assert!(res.status().is_success());
    assert_eq!(Some(9), res.content_length());
}

#[tokio::test]
async fn subscribe_returns_200_with_valid_data() {
    let test_info = spawn_app().await.expect("error spawning app...");
    let url = format!("http://{}/subscription", test_info.socket_addr);
    let client = reqwest::Client::new();

    let configuration = get_configuration().expect("error reading configuration from yaml");
    let connection_url = configuration.database.connection_string();
    let mut con = PgConnection::connect(&connection_url)
        .await
        .expect("error connecting to postgres");

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

    assert_eq!(saved.email, "username@example.com");
    assert_eq!(saved.name, "username");
}

#[tokio::test]
async fn subscribe_returns_400_with_invalid_data() {
    let test_info = spawn_app().await.expect("error spawning app...");
    let url = format!("http://{}/subscription", test_info.socket_addr);
    let client = reqwest::Client::new();

    let configuration = get_configuration().expect("error reading configuration from yaml");
    let connection_url = configuration.database.connection_string();

    let con = PgConnection::connect(&connection_url)
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
            axum::http::StatusCode::UNPROCESSABLE_ENTITY, // 422
            "status is not 400 when whole data is `{invalid_body}`"
        )
    }
}

async fn configure_database(conf: &DatabaseSettings) -> PgPool {
    let maintenance_settings = DatabaseSettings {
        database_name: "postgres".to_string(),
        username: "postgres".to_string(),
        password: "password".to_string(),
        ..conf.clone()
    };
    let mut connection = PgConnection::connect(&maintenance_settings.connection_string())
        .await
        .expect("error connecting to postgres..");
    // create db
    connection
        .execute(format!(r#"CREATE DATABASE "{}";"#, conf.database_name).as_str())
        .await
        .expect("Failed to create database.");

    // migrate db
    let connection_pool = PgPool::connect(&conf.connection_string())
        .await
        .expect("Failed to connect to Postgres.");
    sqlx::migrate!("./migrations")
        .run(&connection_pool)
        .await
        .expect("error migrating db");

    connection_pool
}

/// returns server_addr and pgpool
async fn spawn_app() -> Result<TestAppInfo, AppError> {
    let addr = SocketAddr::from(([127, 0, 0, 1], 0)); // port-0はOSが自動でportを割り当てる
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| AppError::EstablishServer(e.to_string()))?;
    let socket_addr = listener.local_addr().unwrap();

    let mut conf = get_configuration().expect("error getting configuration");
    conf.database.database_name = Uuid::new_v4().to_string();
    let connection_pool = configure_database(&mut conf.database).await;

    let app = app_internal(connection_pool.clone());

    let ret_val = TestAppInfo {
        socket_addr,
        db_pool: connection_pool,
    };

    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("Server failed");
    });

    Ok(ret_val)
}

struct TestAppInfo {
    pub socket_addr: SocketAddr,
    pub db_pool: PgPool,
}
