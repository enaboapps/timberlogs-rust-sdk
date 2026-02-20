use timberlogs::{Environment, LogEntry, LogLevel, TimberlogsClient, TimberlogsConfig};
fn test_config(api_key: &str) -> TimberlogsConfig {
    TimberlogsConfig {
        source: "test".into(),
        environment: Environment::Development,
        api_key: api_key.into(),
        version: None,
        user_id: None,
        session_id: None,
        dataset: None,
        batch_size: Some(1),
        flush_interval_ms: Some(60000),
        min_level: None,
        retry: None,
    }
}

#[tokio::test]
async fn test_log_entry_defaults() {
    let entry = LogEntry::default();
    assert_eq!(entry.level, LogLevel::Info);
    assert!(entry.message.is_empty());
    assert!(entry.data.is_none());
    assert!(entry.tags.is_none());
}

#[tokio::test]
async fn test_min_level_filtering() {
    let mut client = TimberlogsClient::new(TimberlogsConfig {
        min_level: Some(LogLevel::Warn),
        flush_interval_ms: Some(60000),
        ..test_config("tb_test_key")
    });

    // These should be silently filtered (no error, no queue)
    client.debug("should be filtered", None).await.unwrap();
    client.info("should be filtered", None).await.unwrap();

    client.disconnect().await.unwrap();
}

#[tokio::test]
async fn test_validation_empty_message() {
    let client = TimberlogsClient::new(test_config("tb_test_key"));

    let result = client
        .log(LogEntry {
            level: LogLevel::Info,
            message: String::new(),
            ..Default::default()
        })
        .await;

    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("message must not be empty"));
}

#[tokio::test]
async fn test_validation_too_many_tags() {
    let client = TimberlogsClient::new(test_config("tb_test_key"));

    let result = client
        .log(LogEntry {
            level: LogLevel::Info,
            message: "test".into(),
            tags: Some(vec!["tag".into(); 21]),
            ..Default::default()
        })
        .await;

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("at most 20"));
}

#[tokio::test]
async fn test_validation_step_index_bounds() {
    let client = TimberlogsClient::new(test_config("tb_test_key"));

    let result = client
        .log(LogEntry {
            level: LogLevel::Info,
            message: "test".into(),
            step_index: Some(1001),
            ..Default::default()
        })
        .await;

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("step_index"));
}

#[tokio::test]
async fn test_log_entry_with_all_fields() {
    let entry = LogEntry {
        level: LogLevel::Error,
        message: "test error".into(),
        data: Some(std::collections::HashMap::from([(
            "key".into(),
            serde_json::json!("value"),
        )])),
        user_id: Some("user_1".into()),
        session_id: Some("sess_1".into()),
        request_id: Some("req_1".into()),
        error_name: Some("TestError".into()),
        error_stack: Some("at main.rs:1".into()),
        tags: Some(vec!["tag1".into()]),
        flow_id: Some("flow_1".into()),
        step_index: Some(0),
        dataset: Some("test-dataset".into()),
    };

    assert_eq!(entry.level, LogLevel::Error);
    assert_eq!(entry.user_id.as_deref(), Some("user_1"));
    assert_eq!(entry.tags.as_ref().unwrap().len(), 1);
}
