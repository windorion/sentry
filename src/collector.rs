use chrono::Utc;
use sysinfo::{Disks, Networks, System};

use crate::model::{
    DiskSnapshot, HealthState, LoadAverage, NetworkSnapshot, ProcessSnapshot, ServiceStatus,
    SystemSnapshot,
};

pub trait SnapshotSource {
    fn sample(&mut self) -> SystemSnapshot;
}

pub struct LocalCollector {
    system: System,
    disks: Disks,
    networks: Networks,
}

impl LocalCollector {
    pub fn new() -> Self {
        Self {
            system: System::new_all(),
            disks: Disks::new_with_refreshed_list(),
            networks: Networks::new_with_refreshed_list(),
        }
    }
}

impl Default for LocalCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl SnapshotSource for LocalCollector {
    fn sample(&mut self) -> SystemSnapshot {
        self.system.refresh_all();
        self.disks.refresh(true);
        self.networks.refresh(true);

        let mut processes = self
            .system
            .processes()
            .iter()
            .map(|(pid, process)| ProcessSnapshot {
                pid: pid.as_u32(),
                name: process.name().to_string_lossy().into_owned(),
                cpu_usage_percent: process.cpu_usage(),
                memory_bytes: process.memory(),
                virtual_memory_bytes: process.virtual_memory(),
                status: process.status().to_string(),
                run_time_seconds: process.run_time(),
                command: process
                    .cmd()
                    .iter()
                    .map(|part| part.to_string_lossy())
                    .collect::<Vec<_>>()
                    .join(" "),
            })
            .collect::<Vec<_>>();
        processes.sort_by(|left, right| {
            right
                .cpu_usage_percent
                .total_cmp(&left.cpu_usage_percent)
                .then_with(|| right.memory_bytes.cmp(&left.memory_bytes))
        });

        let disks = self
            .disks
            .list()
            .iter()
            .map(|disk| DiskSnapshot {
                name: disk.name().to_string_lossy().into_owned(),
                mount_point: disk.mount_point().display().to_string(),
                file_system: disk.file_system().to_string_lossy().into_owned(),
                kind: disk.kind().to_string(),
                total_bytes: disk.total_space(),
                available_bytes: disk.available_space(),
            })
            .collect();

        let networks = self
            .networks
            .iter()
            .map(|(interface, data)| NetworkSnapshot {
                interface: interface.clone(),
                received_bytes: data.received(),
                transmitted_bytes: data.transmitted(),
                total_received_bytes: data.total_received(),
                total_transmitted_bytes: data.total_transmitted(),
                packets_received: data.packets_received(),
                packets_transmitted: data.packets_transmitted(),
                errors_received: data.errors_on_received(),
                errors_transmitted: data.errors_on_transmitted(),
            })
            .collect();

        let load = System::load_average();
        SystemSnapshot {
            timestamp: Utc::now(),
            host_name: System::host_name().unwrap_or_else(|| "unknown".to_owned()),
            os_name: System::long_os_version()
                .or_else(System::name)
                .unwrap_or_else(|| std::env::consts::OS.to_owned()),
            kernel_version: System::kernel_version().unwrap_or_else(|| "unknown".to_owned()),
            uptime_seconds: System::uptime(),
            cpu_usage_percent: self.system.global_cpu_usage(),
            load_average: LoadAverage {
                one: load.one,
                five: load.five,
                fifteen: load.fifteen,
            },
            memory_used_bytes: self.system.used_memory(),
            memory_total_bytes: self.system.total_memory(),
            swap_used_bytes: self.system.used_swap(),
            swap_total_bytes: self.system.total_swap(),
            processes,
            disks,
            networks,
            services: Vec::new(),
        }
    }
}

pub struct DemoCollector {
    tick: u64,
}

impl DemoCollector {
    pub fn new() -> Self {
        Self { tick: 0 }
    }
}

