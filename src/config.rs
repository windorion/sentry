use std::{
    fs, io,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const CONFIG_FILE_NAME: &str = "wsentry.toml";

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct AppConfig {
    pub refresh_interval_ms: u64,
    pub history_points: usize,
    pub thresholds: Thresholds,
    pub service: Vec<ServiceConfig>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            refresh_interval_ms: 1_000,
            history_points: 120,
            thresholds: Thresholds::default(),
            service: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
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
    toml::from_str(&contents).map_err(|source| ConfigError::Parse {
        path: path.to_path_buf(),
        source,
    })
}

pub fn load_or_default(path: Option<&Path>) -> Result<(AppConfig, Option<PathBuf>), ConfigError> {
    let discovered = path.map(Path::to_path_buf).or_else(|| discover_path(None));
    match discovered {
        Some(path) => load(&path).map(|config| (config, Some(path))),
        None => Ok((AppConfig::default(), None)),
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
refresh_interval_ms = 1000
history_points = 120

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
"#
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starter_config_is_valid() {
        let parsed: AppConfig = toml::from_str(starter_config()).expect("starter config parses");
        assert_eq!(parsed.service.len(), 2);
        assert_eq!(parsed.refresh_interval_ms, 1_000);
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
}
