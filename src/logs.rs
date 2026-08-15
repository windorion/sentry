use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
};

use crate::{
    config::LogConfig,
    model::{LogEntry, LogLevel, LogSourceStatus},
};

const INITIAL_READ_BYTES: u64 = 256 * 1024;
const MAX_APPEND_BYTES: u64 = 1024 * 1024;

#[derive(Debug)]
struct SourceState {
    name: String,
    path: PathBuf,
    offset: u64,
    initialized: bool,
    pending: Vec<u8>,
}

#[derive(Debug, Default)]
pub struct LogBatch {
    pub entries: Vec<LogEntry>,
    pub sources: Vec<LogSourceStatus>,
}

#[derive(Debug)]
pub struct LogTailer {
    sources: Vec<SourceState>,
    sequence: u64,
}

impl LogTailer {
    pub fn new(configs: &[LogConfig], base_directory: &Path) -> Self {
        let sources = configs
            .iter()
            .map(|config| SourceState {
                name: config.name.clone(),
                path: if config.path.is_absolute() {
                    config.path.clone()
                } else {
                    base_directory.join(&config.path)
                },
                offset: 0,
                initialized: false,
                pending: Vec::new(),
            })
            .collect();
        Self {
            sources,
            sequence: 0,
        }
    }

    pub fn poll(&mut self) -> LogBatch {
        let mut batch = LogBatch::default();
        for source in &mut self.sources {
            match read_source(source) {
                Ok(lines) => {
                    batch.sources.push(LogSourceStatus {
                        name: source.name.clone(),
                        path: source.path.display().to_string(),
                        available: true,
                        message: None,
                    });
                    for line in lines {
                        self.sequence += 1;
                        batch.entries.push(LogEntry {
                            sequence: self.sequence,
                            source: source.name.clone(),
                            level: detect_level(&line),
                            line,
                        });
                    }
                }
                Err(error) => batch.sources.push(LogSourceStatus {
                    name: source.name.clone(),
                    path: source.path.display().to_string(),
                    available: false,
                    message: Some(error.to_string()),
                }),
            }
        }
        batch
    }
}

fn read_source(source: &mut SourceState) -> std::io::Result<Vec<String>> {
    let mut file = File::open(&source.path)?;
    let length = file.metadata()?.len();
    if length < source.offset {
        source.offset = 0;
        source.initialized = false;
        source.pending.clear();
    }

    let was_initialized = source.initialized;
    let start = if source.initialized {
        source.offset
    } else {
        length.saturating_sub(INITIAL_READ_BYTES)
    };
    file.seek(SeekFrom::Start(start))?;
    let bytes_to_read = length.saturating_sub(start).min(MAX_APPEND_BYTES);
    let mut bytes = Vec::with_capacity(bytes_to_read as usize);
    file.take(bytes_to_read).read_to_end(&mut bytes)?;
    source.offset = start.saturating_add(bytes.len() as u64);
    source.initialized = true;

    if !was_initialized && start > 0 {
        if let Some(newline) = bytes.iter().position(|byte| *byte == b'\n') {
            bytes.drain(..=newline);
        } else {
            bytes.clear();
        }
    }

    let mut buffered = std::mem::take(&mut source.pending);
    buffered.extend(bytes);
    let complete_length = buffered
        .iter()
        .rposition(|byte| *byte == b'\n')
        .map(|index| index + 1)
        .unwrap_or_else(|| {
            if buffered.len() >= MAX_APPEND_BYTES as usize {
                buffered.len()
            } else {
                0
            }
        });
    source.pending = buffered.split_off(complete_length);
    let text = String::from_utf8_lossy(&buffered);
    Ok(text
        .lines()
        .map(str::trim_end)
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect())
}

pub fn detect_level(line: &str) -> LogLevel {
    let upper = line.to_ascii_uppercase();
    let tokens = [
        ("ERROR", LogLevel::Error),
        ("FATAL", LogLevel::Error),
        ("WARN", LogLevel::Warn),
        ("INFO", LogLevel::Info),
        ("DEBUG", LogLevel::Debug),
        ("TRACE", LogLevel::Trace),
    ];
    tokens
        .into_iter()
        .find_map(|(token, level)| upper.contains(token).then_some(level))
        .unwrap_or(LogLevel::Unknown)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::*;

    #[test]
    fn tails_only_new_lines_and_handles_truncation() {
        let directory = tempdir().expect("temporary directory");
        let path = directory.path().join("api.log");
        fs::write(&path, "INFO started\nWARN slow\n").expect("initial log");
        let mut tailer = LogTailer::new(
            &[LogConfig {
                name: "api".to_owned(),
                path: PathBuf::from("api.log"),
            }],
            directory.path(),
        );

        let first = tailer.poll();
        assert_eq!(first.entries.len(), 2);
        assert_eq!(first.entries[1].level, LogLevel::Warn);
        fs::write(&path, "ERROR restarted\n").expect("truncate log");
        let second = tailer.poll();
        assert_eq!(second.entries.len(), 1);
        assert_eq!(second.entries[0].level, LogLevel::Error);
    }

    #[test]
    fn reports_missing_sources_without_panicking() {
        let directory = tempdir().expect("temporary directory");
        let mut tailer = LogTailer::new(
            &[LogConfig {
                name: "missing".to_owned(),
                path: PathBuf::from("missing.log"),
            }],
            directory.path(),
        );

        let batch = tailer.poll();
        assert!(batch.entries.is_empty());
        assert!(!batch.sources[0].available);
    }

    #[test]
    fn preserves_partial_lines_across_polls() {
        use std::io::Write;

        let directory = tempdir().expect("temporary directory");
        let path = directory.path().join("worker.log");
        fs::write(&path, "INFO ready\nWARN queue").expect("initial log");
        let mut tailer = LogTailer::new(
            &[LogConfig {
                name: "worker".to_owned(),
                path: PathBuf::from("worker.log"),
            }],
            directory.path(),
        );

        let first = tailer.poll();
        assert_eq!(first.entries.len(), 1);
        assert_eq!(first.entries[0].line, "INFO ready");

        let mut file = fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .expect("open log for append");
        file.write_all(b" depth high\n").expect("append log");
        let second = tailer.poll();
        assert_eq!(second.entries.len(), 1);
        assert_eq!(second.entries[0].line, "WARN queue depth high");
    }
}
