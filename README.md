# timberlogs

Rust SDK for [Timberlogs](https://timberlogs.dev) — structured logging made simple.

## Installation

```toml
[dependencies]
timberlogs = "0.1"
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

## License

MIT
