# timberlogs

Rust SDK for [Timberlogs](https://timberlogs.dev) — structured logging made simple.

## Installation

```toml
[dependencies]
timberlogs = "1"
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
```

## Quick Start

```rust
use timberlogs::{TimberlogsClient, TimberlogsConfig, Environment};

#[tokio::main]
async fn main() {
    let mut client = TimberlogsClient::new(TimberlogsConfig {
        source: "my-service".into(),
        environment: Environment::Production,
        api_key: "tb_live_xxxxx".into(),
        version: Some("1.0.0".into()),
        ..Default::default()
    });

    client.info("Server started", None).await.unwrap();
    client.error("Something went wrong", None).await.unwrap();

    client.disconnect().await.unwrap();
}
```

## Configuration

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `source` | `String` | required | Application/service name |
| `environment` | `Environment` | required | `Development`, `Staging`, or `Production` |
| `api_key` | `String` | required | Your Timberlogs API key |
| `version` | `Option<String>` | `None` | Application version |
| `user_id` | `Option<String>` | `None` | Default user ID |
| `session_id` | `Option<String>` | `None` | Default session ID |
| `dataset` | `Option<String>` | `None` | Default dataset |
| `batch_size` | `Option<usize>` | `10` | Logs to batch before sending |
| `flush_interval_ms` | `Option<u64>` | `5000` | Auto-flush interval in ms |
| `min_level` | `Option<LogLevel>` | `Debug` | Minimum level to send |
| `retry` | `Option<RetryConfig>` | 3 retries, exponential backoff | Retry configuration |
| `on_error` | `Option<ErrorCallback>` | `None` | Callback invoked on flush failures |

## Log Levels

```rust
client.debug("Debug message", None).await?;
client.info("Info message", None).await?;
client.warn("Warning message", None).await?;
client.error("Error message", None).await?;
```

## Structured Data

```rust
use std::collections::HashMap;
use serde_json::json;

let mut data = HashMap::new();
data.insert("port".into(), json!(3000));
data.insert("host".into(), json!("0.0.0.0"));

client.info("Server started", Some(data)).await?;
```

## Tags

```rust
use timberlogs::{LogEntry, LogLevel};

client.log(LogEntry {
    level: LogLevel::Info,
    message: "User signed in".into(),
    tags: Some(vec!["auth".into(), "login".into()]),
    ..Default::default()
}).await?;
```

## Full LogEntry Fields

```rust
use timberlogs::{LogEntry, LogLevel};

client.log(LogEntry {
    level: LogLevel::Error,
    message: "Request failed".into(),
    data: Some(HashMap::from([("url".into(), json!("/api/users"))])),
    user_id: Some("user_123".into()),
    session_id: Some("sess_abc".into()),
    request_id: Some("req_xyz".into()),
    error_name: Some("TimeoutError".into()),
    error_stack: Some("at handler:42".into()),
    tags: Some(vec!["api".into()]),
    dataset: Some("web-logs".into()),
    timestamp: Some(1700000000000), // Unix ms, defaults to now if omitted
    ip_address: Some("192.168.1.1".into()),
    country: Some("US".into()),
    ..Default::default()
}).await?;
```

## Dynamic User & Session

Set or update user/session IDs at runtime. These are applied to all subsequent logs.

```rust
client.set_user_id(Some("user_123".into())).await;
client.set_session_id(Some("sess_abc".into())).await;

client.info("Now associated with user", None).await?;

// Clear them
client.set_user_id(None).await;
```

## Flow Tracking

Flows group related logs into a named sequence with auto-incrementing step indices.

```rust
let mut flow = client.flow("user-checkout").await?;

flow.info("Cart loaded", None).await?;       // step 0
flow.info("Payment started", None).await?;   // step 1
flow.info("Order confirmed", None).await?;   // step 2
```

Each flow gets a server-generated ID. The `Flow` struct provides the same convenience methods as the client (`debug`, `info`, `warn`, `error`) plus `log_with_level` for custom levels and tags.

## Raw Ingestion

Ingest logs in alternative formats without structured parsing.

```rust
use timberlogs::{RawFormat, IngestRawOptions, Environment};

// Simple raw JSON
client.ingest_raw(
    r#"{"msg":"hello","level":"info"}"#,
    RawFormat::Json,
    None,
).await?;

// CSV with options
client.ingest_raw(
    "level,message\ninfo,hello\nerror,fail",
    RawFormat::Csv,
    Some(IngestRawOptions {
        source: Some("import-job".into()),
        environment: Some(Environment::Production),
        level: Some(LogLevel::Info),
        dataset: Some("csv-import".into()),
    }),
).await?;
```

Supported formats: `Json`, `Jsonl`, `Syslog`, `Text`, `Csv`, `Obl`.

## Error Handling

### on_error Callback

Background flushes run on a timer and can fail silently. Use `on_error` to capture these failures.

```rust
let client = TimberlogsClient::new(TimberlogsConfig {
    on_error: Some(Box::new(|err| {
        eprintln!("Timberlogs flush error: {err}");
    })),
    ..Default::default()
});
```

### Handling Errors Directly

All async methods return `Result<_, TimberlogsError>`:

```rust
use timberlogs::TimberlogsError;

match client.info("test", None).await {
    Ok(()) => {},
    Err(TimberlogsError::Validation(msg)) => eprintln!("Invalid: {msg}"),
    Err(TimberlogsError::Http { status, body }) => eprintln!("HTTP {status}: {body}"),
    Err(TimberlogsError::Request(e)) => eprintln!("Network error: {e}"),
    Err(e) => eprintln!("Other: {e}"),
}
```

## Flush & Disconnect

Logs are batched and sent automatically. You can also flush manually or disconnect gracefully.

```rust
// Force send all buffered logs
client.flush().await?;

// Flush and stop the background timer
client.disconnect().await?;
```

Always call `disconnect()` before your application exits to ensure all buffered logs are sent.

## Retry Configuration

```rust
use timberlogs::RetryConfig;

let client = TimberlogsClient::new(TimberlogsConfig {
    retry: Some(RetryConfig {
        max_retries: 5,
        initial_delay_ms: 500,
        max_delay_ms: 60000,
    }),
    ..Default::default()
});
```

Retries use exponential backoff, doubling the delay on each attempt up to `max_delay_ms`.

## API Reference

### TimberlogsClient

| Method | Description |
|--------|-------------|
| `new(config)` | Create a new client with the given configuration |
| `debug(msg, data)` | Log at Debug level |
| `info(msg, data)` | Log at Info level |
| `warn(msg, data)` | Log at Warn level |
| `error(msg, data)` | Log at Error level |
| `log(entry)` | Log a full `LogEntry` |
| `flow(name)` | Create a new flow and return a `Flow` handle |
| `ingest_raw(body, format, options)` | Ingest raw-formatted logs |
| `set_user_id(id)` | Set or clear the default user ID |
| `set_session_id(id)` | Set or clear the default session ID |
| `flush()` | Manually flush all buffered logs |
| `disconnect()` | Flush and stop the background flush timer |

### Flow

| Method | Description |
|--------|-------------|
| `debug(msg, data)` | Log at Debug level within the flow |
| `info(msg, data)` | Log at Info level within the flow |
| `warn(msg, data)` | Log at Warn level within the flow |
| `error(msg, data)` | Log at Error level within the flow |
| `log_with_level(level, msg, data, tags)` | Log with custom level and tags |

### Enums

| Enum | Variants |
|------|----------|
| `LogLevel` | `Debug`, `Info`, `Warn`, `Error` |
| `Environment` | `Development`, `Staging`, `Production` |
| `RawFormat` | `Json`, `Jsonl`, `Syslog`, `Text`, `Csv`, `Obl` |

## License

MIT
