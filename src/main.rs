mod action;
mod app;
mod cli;
mod collector;
mod config;
mod doctor;
mod format;
mod health;
mod logs;
mod model;
mod report;
mod runtime;
mod sockets;
mod terminal;
mod ui;

use std::{fs, path::Path};

use clap::Parser;
use color_eyre::eyre::{Result, eyre};
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

use crate::{
    app::Tab,
    cli::{Cli, Command, OutputFormat},
    collector::{LocalCollector, SnapshotSource},
    terminal::RunMode,
};

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    terminal::install_panic_hook();
    let _log_guard = init_logging();
    let cli = Cli::parse();

    match cli.command {
        Some(Command::Demo) => {
            terminal::run(
                RunMode::Demo,
                config::AppConfig::default(),
                None,
                Tab::Overview,
            )
            .await
        }
        Some(Command::Init { path }) => {
            let path = config::init(&path)?;
            println!("Created {}", path.display());
            Ok(())
        }
        Some(Command::Processes) => {
            let (config, path) = load_for_target(cli.target.as_deref())?;
            terminal::run(RunMode::Local, config, path, Tab::Processes).await
        }
        Some(Command::Logs) => {
            let (config, path) = load_for_target(cli.target.as_deref())?;
            terminal::run(RunMode::Local, config, path, Tab::Logs).await
        }
        Some(Command::Ports) => {
            let (config, path) = load_for_target(cli.target.as_deref())?;
            terminal::run(RunMode::Local, config, path, Tab::Ports).await
        }
        Some(Command::Check { config, json }) => {
            let (config, path) = config::load_or_default(config.as_deref())?;
            if config.service.is_empty() {
                return Err(eyre!(
                    "no services configured{}",
                    path.map(|value| format!(" in {}", value.display()))
                        .unwrap_or_default()
                ));
            }
            let statuses = health::check_all(&config.service).await;
            if json {
                println!("{}", serde_json::to_string_pretty(&statuses)?);
            } else {
                for status in &statuses {
                    println!(
                        "{:<10} {:<20} {:<8} {}",
                        format!("{:?}", status.state),
                        status.name,
                        status
                            .latency_ms
                            .map(|value| format!("{value}ms"))
                            .unwrap_or_else(|| "—".to_owned()),
                        status.target
                    );
                }
            }
            let failed = statuses
                .iter()
                .any(|status| status.state == model::HealthState::Unhealthy);
            if failed {
                std::process::exit(2);
            }
            Ok(())
        }
        Some(Command::Report { format, config }) => {
            let (config, _) = config::load_or_default(config.as_deref())?;
            let mut collector = LocalCollector::new();
            let mut snapshot = collector.sample();
            snapshot.sockets = sockets::collect().unwrap_or_default();
            snapshot.services = health::check_all(&config.service).await;
            match format {
                OutputFormat::Text => print!("{}", report::text(&snapshot)),
                OutputFormat::Json => println!("{}", report::json(&snapshot)?),
            }
            Ok(())
        }
        Some(Command::Doctor { json }) => {
            let report = doctor::inspect();
            if json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                print!("{}", doctor::text(&report));
            }
            if report.checks.iter().any(|check| !check.ok) {
                std::process::exit(1);
            }
            Ok(())
        }
        None => {
            let (config, path) = load_for_target(cli.target.as_deref())?;
            terminal::run(RunMode::Local, config, path, Tab::Overview).await
        }
    }
}

fn load_for_target(
    target: Option<&Path>,
) -> Result<(config::AppConfig, Option<std::path::PathBuf>)> {
    match config::discover_path(target) {
        Some(path) => config::load(&path)
            .map(|config| (config, Some(path)))
            .map_err(Into::into),
        None => Ok((config::AppConfig::default(), None)),
    }
}

fn init_logging() -> Option<tracing_appender::non_blocking::WorkerGuard> {
    let project = directories::ProjectDirs::from("dev", "Windorion", "wsentry")?;
    let directory = project.data_local_dir().join("logs");
    fs::create_dir_all(&directory).ok()?;
    let appender = tracing_appender::rolling::RollingFileAppender::builder()
        .rotation(tracing_appender::rolling::Rotation::DAILY)
        .filename_prefix("wsentry")
        .filename_suffix("log")
        .max_log_files(7)
        .build(directory)
        .ok()?;
    let (writer, guard) = tracing_appender::non_blocking(appender);
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::registry()
        .with(filter)
        .with(
            tracing_subscriber::fmt::layer()
                .with_ansi(false)
                .with_writer(writer),
        )
        .try_init()
        .ok()?;
    Some(guard)
}
