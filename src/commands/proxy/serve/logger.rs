// This is free and unencumbered software released into the public domain.

//! Optional request/response body logging for `asimov proxy serve`.
//!
//! Enabled by setting `ASIMOV_PROXY_LOG_FILE` to a file path; bodies are
//! appended to that file. Response bodies are logged chunk by chunk as they
//! are streamed through the proxy.

use std::{
    fs::{File, OpenOptions},
    io::{self, Write as _},
    path::Path,
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Clone)]
pub struct BodyLogger {
    file: Arc<Mutex<File>>,
}

impl BodyLogger {
    /// Constructs a logger from `ASIMOV_PROXY_LOG_FILE`, if set.
    pub fn from_env() -> io::Result<Option<Self>> {
        match std::env::var("ASIMOV_PROXY_LOG_FILE") {
            Ok(path) if !path.is_empty() => Self::open(path.as_ref()).map(Some),
            _ => Ok(None),
        }
    }

    pub fn open(path: &Path) -> io::Result<Self> {
        let file = OpenOptions::new().create(true).append(true).open(path)?;
        Ok(Self {
            file: Arc::new(Mutex::new(file)),
        })
    }

    /// Logs a complete (buffered) request body.
    pub fn log_request_body(&self, data: &[u8]) {
        self.log("request body", data);
    }

    /// Logs a single chunk of a (possibly streamed) response body.
    pub fn log_response_chunk(&self, data: &[u8]) {
        self.log("response body chunk", data);
    }

    fn log(&self, label: &str, data: &[u8]) {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_secs())
            .unwrap_or_default();
        let Ok(mut file) = self.file.lock() else {
            return; // the lock was poisoned; drop the log entry
        };
        let _ = writeln!(
            file,
            "--- {} @{} ({} bytes) ---",
            label,
            timestamp,
            data.len()
        );
        let _ = file.write_all(data);
        let _ = writeln!(file);
    }
}
