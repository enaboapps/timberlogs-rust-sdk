use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{interval, Duration};

use crate::error::TimberlogsError;
use crate::types::{BatchPayload, CreateLogArgs, Environment, FlowResponse, IngestResponse, LogEntry, LogLevel};

const TIMBERLOGS_ENDPOINT: &str = "https://timberlogs-ingest.enaboapps.workers.dev/v1/logs";
const TIMBERLOGS_FLOWS_ENDPOINT: &str = "https://timberlogs-ingest.enaboapps.workers.dev/v1/flows";

const DEFAULT_BATCH_SIZE: usize = 10;
const DEFAULT_FLUSH_INTERVAL_MS: u64 = 5000;
const DEFAULT_MAX_RETRIES: u32 = 3;
const DEFAULT_INITIAL_DELAY_MS: u64 = 1000;
const DEFAULT_MAX_DELAY_MS: u64 = 30000;

pub struct RetryConfig {
    pub max_retries: u32,
    pub initial_delay_ms: u64,
    pub max_delay_ms: u64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: DEFAULT_MAX_RETRIES,
            initial_delay_ms: DEFAULT_INITIAL_DELAY_MS,
            max_delay_ms: DEFAULT_MAX_DELAY_MS,
        }
    }
}

pub struct TimberlogsConfig {
    pub source: String,
    pub environment: Environment,
    pub api_key: String,
    pub version: Option<String>,
    pub user_id: Option<String>,
    pub session_id: Option<String>,
    pub dataset: Option<String>,
    pub batch_size: Option<usize>,
    pub flush_interval_ms: Option<u64>,
    pub min_level: Option<LogLevel>,
    pub retry: Option<RetryConfig>,
}

impl Default for TimberlogsConfig {
    fn default() -> Self {
        Self {
            source: String::new(),
            environment: Environment::Development,
            api_key: String::new(),
            version: None,
            user_id: None,
            session_id: None,
            dataset: None,
            batch_size: None,
            flush_interval_ms: None,
            min_level: None,
            retry: None,
        }
    }
}

struct ClientInner {
    queue: Vec<CreateLogArgs>,
    http: reqwest::Client,
}

pub struct TimberlogsClient {
    config: Arc<ClientConfig>,
    inner: Arc<Mutex<ClientInner>>,
    flush_handle: Option<tokio::task::JoinHandle<()>>,
}

struct ClientConfig {
    source: String,
    environment: Environment,
    api_key: String,
    version: Option<String>,
    user_id: Mutex<Option<String>>,
    session_id: Mutex<Option<String>>,
    dataset: Option<String>,
    batch_size: usize,
    min_level: LogLevel,
    retry: RetryConfig,
}

fn validate_entry(entry: &LogEntry) -> Result<(), TimberlogsError> {
    if entry.message.is_empty() {
        return Err(TimberlogsError::Validation(
            "message must not be empty".into(),
        ));
    }
    if entry.message.len() > 10_000 {
        return Err(TimberlogsError::Validation(format!(
            "message exceeds 10000 characters: {}",
            entry.message.len()
        )));
    }
    if let Some(ref tags) = entry.tags {
        if tags.len() > 20 {
            return Err(TimberlogsError::Validation(format!(
                "tags must have at most 20 items, got {}",
                tags.len()
            )));
        }
        for (i, tag) in tags.iter().enumerate() {
            if tag.len() > 50 {
                return Err(TimberlogsError::Validation(format!(
                    "tags[{i}] exceeds 50 characters: {}",
                    tag.len()
                )));
            }
        }
    }
    if let Some(step) = entry.step_index {
        if step > 1000 {
            return Err(TimberlogsError::Validation(format!(
                "step_index must be 0-1000, got {step}"
            )));
        }
    }
    Ok(())
}

impl TimberlogsClient {
    pub fn new(config: TimberlogsConfig) -> Self {
        let client_config = Arc::new(ClientConfig {
            source: config.source,
            environment: config.environment,
            api_key: config.api_key,
            version: config.version,
            user_id: Mutex::new(config.user_id),
            session_id: Mutex::new(config.session_id),
            dataset: config.dataset,
            batch_size: config.batch_size.unwrap_or(DEFAULT_BATCH_SIZE),
            min_level: config.min_level.unwrap_or(LogLevel::Debug),
            retry: config.retry.unwrap_or_default(),
        });

        let inner = Arc::new(Mutex::new(ClientInner {
            queue: Vec::new(),
            http: reqwest::Client::new(),
        }));

        let flush_interval = config
            .flush_interval_ms
            .unwrap_or(DEFAULT_FLUSH_INTERVAL_MS);

        let flush_config = Arc::clone(&client_config);
        let flush_inner = Arc::clone(&inner);
        let flush_handle = tokio::spawn(async move {
            let mut ticker = interval(Duration::from_millis(flush_interval));
            loop {
                ticker.tick().await;
                let _ = flush_batch(&flush_config, &flush_inner).await;
            }
        });

        Self {
            config: client_config,
            inner,
            flush_handle: Some(flush_handle),
        }
    }

