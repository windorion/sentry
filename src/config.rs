use std::{
    collections::HashSet,
    fs,
    hash::{DefaultHasher, Hash, Hasher},
    io,
    path::{Path, PathBuf},
    time::Duration,
};

use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const CONFIG_FILE_NAME: &str = "wsentry.toml";

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct AppConfig {
    pub schema_version: u32,
    pub refresh_interval_ms: u64,
    pub socket_refresh_interval_ms: u64,
    pub log_refresh_interval_ms: u64,
    pub history_points: usize,
    pub log_buffer_lines: usize,
    pub event_buffer_entries: usize,
    pub thresholds: Thresholds,
    pub service: Vec<ServiceConfig>,
    pub log: Vec<LogConfig>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            schema_version: 1,
            refresh_interval_ms: 1_000,
            socket_refresh_interval_ms: 2_000,
            log_refresh_interval_ms: 500,
            history_points: 120,
            log_buffer_lines: 2_000,
            event_buffer_entries: 500,
            thresholds: Thresholds::default(),
            service: Vec::new(),
            log: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Thresholds {
    pub cpu_percent: f32,
    pub memory_percent: f32,
    pub disk_percent: f32,
}

impl Default for Thresholds {
    fn default() -> Self {
        Self {
            cpu_percent: 85.0,
            memory_percent: 90.0,
            disk_percent: 90.0,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceConfig {
    pub name: String,
    #[serde(default)]
    pub health: Option<String>,
    #[serde(default)]
    pub tcp: Option<String>,
    #[serde(default = "default_interval")]
    pub interval: String,
    #[serde(default = "default_timeout")]
    pub timeout: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LogConfig {
    pub name: String,
    pub path: PathBuf,
}

fn default_interval() -> String {
    "30s".to_owned()
}

fn default_timeout() -> String {
    "3s".to_owned()
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("failed to read {path}: {source}")]
    Read { path: PathBuf, source: io::Error },
    #[error("failed to parse {path}: {source}")]
    Parse {
        path: PathBuf,
        source: toml::de::Error,
    },
    #[error("invalid configuration in {path}: {message}")]
    Invalid { path: PathBuf, message: String },
    #[error("refusing to overwrite existing configuration: {0}")]
    AlreadyExists(PathBuf),
    #[error("failed to write {path}: {source}")]
    Write { path: PathBuf, source: io::Error },
}

pub fn discover_path(target: Option<&Path>) -> Option<PathBuf> {
    let direct = target.map(|path| {
        if path.is_dir() {
            path.join(CONFIG_FILE_NAME)
        } else {
            path.to_path_buf()
        }
    });

    if let Some(path) = direct.filter(|path| path.is_file()) {
        return Some(path);
    }

    std::env::current_dir()
        .ok()
        .map(|directory| directory.join(CONFIG_FILE_NAME))
        .filter(|path| path.is_file())
}

pub fn load(path: &Path) -> Result<AppConfig, ConfigError> {
    let contents = fs::read_to_string(path).map_err(|source| ConfigError::Read {
        path: path.to_path_buf(),
        source,
    })?;
    let config = toml::from_str(&contents).map_err(|source| ConfigError::Parse {
        path: path.to_path_buf(),
        source,
    })?;
    validate(&config).map_err(|message| ConfigError::Invalid {
        path: path.to_path_buf(),
        message,
    })?;
    Ok(config)
}

pub fn validate(config: &AppConfig) -> Result<(), String> {
    if config.schema_version != 1 {
        return Err(format!(
            "unsupported schema_version {}; this build supports schema_version = 1",
            config.schema_version
        ));
    }
    for (name, value) in [
        ("refresh_interval_ms", config.refresh_interval_ms),
        (
            "socket_refresh_interval_ms",
            config.socket_refresh_interval_ms,
        ),
        ("log_refresh_interval_ms", config.log_refresh_interval_ms),
    ] {
        if value < 100 {
            return Err(format!("{name} must be at least 100"));
        }
    }
    if config.history_points == 0 {
        return Err("history_points must be greater than zero".to_owned());
    }
    if config.log_buffer_lines == 0 {
        return Err("log_buffer_lines must be greater than zero".to_owned());
    }
    if config.event_buffer_entries == 0 {
        return Err("event_buffer_entries must be greater than zero".to_owned());
    }
    for (name, value) in [
        ("cpu_percent", config.thresholds.cpu_percent),
        ("memory_percent", config.thresholds.memory_percent),
        ("disk_percent", config.thresholds.disk_percent),
    ] {
        if !(0.0..=100.0).contains(&value) {
            return Err(format!("thresholds.{name} must be between 0 and 100"));
        }
    }

    let mut service_names = HashSet::new();
    for service in &config.service {
        if service.name.trim().is_empty() {
            return Err("service names cannot be empty".to_owned());
        }
        if !service_names.insert(service.name.to_lowercase()) {
            return Err(format!("duplicate service name: {}", service.name));
        }
        if service.health.is_some() == service.tcp.is_some() {
            return Err(format!(
                "service {} requires exactly one of health or tcp",
                service.name
            ));
        }
        if let Some(target) = &service.health {
            let url = reqwest::Url::parse(target).map_err(|error| {
                format!("service {} health URL is invalid: {error}", service.name)
            })?;
            if !matches!(url.scheme(), "http" | "https") {
                return Err(format!(
                    "service {} health URL must use http or https",
                    service.name
                ));
            }
        }
        if let Some(target) = &service.tcp {
            validate_tcp_target(target, &service.name)?;
        }
        let interval = validate_duration(
            &service.interval,
            &format!("service {} interval", service.name),
        )?;
        if interval < Duration::from_millis(100) {
            return Err(format!(
                "service {} interval must be at least 100ms",
                service.name
            ));
        }
        let timeout = validate_duration(
            &service.timeout,
            &format!("service {} timeout", service.name),
        )?;
        if timeout > interval {
            return Err(format!(
                "service {} timeout cannot exceed its interval",
                service.name
            ));
        }
    }

    let mut log_names = HashSet::new();
    for log in &config.log {
        if log.name.trim().is_empty() {
            return Err("log source names cannot be empty".to_owned());
        }
        if log.path.as_os_str().is_empty() {
            return Err(format!("log source {} requires a path", log.name));
        }
        if !log_names.insert(log.name.to_lowercase()) {
            return Err(format!("duplicate log source name: {}", log.name));
        }
    }
    Ok(())
}

fn validate_duration(value: &str, field: &str) -> Result<Duration, String> {
    let duration =
        humantime::parse_duration(value).map_err(|error| format!("{field} is invalid: {error}"))?;
    if duration.is_zero() {
        return Err(format!("{field} must be greater than zero"));
    }
    Ok(duration)
}

fn validate_tcp_target(target: &str, service_name: &str) -> Result<(), String> {
    let (host, port) = target
        .rsplit_once(':')
        .ok_or_else(|| format!("service {service_name} TCP target must be in host:port form"))?;
    if host.trim().is_empty() {
        return Err(format!("service {service_name} TCP target requires a host"));
    }
    let port = port
        .parse::<u16>()
        .map_err(|_| format!("service {service_name} TCP target requires a valid non-zero port"))?;
    if port == 0 {
        return Err(format!(
            "service {service_name} TCP target requires a valid non-zero port"
        ));
    }
    Ok(())
}

pub fn load_or_default(path: Option<&Path>) -> Result<(AppConfig, Option<PathBuf>), ConfigError> {
    let discovered = path.map(Path::to_path_buf).or_else(|| discover_path(None));
    match discovered {
        Some(path) => load(&path).map(|config| (config, Some(path))),
        None => Ok((AppConfig::default(), None)),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FileFingerprint {
    Present { length: u64, hash: u64 },
    Missing,
}

pub struct ConfigWatcher {
    path: PathBuf,
    observed: FileFingerprint,
}

impl ConfigWatcher {
    pub fn new(path: PathBuf) -> Self {
        let observed = fingerprint(&path);
        Self { path, observed }
    }

    pub fn poll(&mut self) -> Option<Result<AppConfig, ConfigError>> {
        let current = fingerprint(&self.path);
        if current == self.observed {
            return None;
        }
        self.observed = current;
        Some(load(&self.path))
    }
}

fn fingerprint(path: &Path) -> FileFingerprint {
    match fs::read(path) {
        Ok(contents) => {
            let mut hasher = DefaultHasher::new();
            contents.hash(&mut hasher);
            FileFingerprint::Present {
                length: contents.len() as u64,
                hash: hasher.finish(),
            }
        }
        Err(_) => FileFingerprint::Missing,
    }
}

pub fn init(directory: &Path) -> Result<PathBuf, ConfigError> {
    let path = directory.join(CONFIG_FILE_NAME);
    if path.exists() {
        return Err(ConfigError::AlreadyExists(path));
    }

    fs::create_dir_all(directory).map_err(|source| ConfigError::Write {
        path: directory.to_path_buf(),
        source,
    })?;
    fs::write(&path, starter_config()).map_err(|source| ConfigError::Write {
        path: path.clone(),
        source,
    })?;
    Ok(path)
}

pub fn starter_config() -> &'static str {
    r#"# Windorion Sentry project configuration
schema_version = 1
refresh_interval_ms = 1000
socket_refresh_interval_ms = 2000
log_refresh_interval_ms = 500
history_points = 120
log_buffer_lines = 2000
event_buffer_entries = 500

[thresholds]
cpu_percent = 85.0
memory_percent = 90.0
disk_percent = 90.0

[[service]]
name = "api"
health = "http://localhost:8080/health"
interval = "10s"
timeout = "3s"

[[service]]
name = "postgres"
tcp = "localhost:5432"
interval = "30s"
timeout = "3s"

# Uncomment to follow a project log file. Relative paths are resolved from
# the directory containing wsentry.toml.
# [[log]]
# name = "api"
# path = "./logs/api.log"
"#
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starter_config_is_valid() {
        let parsed: AppConfig = toml::from_str(starter_config()).expect("starter config parses");
        assert_eq!(parsed.service.len(), 2);
        assert!(parsed.log.is_empty());
        assert_eq!(parsed.refresh_interval_ms, 1_000);
        assert_eq!(parsed.schema_version, 1);
    }

    #[test]
    fn init_does_not_overwrite() {
        let temp = tempfile::tempdir().expect("temp dir");
        let first = init(temp.path()).expect("first init");
        assert!(first.exists());
        assert!(matches!(
            init(temp.path()),
            Err(ConfigError::AlreadyExists(_))
        ));
    }

    #[test]
    fn validation_rejects_ambiguous_service_targets() {
        let mut config = AppConfig::default();
        config.service.push(ServiceConfig {
            name: "api".to_owned(),
            health: Some("http://localhost/health".to_owned()),
            tcp: Some("localhost:80".to_owned()),
            interval: "10s".to_owned(),
            timeout: "1s".to_owned(),
        });
        assert!(validate(&config).is_err());
    }

    #[test]
    fn validation_rejects_duplicate_names_case_insensitively() {
        let config = AppConfig {
            log: vec![
                LogConfig {
                    name: "API".to_owned(),
                    path: PathBuf::from("one.log"),
                },
                LogConfig {
                    name: "api".to_owned(),
                    path: PathBuf::from("two.log"),
                },
            ],
            ..AppConfig::default()
        };
        assert!(validate(&config).is_err());
    }

    #[test]
    fn validation_rejects_zero_sized_buffers_and_fast_refreshes() {
        let mut config = AppConfig {
            log_buffer_lines: 0,
            ..AppConfig::default()
        };
        assert!(validate(&config).is_err());

        config.log_buffer_lines = 100;
        config.socket_refresh_interval_ms = 99;
        assert!(validate(&config).is_err());
    }

    #[test]
    fn parser_rejects_unknown_fields_and_invalid_targets() {
        let unknown = starter_config().replace(
            "schema_version = 1",
            "schema_version = 1\nrefresh_intervall_ms = 1000",
        );
        assert!(toml::from_str::<AppConfig>(&unknown).is_err());

        let mut config = AppConfig::default();
        config.service.push(ServiceConfig {
            name: "api".to_owned(),
            health: Some("file:///tmp/health".to_owned()),
            tcp: None,
            interval: "10s".to_owned(),
            timeout: "1s".to_owned(),
        });
        assert!(validate(&config).is_err());

        config.service[0].health = None;
        config.service[0].tcp = Some("localhost:0".to_owned());
        assert!(validate(&config).is_err());
    }

    #[test]
    fn validation_rejects_unsafe_service_schedules() {
        let mut config = AppConfig::default();
        config.service.push(ServiceConfig {
            name: "api".to_owned(),
            health: Some("http://localhost/health".to_owned()),
            tcp: None,
            interval: "50ms".to_owned(),
            timeout: "20ms".to_owned(),
        });
        assert!(validate(&config).is_err());

        config.service[0].interval = "1s".to_owned();
        config.service[0].timeout = "2s".to_owned();
        assert!(validate(&config).is_err());
    }

    #[test]
    fn watcher_only_reports_config_changes() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = directory.path().join(CONFIG_FILE_NAME);
        fs::write(&path, starter_config()).expect("write initial config");
        let mut watcher = ConfigWatcher::new(path.clone());
        assert!(watcher.poll().is_none());

        let changed = starter_config().replace("history_points = 120", "history_points = 121");
        fs::write(&path, changed).expect("change config");
        let reloaded = watcher
            .poll()
            .expect("change detected")
            .expect("valid config");
        assert_eq!(reloaded.history_points, 121);
        assert!(watcher.poll().is_none());
    }
}