impl SnapshotSource for DemoCollector {
    fn sample(&mut self) -> SystemSnapshot {
        self.tick += 1;
        let wave = ((self.tick as f32 / 4.0).sin() + 1.0) / 2.0;
        let cpu = 28.0 + wave * 46.0;
        let memory_total = 16 * 1024 * 1024 * 1024;
        let memory_used = (memory_total as f32 * (0.52 + wave * 0.12)) as u64;

        let processes = vec![
            ProcessSnapshot {
                pid: 4_201,
                name: "api-server".to_owned(),
                cpu_usage_percent: cpu * 0.38,
                memory_bytes: 1_240_000_000,
                virtual_memory_bytes: 2_800_000_000,
                status: "Run".to_owned(),
                run_time_seconds: 8_420,
                command: "./api-server --environment development".to_owned(),
            },
            ProcessSnapshot {
                pid: 4_377,
                name: "postgres".to_owned(),
                cpu_usage_percent: cpu * 0.21,
                memory_bytes: 860_000_000,
                virtual_memory_bytes: 1_900_000_000,
                status: "Sleep".to_owned(),
                run_time_seconds: 22_180,
                command: "postgres -D ./data".to_owned(),
            },
            ProcessSnapshot {
                pid: 4_612,
                name: "worker".to_owned(),
                cpu_usage_percent: cpu * 0.14,
                memory_bytes: 430_000_000,
                virtual_memory_bytes: 980_000_000,
                status: "Run".to_owned(),
                run_time_seconds: 6_221,
                command: "./worker --queue default".to_owned(),
            },
        ];

        SystemSnapshot {
            timestamp: Utc::now(),
            host_name: "demo-workstation".to_owned(),
            os_name: "Windorion OS 1.0".to_owned(),
            kernel_version: "demo-kernel".to_owned(),
            uptime_seconds: 391_420 + self.tick,
            cpu_usage_percent: cpu,
            load_average: LoadAverage {
                one: 1.24,
                five: 1.08,
                fifteen: 0.92,
            },
            memory_used_bytes: memory_used,
            memory_total_bytes: memory_total,
            swap_used_bytes: 540_000_000,
            swap_total_bytes: 2 * 1024 * 1024 * 1024,
            processes,
            disks: vec![DiskSnapshot {
                name: "system".to_owned(),
                mount_point: "/".to_owned(),
                file_system: "apfs".to_owned(),
                kind: "SSD".to_owned(),
                total_bytes: 512 * 1024 * 1024 * 1024,
                available_bytes: 206 * 1024 * 1024 * 1024,
            }],
            networks: vec![NetworkSnapshot {
                interface: "en0".to_owned(),
                received_bytes: 820_000 + self.tick * 8_000,
                transmitted_bytes: 210_000 + self.tick * 3_000,
                total_received_bytes: 24_800_000_000,
                total_transmitted_bytes: 8_400_000_000,
                packets_received: 42_000,
                packets_transmitted: 18_000,
                errors_received: 0,
                errors_transmitted: 0,
            }],
            services: vec![
                ServiceStatus {
                    name: "api".to_owned(),
                    target: "http://127.0.0.1:8080/health".to_owned(),
                    state: HealthState::Healthy,
                    latency_ms: Some(18),
                    status_code: Some(200),
                    checked_at: Utc::now(),
                    message: None,
                },
                ServiceStatus {
                    name: "postgres".to_owned(),
                    target: "127.0.0.1:5432".to_owned(),
                    state: HealthState::Healthy,
                    latency_ms: Some(4),
                    status_code: None,
                    checked_at: Utc::now(),
                    message: None,
                },
                ServiceStatus {
                    name: "worker".to_owned(),
                    target: "127.0.0.1:9000".to_owned(),
                    state: HealthState::Unhealthy,
                    latency_ms: Some(302),
                    status_code: None,
                    checked_at: Utc::now(),
                    message: Some("connection refused".to_owned()),
                },
            ],
        }
    }
}
