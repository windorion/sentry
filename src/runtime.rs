use std::{path::PathBuf, time::Duration};

use tokio::{
    sync::mpsc,
    task::JoinHandle,
    time::{MissedTickBehavior, interval},
};

use crate::{
    collector::SnapshotSource,
    config::AppConfig,
    health,
    logs::{LogBatch, LogTailer},
    model::{LogEntry, LogLevel, LogSourceStatus, ServiceStatus, SocketSnapshot, SystemSnapshot},
    sockets,
};

#[derive(Debug)]
pub enum RuntimeUpdate {
    Snapshot(Box<SystemSnapshot>),
    Services(Vec<ServiceStatus>),
    Sockets(Result<Vec<SocketSnapshot>, String>),
    Logs(LogBatch),
    Error(String),
}

#[derive(Clone, Copy, Debug)]
pub enum RuntimeCommand {
    Refresh,
    SetPaused(bool),
    Shutdown,
}

pub struct RuntimeHandle {
    updates: mpsc::Receiver<RuntimeUpdate>,
    commands: mpsc::UnboundedSender<RuntimeCommand>,
    task: JoinHandle<()>,
}

impl RuntimeHandle {
    pub fn drain(&mut self) -> Vec<RuntimeUpdate> {
        let mut updates = Vec::new();
        while let Ok(update) = self.updates.try_recv() {
            updates.push(update);
        }
        updates
    }

    pub fn command(&self, command: RuntimeCommand) {
        let _ = self.commands.send(command);
    }

    pub async fn stop(mut self) {
        let _ = self.commands.send(RuntimeCommand::Shutdown);
        if tokio::time::timeout(Duration::from_secs(1), &mut self.task)
            .await
            .is_err()
        {
            self.task.abort();
            let _ = self.task.await;
        }
    }
}

pub fn start(
    source: Box<dyn SnapshotSource>,
    config: AppConfig,
    base_directory: PathBuf,
    demo: bool,
) -> RuntimeHandle {
    let (update_tx, update_rx) = mpsc::channel(16);
    let (command_tx, command_rx) = mpsc::unbounded_channel();
    let task = tokio::spawn(run_worker(
        source,
        config,
        base_directory,
        demo,
        update_tx,
        command_rx,
    ));
    RuntimeHandle {
        updates: update_rx,
        commands: command_tx,
        task,
    }
}

async fn run_worker(
    source: Box<dyn SnapshotSource>,
    config: AppConfig,
    base_directory: PathBuf,
    demo: bool,
    updates: mpsc::Sender<RuntimeUpdate>,
    mut commands: mpsc::UnboundedReceiver<RuntimeCommand>,
) {
    let mut source = Some(source);
    let mut tailer = Some(LogTailer::new(&config.log, &base_directory));
    let mut paused = false;
    let mut demo_sequence = 0;

    let mut system_tick = interval(Duration::from_millis(config.refresh_interval_ms.max(250)));
    let mut socket_tick = interval(Duration::from_millis(
        config.socket_refresh_interval_ms.max(500),
    ));
    let mut log_tick = interval(Duration::from_millis(
        config.log_refresh_interval_ms.max(100),
    ));
    let mut service_tick = interval(service_interval(&config));
    for timer in [
        &mut system_tick,
        &mut socket_tick,
        &mut log_tick,
        &mut service_tick,
    ] {
        timer.set_missed_tick_behavior(MissedTickBehavior::Skip);
    }
    system_tick.tick().await;

    loop {
        tokio::select! {
            command = commands.recv() => match command {
                Some(RuntimeCommand::Refresh) => {
                    system_tick.reset_immediately();
                    socket_tick.reset_immediately();
                    log_tick.reset_immediately();
                    service_tick.reset_immediately();
                }
                Some(RuntimeCommand::SetPaused(value)) => paused = value,
                Some(RuntimeCommand::Shutdown) | None => break,
            },
            _ = system_tick.tick(), if !paused => {
                let Some(current) = source.take() else { break };
                match sample_in_background(current).await {
                    Ok((next, snapshot)) => {
                        source = Some(next);
                        if updates.send(RuntimeUpdate::Snapshot(Box::new(snapshot))).await.is_err() { break; }
                    }
                    Err(error) => {
                        let _ = updates.send(RuntimeUpdate::Error(format!("system collector stopped: {error}"))).await;
                        break;
                    }
                }
            },
            _ = socket_tick.tick(), if !paused && !demo => {
                let result = tokio::task::spawn_blocking(sockets::collect)
                    .await
                    .map_err(|error| error.to_string())
                    .and_then(|result| result);
                if updates.send(RuntimeUpdate::Sockets(result)).await.is_err() { break; }
            },
            _ = service_tick.tick(), if !paused && !config.service.is_empty() => {
                let statuses = health::check_all(&config.service).await;
                if updates.send(RuntimeUpdate::Services(statuses)).await.is_err() { break; }
            },
            _ = log_tick.tick(), if !paused && (demo || !config.log.is_empty()) => {
                let batch = if demo {
                    demo_sequence += 1;
                    demo_log_batch(demo_sequence)
                } else {
                    let Some(current) = tailer.take() else { break };
                    match tokio::task::spawn_blocking(move || {
                        let mut tailer = current;
                        let batch = tailer.poll();
                        (tailer, batch)
                    }).await {
                        Ok((next, batch)) => {
                            tailer = Some(next);
                            batch
                        }
                        Err(error) => {
                            let _ = updates.send(RuntimeUpdate::Error(format!("log collector stopped: {error}"))).await;
                            break;
                        }
                    }
                };
                if updates.send(RuntimeUpdate::Logs(batch)).await.is_err() { break; }
            },
        }
    }
}

