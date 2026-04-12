use reqwest::Client;
use serde_json::json;
use std::env;

#[tokio::main]
async fn main() {
    let client = Client::new();
    let alert = json!({ "message": "Test alert from simulator!" });

    // Accept JWT from env var or argument
    let jwt = env::args().nth(1).or_else(|| env::var("JWT").ok());
    let jwt = jwt.expect("Pass JWT as first arg or set JWT env var");

    let resp = client
        .post("https://127.0.0.1:3000/alerts")
        .json(&alert)
        .bearer_auth(jwt)
        .send()
        .await
        .expect("Failed to send alert");

    let status = resp.status();
    let text = resp
        .text()
        .await
        .unwrap_or_else(|_| "<no body>".to_string());
    println!("Status: {status}");
    println!("Response: {text}");
}
