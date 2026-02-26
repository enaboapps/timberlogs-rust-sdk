use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use timberlogs::{
    Environment, IngestRawOptions, LogEntry, LogLevel, RawFormat, RetryConfig, TimberlogsClient,
    TimberlogsConfig, TimberlogsError,
};

fn test_config(api_key: &str) -> TimberlogsConfig {
    TimberlogsConfig {
        source: "test".into(),
        environment: Environment::Development,
        api_key: api_key.into(),
        batch_size: Some(1),
        flush_interval_ms: Some(60000),
        ..Default::default()
    }
}

fn mock_config(api_key: &str, base_url: &str) -> TimberlogsConfig {
    TimberlogsConfig {
        base_url: Some(base_url.to_string()),
        retry: Some(RetryConfig {
            max_retries: 0,
            initial_delay_ms: 10,
            max_delay_ms: 10,
        }),
        ..test_config(api_key)
    }
}

// ── LogEntry defaults ──

#[tokio::test]
async fn test_log_entry_defaults() {
    let entry = LogEntry::default();
    assert_eq!(entry.level, LogLevel::Info);
    assert!(entry.message.is_empty());
    assert!(entry.data.is_none());
    assert!(entry.tags.is_none());
    assert!(entry.user_id.is_none());
    assert!(entry.session_id.is_none());
    assert!(entry.request_id.is_none());
    assert!(entry.error_name.is_none());
    assert!(entry.error_stack.is_none());
    assert!(entry.flow_id.is_none());
    assert!(entry.step_index.is_none());
    assert!(entry.dataset.is_none());
    assert!(entry.timestamp.is_none());
    assert!(entry.ip_address.is_none());
    assert!(entry.country.is_none());
}

#[tokio::test]
async fn test_log_entry_with_all_fields() {
    let entry = LogEntry {
        level: LogLevel::Error,
        message: "test error".into(),
        data: Some(HashMap::from([("key".into(), serde_json::json!("value"))])),
        user_id: Some("user_1".into()),
        session_id: Some("sess_1".into()),
        request_id: Some("req_1".into()),
        error_name: Some("TestError".into()),
        error_stack: Some("at main.rs:1".into()),
        tags: Some(vec!["tag1".into()]),
        flow_id: Some("flow_1".into()),
        step_index: Some(0),
        dataset: Some("test-dataset".into()),
        timestamp: Some(1700000000000),
        ip_address: Some("192.168.1.1".into()),
        country: Some("US".into()),
    };

    assert_eq!(entry.level, LogLevel::Error);
    assert_eq!(entry.user_id.as_deref(), Some("user_1"));
    assert_eq!(entry.timestamp, Some(1700000000000));
    assert_eq!(entry.ip_address.as_deref(), Some("192.168.1.1"));
    assert_eq!(entry.country.as_deref(), Some("US"));
}

// ── Config defaults ──

#[tokio::test]
async fn test_config_defaults() {
    let config = TimberlogsConfig::default();
    assert!(config.source.is_empty());
    assert_eq!(config.environment, Environment::Development);
    assert!(config.api_key.is_empty());
    assert!(config.version.is_none());
    assert!(config.batch_size.is_none());
    assert!(config.flush_interval_ms.is_none());
    assert!(config.min_level.is_none());
    assert!(config.retry.is_none());
    assert!(config.on_error.is_none());
    assert!(config.base_url.is_none());
}

// ── Min level filtering ──

#[tokio::test]
async fn test_min_level_filtering() {
    let mut client = TimberlogsClient::new(TimberlogsConfig {
        min_level: Some(LogLevel::Warn),
        flush_interval_ms: Some(60000),
        ..test_config("tb_test_key")
    });

    // Below min_level: should succeed silently (filtered out)
    client.debug("should be filtered", None).await.unwrap();
    client.info("should be filtered", None).await.unwrap();

    client.disconnect().await.unwrap();
}