async fn sample_in_background(
    source: Box<dyn SnapshotSource>,
) -> Result<(Box<dyn SnapshotSource>, SystemSnapshot), tokio::task::JoinError> {
    tokio::task::spawn_blocking(move || {
        let mut source = source;
        let snapshot = source.sample();
        (source, snapshot)
    })
    .await
}

fn service_interval(config: &AppConfig) -> Duration {
    config
        .service
        .iter()
        .filter_map(|service| humantime::parse_duration(&service.interval).ok())
        .min()
        .unwrap_or_else(|| Duration::from_secs(30))
        .max(Duration::from_secs(2))
}

fn demo_log_batch(sequence: u64) -> LogBatch {
    let samples = [
        (LogLevel::Info, "HTTP server ready on 127.0.0.1:8080"),
        (LogLevel::Debug, "completed health check in 18ms"),
        (LogLevel::Warn, "worker queue depth reached 128 jobs"),
        (LogLevel::Info, "processed request GET /api/projects"),
        (LogLevel::Error, "worker connection refused; retrying"),
    ];
    let (level, line) = samples[(sequence as usize - 1) % samples.len()];
    LogBatch {
        entries: vec![LogEntry {
            sequence,
            source: if level == LogLevel::Error {
                "worker".to_owned()
            } else {
                "api".to_owned()
            },
            level,
            line: line.to_owned(),
        }],
        sources: vec![
            LogSourceStatus {
                name: "api".to_owned(),
                path: "demo://api".to_owned(),
                available: true,
                message: None,
            },
            LogSourceStatus {
                name: "worker".to_owned(),
                path: "demo://worker".to_owned(),
                available: true,
                message: None,
            },
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collector::DemoCollector;

    #[test]
    fn demo_logs_cycle_through_levels() {
        assert_eq!(demo_log_batch(1).entries[0].level, LogLevel::Info);
        assert_eq!(demo_log_batch(3).entries[0].level, LogLevel::Warn);
        assert_eq!(demo_log_batch(5).entries[0].level, LogLevel::Error);
    }

    #[tokio::test]
    async fn worker_accepts_commands_and_shuts_down() {
        let worker = start(
            Box::new(DemoCollector::new()),
            AppConfig::default(),
            PathBuf::from("."),
            true,
        );
        worker.command(RuntimeCommand::Refresh);
        worker.command(RuntimeCommand::SetPaused(true));
        tokio::time::timeout(Duration::from_secs(2), worker.stop())
            .await
            .expect("worker stops within timeout");
    }
}
