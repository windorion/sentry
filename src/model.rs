use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize)]
pub struct SystemSnapshot {
    pub timestamp: DateTime<Utc>,
    pub host_name: String,
    pub os_name: String,
    pub kernel_version: String,
    pub uptime_seconds: Option<u64>,
    pub cpu_usage_percent: f32,
    pub load_average: LoadAverage,
    pub memory_used_bytes: u64,
    pub memory_total_bytes: u64,
    pub swap_used_bytes: u64,
    pub swap_total_bytes: u64,
    pub processes: Vec<ProcessSnapshot>,
    pub disks: Vec<DiskSnapshot>,
    pub networks: Vec<NetworkSnapshot>,
    pub sockets: Vec<SocketSnapshot>,
    pub services: Vec<ServiceStatus>,
}

#[derive(Clone, Copy, Debug, Default, Serialize)]
pub struct LoadAverage {
    pub one: f64,
    pub five: f64,
    pub fifteen: f64,
}

#[derive(Clone, Debug, Serialize)]
pub struct ProcessSnapshot {
    pub pid: u32,
    pub name: String,
    pub cpu_usage_percent: f32,
    pub memory_bytes: u64,
    pub virtual_memory_bytes: u64,
    pub status: String,
    pub run_time_seconds: u64,
    pub command: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct DiskSnapshot {
    pub name: String,
    pub mount_point: String,
    pub file_system: String,
    pub kind: String,
    pub total_bytes: u64,
    pub available_bytes: u64,
}

impl DiskSnapshot {
    pub fn used_bytes(&self) -> u64 {
        self.total_bytes.saturating_sub(self.available_bytes)
    }

    pub fn used_ratio(&self) -> f64 {
        if self.total_bytes == 0 {
            0.0
        } else {
            self.used_bytes() as f64 / self.total_bytes as f64
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct NetworkSnapshot {
    pub interface: String,
    pub received_bytes: u64,
    pub transmitted_bytes: u64,
    pub total_received_bytes: u64,
    pub total_transmitted_bytes: u64,
    pub packets_received: u64,
    pub packets_transmitted: u64,
    pub errors_received: u64,
    pub errors_transmitted: u64,
}

#[derive(Clone, Debug, Serialize)]
pub struct SocketSnapshot {
    pub protocol: SocketProtocol,
    pub local_address: String,
    pub local_port: u16,
    pub remote_address: Option<String>,
    pub remote_port: Option<u16>,
    pub state: String,
    pub associated_pids: Vec<u32>,
    pub process_names: Vec<String>,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SocketProtocol {
    Tcp,
    Udp,
}

#[derive(Clone, Debug, Serialize)]
pub struct LogEntry {
    pub sequence: u64,
    pub source: String,
    pub level: LogLevel,
    pub line: String,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
    Unknown,
}

impl LogLevel {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Trace => "TRACE",
            Self::Debug => "DEBUG",
            Self::Info => "INFO",
            Self::Warn => "WARN",
            Self::Error => "ERROR",
            Self::Unknown => "—",
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct LogSourceStatus {
    pub name: String,
    pub path: String,
    pub available: bool,
    pub message: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct EventEntry {
    pub sequence: u64,
    pub timestamp: DateTime<Utc>,
    pub level: EventLevel,
    pub key: String,
    pub message: String,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum EventLevel {
    Alert,
    Recovery,
    Info,
}

impl EventLevel {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Alert => "ALERT",
            Self::Recovery => "RECOVERED",
            Self::Info => "INFO",
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct ServiceStatus {
    pub name: String,
    pub target: String,
    pub state: HealthState,
    pub latency_ms: Option<u128>,
    pub status_code: Option<u16>,
    pub checked_at: DateTime<Utc>,
    pub message: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HealthState {
    Healthy,
    Unhealthy,
    Unknown,
}
