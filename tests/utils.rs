use std::cell::Cell;
use std::net::SocketAddr;
use std::sync::Arc;
use axum::Router;
use garde::rules::email;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use uuid::Uuid;
use email_sender::{app_internal, AppState};
use email_sender::configuration::{get_configuration, DatabaseSettings};
use email_sender::email_client::EmailClient;
use email_sender::errors::AppError;
use email_sender::validation::ValidatedEmail;

pub async fn configure_database(conf: &DatabaseSettings) -> PgPool {
    let maintenance_settings = DatabaseSettings {
        database_name: "postgres".to_string(),
        username: "postgres".to_string(),
        password: "password".to_string(),
        ..conf.clone()
    };
    let mut connection = PgConnection::connect_with(&maintenance_settings.connection_options())
        .await
        .expect("error connecting to postgres..");
    // create db
    connection
        .execute(format!(r#"CREATE DATABASE "{}";"#, conf.database_name).as_str())
        .await
        .expect("Failed to create database.");

    // migrate db
    let connection_pool = PgPool::connect_with(conf.connection_options())
        .await
        .expect("Failed to connect to Postgres.");
    sqlx::migrate!("./migrations")
        .run(&connection_pool)
        .await
        .expect("error migrating db");

    connection_pool
}

/// returns server_addr and pgpool
pub async fn spawn_app() -> Result<TestAppInfo, AppError> {
    let email_server = mockito::Server::new_async().await;
    println!("mock addr {:?}", email_server.url());
    let addr = SocketAddr::from(([127, 0, 0, 1], 0)); // port-0はOSが自動でportを割り当てる
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| AppError::EstablishServer(e.to_string()))?;
    let socket_addr = listener.local_addr().unwrap();

    let mut conf = get_configuration().expect("error getting configuration");
    conf.database.database_name = Uuid::new_v4().to_string();
    let connection_pool = configure_database(&conf.database).await;
    
    let timeout = conf.email_client.timeout();
    let client = EmailClient::new(
        &email_server.url(),
        ValidatedEmail::parse(&conf.email_client.sender_email)?,
        &conf.email_client.authorization_token,
        timeout
    );
    
    let app_state = AppState {
        pg_pool: Arc::new(connection_pool.clone()),
        email_client: Arc::from(client),
        conf: Arc::new(conf),
    };
    
    let app = app_internal(app_state);
    
    let ret_val = TestAppInfo {
        socket_addr,
        db_pool: connection_pool,
        email_server
    };

    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("Server failed");
    });

    Ok(ret_val)
}

pub struct TestAppInfo {
    pub socket_addr: SocketAddr,
    pub db_pool: PgPool,
    pub email_server: mockito::ServerGuard
}