    pub async fn set_user_id(&self, user_id: Option<String>) {
        *self.config.user_id.lock().await = user_id;
    }

    pub async fn set_session_id(&self, session_id: Option<String>) {
        *self.config.session_id.lock().await = session_id;
    }

    pub async fn debug(
        &self,
        message: impl Into<String>,
        data: Option<std::collections::HashMap<String, serde_json::Value>>,
    ) -> Result<(), TimberlogsError> {
        self.log(LogEntry {
            level: LogLevel::Debug,
            message: message.into(),
            data,
            ..Default::default()
        })
        .await
    }

    pub async fn info(
        &self,
        message: impl Into<String>,
        data: Option<std::collections::HashMap<String, serde_json::Value>>,
    ) -> Result<(), TimberlogsError> {
        self.log(LogEntry {
            level: LogLevel::Info,
            message: message.into(),
            data,
            ..Default::default()
        })
        .await
    }

    pub async fn warn(
        &self,
        message: impl Into<String>,
        data: Option<std::collections::HashMap<String, serde_json::Value>>,
    ) -> Result<(), TimberlogsError> {
        self.log(LogEntry {
            level: LogLevel::Warn,
            message: message.into(),
            data,
            ..Default::default()
        })
        .await
    }

    pub async fn error(
        &self,
        message: impl Into<String>,
        data: Option<std::collections::HashMap<String, serde_json::Value>>,
    ) -> Result<(), TimberlogsError> {
        self.log(LogEntry {
            level: LogLevel::Error,
            message: message.into(),
            data,
            ..Default::default()
        })
        .await
    }

    pub async fn log(&self, entry: LogEntry) -> Result<(), TimberlogsError> {
        if entry.level < self.config.min_level {
            return Ok(());
        }

        validate_entry(&entry)?;

        let user_id = entry
            .user_id
            .or_else(|| self.config.user_id.try_lock().ok()?.clone());
        let session_id = entry
            .session_id
            .or_else(|| self.config.session_id.try_lock().ok()?.clone());

        let args = CreateLogArgs {
            level: entry.level,
            message: entry.message,
            source: self.config.source.clone(),
            environment: self.config.environment,
            version: self.config.version.clone(),
            user_id,
            session_id,
            request_id: entry.request_id,
            data: entry.data,
            error_name: entry.error_name,
            error_stack: entry.error_stack,
            tags: entry.tags,
            flow_id: entry.flow_id,
            step_index: entry.step_index,
            dataset: entry.dataset.or_else(|| self.config.dataset.clone()),
        };

        let should_flush = {
            let mut inner = self.inner.lock().await;
            inner.queue.push(args);
            inner.queue.len() >= self.config.batch_size
        };

        if should_flush {
            flush_batch(&self.config, &self.inner).await?;
        }

        Ok(())
    }

    pub async fn flow(&self, name: impl Into<String>) -> Result<Flow<'_>, TimberlogsError> {
        let name = name.into();
        let http = {
            let inner = self.inner.lock().await;
            inner.http.clone()
        };

        let response = http
            .post(TIMBERLOGS_FLOWS_ENDPOINT)
            .header("Content-Type", "application/json")
            .header("X-API-Key", &self.config.api_key)
            .json(&serde_json::json!({ "name": name }))
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = response.text().await.unwrap_or_default();
            return Err(TimberlogsError::Http { status, body });
        }

        let data: FlowResponse = response.json().await?;
        Ok(Flow {
            id: data.flow_id,
            name: data.name,
            step_index: 0,
            client: self,
        })
    }

    pub async fn flush(&self) -> Result<(), TimberlogsError> {
        flush_batch(&self.config, &self.inner).await
    }

    pub async fn disconnect(&mut self) -> Result<(), TimberlogsError> {
        if let Some(handle) = self.flush_handle.take() {
            handle.abort();
        }
        self.flush().await
    }
}