#[tokio::test]
async fn test_min_level_allows_higher_levels() {
    let mut client = TimberlogsClient::new(TimberlogsConfig {
        min_level: Some(LogLevel::Warn),
        flush_interval_ms: Some(60000),
        batch_size: Some(100), // large batch so it doesn't try to flush
        ..test_config("tb_test_key")
    });

    // At or above min_level: should be accepted (queued)
    client.warn("warning", None).await.unwrap();
    client.error("error", None).await.unwrap();

    client.disconnect().await.ok();
}

// ── Validation: empty message ──

#[tokio::test]
async fn test_validation_empty_message() {
    let client = TimberlogsClient::new(test_config("tb_test_key"));

    let result = client
        .log(LogEntry {
            message: String::new(),
            ..Default::default()
        })
        .await;

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("message must not be empty"));
}

// ── Validation: message too long ──

#[tokio::test]
async fn test_validation_message_too_long() {
    let client = TimberlogsClient::new(test_config("tb_test_key"));

    let result = client
        .log(LogEntry {
            message: "x".repeat(10_001),
            ..Default::default()
        })
        .await;

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("message exceeds 10000"));
}

#[tokio::test]
async fn test_validation_message_at_limit() {
    let mut client = TimberlogsClient::new(TimberlogsConfig {
        batch_size: Some(100),
        ..test_config("tb_test_key")
    });

    let result = client
        .log(LogEntry {
            message: "x".repeat(10_000),
            ..Default::default()
        })
        .await;

    assert!(result.is_ok());
    client.disconnect().await.ok();
}

// ── Validation: tags ──

#[tokio::test]
async fn test_validation_too_many_tags() {
    let client = TimberlogsClient::new(test_config("tb_test_key"));

    let result = client
        .log(LogEntry {
            message: "test".into(),
            tags: Some(vec!["tag".into(); 21]),
            ..Default::default()
        })
        .await;

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("at most 20"));
}

#[tokio::test]
async fn test_validation_tag_too_long() {
    let client = TimberlogsClient::new(test_config("tb_test_key"));

    let result = client
        .log(LogEntry {
            message: "test".into(),
            tags: Some(vec!["x".repeat(51)]),
            ..Default::default()
        })
        .await;

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("tags[0] exceeds 50"));
}

// ── Validation: step_index ──

#[tokio::test]
async fn test_validation_step_index_bounds() {
    let client = TimberlogsClient::new(test_config("tb_test_key"));

    let result = client
        .log(LogEntry {
            message: "test".into(),
            step_index: Some(1001),
            ..Default::default()
        })
        .await;

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("step_index"));
}

#[tokio::test]
async fn test_validation_step_index_at_limit() {
    let mut client = TimberlogsClient::new(TimberlogsConfig {
        batch_size: Some(100),
        ..test_config("tb_test_key")
    });

    let result = client
        .log(LogEntry {
            message: "test".into(),
            step_index: Some(1000),
            ..Default::default()
        })
        .await;

    assert!(result.is_ok());
    client.disconnect().await.ok();
}

// ── Validation: string field limits ──

async fn assert_field_too_long(field: &str, max_len: usize) {
    let client = TimberlogsClient::new(test_config("tb_test_key"));
    let long = "x".repeat(max_len + 1);

    let mut entry = LogEntry {
        message: "test".into(),
        ..Default::default()
    };

    match field {
        "user_id" => entry.user_id = Some(long),
        "session_id" => entry.session_id = Some(long),
        "request_id" => entry.request_id = Some(long),
        "error_name" => entry.error_name = Some(long),
        "error_stack" => entry.error_stack = Some(long),
        "flow_id" => entry.flow_id = Some(long),
        "dataset" => entry.dataset = Some(long),
        "ip_address" => entry.ip_address = Some(long),
        "country" => entry.country = Some(long),
        _ => panic!("unknown field: {field}"),
    }

    let result = client.log(entry).await;
    assert!(result.is_err(), "{field} should fail validation");
    let err = result.unwrap_err().to_string();
    assert!(err.contains(field), "error should mention {field}: {err}");
    assert!(
        err.contains(&format!("exceeds {max_len}")),
        "error should mention limit {max_len}: {err}"
    );
}

