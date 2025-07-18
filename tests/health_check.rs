mod utils;

use crate::utils::spawn_app;

#[tokio::test]
async fn health_check_works() {
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