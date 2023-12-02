use sqlx::{Connection, PgConnection};
use std::net::TcpListener;

use zero2prod::configuration::get_configuration;

fn spawn_app() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("Couldn't find random port");

    let port = listener.local_addr().unwrap().port();
    let server = zero2prod::startup::run(listener).expect("Failed to bind address");

    tokio::spawn(server);

    format!("http://127.0.0.1:{}", port)
}

#[tokio::test]
async fn health_check_works() {
    let server_addrs = spawn_app();
    let client = reqwest::Client::new();

    let response = client
        .get(&format!("{}/health_check", &server_addrs))
        .send()
        .await
        .expect("Failed to execute request");

    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length())
}

#[tokio::test]
async fn subscribe_returns_200_for_valid_form_data() {
    let server_addrs = spawn_app();
    let configuration = get_configuration().expect("Failed to read connection");
    let connection_string = configuration.database.connection_string();

    let mut connection = PgConnection::connect(&connection_string)
        .await
        .expect("Failed to connect Postgres.");
    let client = reqwest::Client::new();

    let body = "name=Suraj%20Vijayan&email=surajvijay67%40@gmail.com";
    let response = client
        .post(&format!("{}/subscriptions", &server_addrs))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(200, response.status().as_u16());

    let saved = sqlx::query!("SELECT email, name FROM subscriptions",)
        .fetch_one(&mut connection)
        .await
        .expect("Failed to fetch saved subscription.");

    assert_eq!(saved.email, "surajvijay67@gmail.com");
    assert_eq!(saved.name, "Suraj Vijayan");
}

#[tokio::test]
async fn subscribe_returns_400_when_data_is_missing() {
    let server_addrs = spawn_app();
    let client = reqwest::Client::new();
    let test_cases = vec![
        ("name=Suraj%20Vijayan", "missing email"),
        ("email=surajvijay67%40@gmail.com", "missing name"),
        ("", "missing both name and email"),
    ];

    for (invalid_body, err_msg) in test_cases {
        let response = client
            .post(&format!("{}/subscriptions", &server_addrs))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(invalid_body)
            .send()
            .await
            .expect("Failed to execute request");

        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not fail with 400 Bad Request when the payload was {}.",
            err_msg
        );
    }
}