#[tokio::test]
async fn test_validation_user_id_too_long() {
    assert_field_too_long("user_id", 100).await;
}

#[tokio::test]
async fn test_validation_session_id_too_long() {
    assert_field_too_long("session_id", 100).await;
}

#[tokio::test]
async fn test_validation_request_id_too_long() {
    assert_field_too_long("request_id", 100).await;
}

#[tokio::test]
async fn test_validation_error_name_too_long() {
    assert_field_too_long("error_name", 200).await;
}

#[tokio::test]
async fn test_validation_error_stack_too_long() {
    assert_field_too_long("error_stack", 10_000).await;
}

#[tokio::test]
async fn test_validation_flow_id_too_long() {
    assert_field_too_long("flow_id", 100).await;
}

#[tokio::test]
async fn test_validation_dataset_too_long() {
    assert_field_too_long("dataset", 50).await;
}

#[tokio::test]
async fn test_validation_ip_address_too_long() {
    assert_field_too_long("ip_address", 100).await;
}

#[tokio::test]
async fn test_validation_country_too_long() {
    assert_field_too_long("country", 10).await;
}

// ── set_user_id / set_session_id ──

#[tokio::test]
async fn test_set_user_id() {
    let mut server = mockito::Server::new_async().await;
    let mock = server
        .mock("POST", "/v1/logs")
        .with_status(200)
        .with_body(r#"{"success":true,"count":1}"#)
        .create_async()
        .await;

    let mut client = TimberlogsClient::new(mock_config("tb_key", &server.url()));

    client.set_user_id(Some("user_123".into())).await;
    client.info("test message", None).await.unwrap();

    client.disconnect().await.unwrap();
    mock.assert_async().await;
}

#[tokio::test]
async fn test_set_session_id() {
    let mut server = mockito::Server::new_async().await;
    let mock = server
        .mock("POST", "/v1/logs")
        .with_status(200)
        .with_body(r#"{"success":true,"count":1}"#)
        .create_async()
        .await;

    let mut client = TimberlogsClient::new(mock_config("tb_key", &server.url()));

    client.set_session_id(Some("sess_abc".into())).await;
    client.info("test message", None).await.unwrap();

    client.disconnect().await.unwrap();
    mock.assert_async().await;
}

// ── Batch flush and HTTP ──

#[tokio::test]
async fn test_batch_flush_on_size() {
    let mut server = mockito::Server::new_async().await;
    let mock = server
        .mock("POST", "/v1/logs")
        .with_status(200)
        .with_body(r#"{"success":true,"count":2}"#)
        .expect(1)
        .create_async()
        .await;

    let mut client = TimberlogsClient::new(TimberlogsConfig {
        batch_size: Some(2),
        ..mock_config("tb_key", &server.url())
    });

    // First log: queued, not flushed
    client.info("msg 1", None).await.unwrap();
    // Second log: triggers flush
    client.info("msg 2", None).await.unwrap();

    client.disconnect().await.unwrap();
    mock.assert_async().await;
}

#[tokio::test]
async fn test_manual_flush() {
    let mut server = mockito::Server::new_async().await;
    let mock = server
        .mock("POST", "/v1/logs")
        .with_status(200)
        .with_body(r#"{"success":true,"count":1}"#)
        .expect(1)
        .create_async()
        .await;

    let mut client = TimberlogsClient::new(TimberlogsConfig {
        batch_size: Some(100), // won't auto-flush
        ..mock_config("tb_key", &server.url())
    });

    client.info("buffered msg", None).await.unwrap();
    client.flush().await.unwrap();

    client.disconnect().await.unwrap();
    mock.assert_async().await;
}

#[tokio::test]
async fn test_flush_empty_queue() {
    let mut client = TimberlogsClient::new(test_config("tb_test_key"));
    // Flush with nothing queued should succeed
    client.flush().await.unwrap();
    client.disconnect().await.unwrap();
}

// ── Disconnect / graceful shutdown ──

#[tokio::test]
async fn test_disconnect_flushes() {
    let mut server = mockito::Server::new_async().await;
    let mock = server
        .mock("POST", "/v1/logs")
        .with_status(200)
        .with_body(r#"{"success":true,"count":1}"#)
        .expect(1)
        .create_async()
        .await;

    let mut client = TimberlogsClient::new(TimberlogsConfig {
        batch_size: Some(100),
        ..mock_config("tb_key", &server.url())
    });

    client.info("will be flushed on disconnect", None).await.unwrap();
    client.disconnect().await.unwrap();

    mock.assert_async().await;
}

// ── HTTP error handling ──

#[tokio::test]
async fn test_http_error_returns_error() {
    let mut server = mockito::Server::new_async().await;
    let _mock = server
        .mock("POST", "/v1/logs")
        .with_status(500)
        .with_body("Internal Server Error")
        .create_async()
        .await;

    let client = TimberlogsClient::new(mock_config("tb_key", &server.url()));

    // batch_size=1, so this triggers a flush that will fail
    let result = client.info("test", None).await;
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("500"));
}

// ── Retry logic ──

#[tokio::test]
async fn test_retry_succeeds_after_failure() {
    let mut server = mockito::Server::new_async().await;

    // First request fails, second succeeds
    let _fail = server
        .mock("POST", "/v1/logs")
        .with_status(500)
        .with_body("error")
        .expect(1)
        .create_async()
        .await;

    let _ok = server
        .mock("POST", "/v1/logs")
        .with_status(200)
        .with_body(r#"{"success":true,"count":1}"#)
        .expect(1)
        .create_async()
        .await;

    let mut client = TimberlogsClient::new(TimberlogsConfig {
        retry: Some(RetryConfig {
            max_retries: 1,
            initial_delay_ms: 10,
            max_delay_ms: 10,
        }),
        ..mock_config("tb_key", &server.url())
    });

    client.info("retry test", None).await.unwrap();
    client.disconnect().await.unwrap();
}

// ── on_error callback ──

#[tokio::test]
async fn test_on_error_callback() {
    let mut server = mockito::Server::new_async().await;
    let _mock = server
        .mock("POST", "/v1/logs")
        .with_status(500)
        .with_body("error")
        .create_async()
        .await;

    let error_count = Arc::new(AtomicU32::new(0));
    let counter = Arc::clone(&error_count);

    let mut client = TimberlogsClient::new(TimberlogsConfig {
        on_error: Some(Box::new(move |_err: &TimberlogsError| {
            counter.fetch_add(1, Ordering::SeqCst);
        })),
        batch_size: Some(100),
        flush_interval_ms: Some(50), // short interval to trigger background flush
        ..mock_config("tb_key", &server.url())
    });

    client.info("queued", None).await.unwrap();

    // Wait for background flush to fire
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    assert!(error_count.load(Ordering::SeqCst) > 0, "on_error should have been called");

    client.disconnect().await.ok();
}

// ── ingest_raw ──

#[tokio::test]
async fn test_ingest_raw_json() {
    let mut server = mockito::Server::new_async().await;
    let mock = server
        .mock("POST", "/v1/logs")
        .match_query(mockito::Matcher::AllOf(vec![
            mockito::Matcher::UrlEncoded("format".into(), "json".into()),
        ]))
        .match_header("content-type", "application/json")
        .with_status(200)
        .with_body("")
        .expect(1)
        .create_async()
        .await;

    let mut client = TimberlogsClient::new(mock_config("tb_key", &server.url()));

    client
        .ingest_raw(r#"{"msg":"hello"}"#, RawFormat::Json, None)
        .await
        .unwrap();

    client.disconnect().await.unwrap();
    mock.assert_async().await;
}

#[tokio::test]
async fn test_ingest_raw_csv_with_options() {
    let mut server = mockito::Server::new_async().await;
    let mock = server
        .mock("POST", "/v1/logs")
        .match_query(mockito::Matcher::AllOf(vec![
            mockito::Matcher::UrlEncoded("format".into(), "csv".into()),
            mockito::Matcher::UrlEncoded("source".into(), "my-app".into()),
            mockito::Matcher::UrlEncoded("dataset".into(), "logs".into()),
        ]))
        .match_header("content-type", "text/csv")
        .with_status(200)
        .with_body("")
        .expect(1)
        .create_async()
        .await;

    let mut client = TimberlogsClient::new(mock_config("tb_key", &server.url()));

    client
        .ingest_raw(
            "level,message\ninfo,hello",
            RawFormat::Csv,
            Some(IngestRawOptions {
                source: Some("my-app".into()),
                dataset: Some("logs".into()),
                ..Default::default()
            }),
        )
        .await
        .unwrap();

    client.disconnect().await.unwrap();
    mock.assert_async().await;
}

#[tokio::test]
async fn test_ingest_raw_all_formats() {
    let formats = vec![
        (RawFormat::Json, "json", "application/json"),
        (RawFormat::Jsonl, "jsonl", "application/x-ndjson"),
        (RawFormat::Syslog, "syslog", "application/x-syslog"),
        (RawFormat::Text, "text", "text/plain"),
        (RawFormat::Csv, "csv", "text/csv"),
        (RawFormat::Obl, "obl", "application/x-obl"),
    ];

    for (format, format_str, content_type) in formats {
        assert_eq!(format.as_str(), format_str);
        assert_eq!(format.content_type(), content_type);
    }
}

// ── LogEntry serialization ──

#[tokio::test]
async fn test_log_entry_serialization_omits_none() {
    let entry = LogEntry {
        message: "test".into(),
        ..Default::default()
    };

    let json = serde_json::to_value(&entry).unwrap();
    assert!(json.get("data").is_none());
    assert!(json.get("userId").is_none());
    assert!(json.get("tags").is_none());
    assert!(json.get("timestamp").is_none());
    assert!(json.get("ipAddress").is_none());
    assert!(json.get("country").is_none());
}

#[tokio::test]
async fn test_log_entry_serialization_includes_set_fields() {
    let entry = LogEntry {
        message: "test".into(),
        user_id: Some("u1".into()),
        timestamp: Some(1700000000000),
        ip_address: Some("10.0.0.1".into()),
        country: Some("GB".into()),
        ..Default::default()
    };

    let json = serde_json::to_value(&entry).unwrap();
    assert_eq!(json["userId"], "u1");
    assert_eq!(json["timestamp"], 1700000000000u64);
    assert_eq!(json["ipAddress"], "10.0.0.1");
    assert_eq!(json["country"], "GB");
}

// ── LogLevel ordering ──

#[tokio::test]
async fn test_log_level_ordering() {
    assert!(LogLevel::Debug < LogLevel::Info);
    assert!(LogLevel::Info < LogLevel::Warn);
    assert!(LogLevel::Warn < LogLevel::Error);
}

// ── Environment serialization ──

#[tokio::test]
async fn test_environment_serialization() {
    assert_eq!(
        serde_json::to_string(&Environment::Development).unwrap(),
        "\"development\""
    );
    assert_eq!(
        serde_json::to_string(&Environment::Staging).unwrap(),
        "\"staging\""
    );
    assert_eq!(
        serde_json::to_string(&Environment::Production).unwrap(),
        "\"production\""
    );
}

// ── Convenience methods ──

#[tokio::test]
async fn test_convenience_methods_set_correct_level() {
    let mut server = mockito::Server::new_async().await;
    let mock = server
        .mock("POST", "/v1/logs")
        .with_status(200)
        .with_body(r#"{"success":true,"count":1}"#)
        .expect(4)
        .create_async()
        .await;

    let mut client = TimberlogsClient::new(mock_config("tb_key", &server.url()));

    client.debug("d", None).await.unwrap();
    client.info("i", None).await.unwrap();
    client.warn("w", None).await.unwrap();
    client.error("e", None).await.unwrap();

    client.disconnect().await.unwrap();
    mock.assert_async().await;
}
