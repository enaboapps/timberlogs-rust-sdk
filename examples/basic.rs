use timberlogs::{Environment, TimberlogsClient, TimberlogsConfig};

#[tokio::main]
async fn main() {
    let mut client = TimberlogsClient::new(TimberlogsConfig {
        source: "rust-example".into(),
        environment: Environment::Development,
        api_key: std::env::var("TIMBERLOGS_API_KEY").expect("TIMBERLOGS_API_KEY required"),
        version: Some("0.1.0".into()),
        user_id: None,
        session_id: None,
        dataset: None,
        batch_size: None,
        flush_interval_ms: None,
        min_level: None,
        retry: None,
    });

    client.info("Server started", None).await.unwrap();
    client.warn("High memory usage", None).await.unwrap();
    client.error("Connection timeout", None).await.unwrap();

    client.disconnect().await.unwrap();
    println!("Logs flushed successfully");
}