impl Drop for TimberlogsClient {
    fn drop(&mut self) {
        if let Some(handle) = self.flush_handle.take() {
            handle.abort();
        }
    }
}

pub struct Flow<'a> {
    pub id: String,
    pub name: String,
    step_index: u32,
    client: &'a TimberlogsClient,
}

impl<'a> Flow<'a> {
    pub async fn debug(
        &mut self,
        message: impl Into<String>,
        data: Option<std::collections::HashMap<String, serde_json::Value>>,
    ) -> Result<&mut Self, TimberlogsError> {
        self.log_with_level(LogLevel::Debug, message, data, None).await
    }

    pub async fn info(
        &mut self,
        message: impl Into<String>,
        data: Option<std::collections::HashMap<String, serde_json::Value>>,
    ) -> Result<&mut Self, TimberlogsError> {
        self.log_with_level(LogLevel::Info, message, data, None).await
    }

    pub async fn warn(
        &mut self,
        message: impl Into<String>,
        data: Option<std::collections::HashMap<String, serde_json::Value>>,
    ) -> Result<&mut Self, TimberlogsError> {
        self.log_with_level(LogLevel::Warn, message, data, None).await
    }

    pub async fn error(
        &mut self,
        message: impl Into<String>,
        data: Option<std::collections::HashMap<String, serde_json::Value>>,
    ) -> Result<&mut Self, TimberlogsError> {
        self.log_with_level(LogLevel::Error, message, data, None).await
    }

    pub async fn log_with_level(
        &mut self,
        level: LogLevel,
        message: impl Into<String>,
        data: Option<std::collections::HashMap<String, serde_json::Value>>,
        tags: Option<Vec<String>>,
    ) -> Result<&mut Self, TimberlogsError> {
        let step = self.step_index;
        self.step_index += 1;

        self.client
            .log(LogEntry {
                level,
                message: message.into(),
                data,
                tags,
                flow_id: Some(self.id.clone()),
                step_index: Some(step),
                ..Default::default()
            })
            .await?;

        Ok(self)
    }
}

async fn flush_batch(
    config: &ClientConfig,
    inner: &Arc<Mutex<ClientInner>>,
) -> Result<(), TimberlogsError> {
    let (logs, http) = {
        let mut guard = inner.lock().await;
        if guard.queue.is_empty() {
            return Ok(());
        }
        let logs = std::mem::take(&mut guard.queue);
        let http = guard.http.clone();
        (logs, http)
    };

    match send_batch(&http, &config.api_key, &config.retry, &logs).await {
        Ok(()) => Ok(()),
        Err(e) => {
            let mut guard = inner.lock().await;
            let mut requeued = logs;
            requeued.append(&mut guard.queue);
            guard.queue = requeued;
            Err(e)
        }
    }
}

async fn send_batch(
    http: &reqwest::Client,
    api_key: &str,
    retry: &RetryConfig,
    logs: &[CreateLogArgs],
) -> Result<(), TimberlogsError> {
    let payload = BatchPayload {
        logs: logs.to_vec(),
    };

    let mut last_error = None;
    let mut delay = retry.initial_delay_ms;

    for attempt in 0..=retry.max_retries {
        let result = http
            .post(TIMBERLOGS_ENDPOINT)
            .header("Content-Type", "application/json")
            .header("X-API-Key", api_key)
            .json(&payload)
            .send()
            .await;

        match result {
            Ok(response) => {
                if response.status().is_success() {
                    let _body: IngestResponse = response.json().await?;
                    return Ok(());
                }
                let status = response.status().as_u16();
                let body = response.text().await.unwrap_or_default();
                last_error = Some(TimberlogsError::Http { status, body });
            }
            Err(e) => {
                last_error = Some(TimberlogsError::Request(e));
            }
        }

        if attempt < retry.max_retries {
            tokio::time::sleep(Duration::from_millis(delay)).await;
            delay = (delay * 2).min(retry.max_delay_ms);
        }
    }

    Err(last_error.unwrap())
}

impl Default for LogEntry {
    fn default() -> Self {
        Self {
            level: LogLevel::Info,
            message: String::new(),
            data: None,
            user_id: None,
            session_id: None,
            request_id: None,
            error_name: None,
            error_stack: None,
            tags: None,
            flow_id: None,
            step_index: None,
            dataset: None,
        }
    }
}
