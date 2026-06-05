//! Append-only JSONL persistence for restart replay.

use std::path::{Path, PathBuf};

use tokio::fs::{self, OpenOptions};
use tokio::io::AsyncWriteExt;

use crate::types::LiveSignal;

pub fn default_persist_path() -> PathBuf {
    std::env::var("SIGNAL_BUFFER_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(".cache/signal-buffer.jsonl"))
}

/// Load the tail of a JSONL file (newest records at end of file).
pub async fn load_jsonl(path: &Path, max_lines: usize) -> Vec<LiveSignal> {
    let content = match fs::read_to_string(path).await {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    content
        .lines()
        .rev()
        .take(max_lines)
        .filter_map(|line| serde_json::from_str::<LiveSignal>(line).ok())
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect()
}

/// Append one signal as a JSON line (best-effort).
pub async fn append_jsonl(path: &Path, signal: &LiveSignal) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent).await;
    }
    let line = match serde_json::to_string(signal) {
        Ok(l) => format!("{l}\n"),
        Err(_) => return,
    };
    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .await
    {
        let _ = file.write_all(line.as_bytes()).await;
    }
}
