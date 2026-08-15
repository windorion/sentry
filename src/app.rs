use std::{collections::VecDeque, path::PathBuf};

use crate::{
    action::Action,
    config::AppConfig,
    format,
    logs::LogBatch,
    model::{
        HealthState, LogEntry, LogSourceStatus, ProcessSnapshot, ServiceStatus, SocketSnapshot,
        SystemSnapshot,
    },
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tab {
    Overview,
    Processes,
    Services,
    Logs,
    Network,
    Ports,
    Disks,
}

impl Tab {
    pub const ALL: [Self; 7] = [
        Self::Overview,
        Self::Processes,
        Self::Services,
        Self::Logs,
        Self::Network,
        Self::Ports,
        Self::Disks,
    ];

    pub const fn title(self) -> &'static str {
        match self {
            Self::Overview => "Overview",
            Self::Processes => "Processes",
            Self::Services => "Services",
            Self::Logs => "Logs",
            Self::Network => "Network",
            Self::Ports => "Ports",
            Self::Disks => "Disks",
        }
    }

    pub fn index(self) -> usize {
        Self::ALL.iter().position(|tab| *tab == self).unwrap_or(0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Search,
}

#[derive(Clone, Copy, Debug)]
pub struct HistoryPoint {
    pub cpu: u64,
    pub memory: u64,
    pub network_rx: u64,
    pub network_tx: u64,
}

pub struct App {
    pub running: bool,
    pub paused: bool,
    pub force_refresh: bool,
    pub tab: Tab,
    pub input_mode: InputMode,
    pub search: String,
    pub show_help: bool,
    pub show_details: bool,
    pub selected_process: usize,
    pub selected_service: usize,
    pub selected_log: usize,
    pub selected_network: usize,
    pub selected_socket: usize,
    pub selected_disk: usize,
    pub snapshot: SystemSnapshot,
    pub history: VecDeque<HistoryPoint>,
    pub config: AppConfig,
    pub config_path: Option<PathBuf>,
    pub logs: VecDeque<LogEntry>,
    pub log_sources: Vec<LogSourceStatus>,
    pub socket_error: Option<String>,
    pub message: Option<String>,
}

impl App {
    pub fn new(
        snapshot: SystemSnapshot,
        config: AppConfig,
        config_path: Option<PathBuf>,
        initial_tab: Tab,
    ) -> Self {
        let mut app = Self {
            running: true,
            paused: false,
            force_refresh: false,
            tab: initial_tab,
            input_mode: InputMode::Normal,
            search: String::new(),
            show_help: false,
            show_details: false,
            selected_process: 0,
            selected_service: 0,
            selected_log: 0,
            selected_network: 0,
            selected_socket: 0,
            selected_disk: 0,
            snapshot,
            history: VecDeque::new(),
            config,
            config_path,
            logs: VecDeque::new(),
            log_sources: Vec::new(),
            socket_error: None,
            message: None,
        };
        app.record_history();
        app
    }

    pub fn apply(&mut self, action: Action) {
        match action {
            Action::Quit => self.running = false,
            Action::NextTab => self.next_tab(),
            Action::PreviousTab => self.previous_tab(),
            Action::MoveUp => self.move_up(),
            Action::MoveDown => self.move_down(),
            Action::First => self.select_first(),
            Action::Last => self.select_last(),
            Action::TogglePause => {
                self.paused = !self.paused;
                self.message = Some(if self.paused {
                    "Live updates paused".to_owned()
                } else {
                    "Live updates resumed".to_owned()
                });
            }
            Action::Refresh => self.force_refresh = true,
            Action::ToggleHelp => {
                self.show_help = !self.show_help;
                self.show_details = false;
            }
            Action::CloseOverlay => {
                self.show_help = false;
                self.show_details = false;
                self.message = None;
            }
            Action::OpenDetails => {
                self.show_details = matches!(
                    self.tab,
                    Tab::Processes | Tab::Services | Tab::Logs | Tab::Ports
                );
            }
            Action::StartSearch => {
                self.input_mode = InputMode::Search;
                self.search.clear();
                self.select_first();
            }
            Action::FinishSearch => self.input_mode = InputMode::Normal,
            Action::ClearSearch => {
                self.input_mode = InputMode::Normal;
                self.search.clear();
                self.select_first();
            }
            Action::SearchChar(character) => {
                self.search.push(character);
                self.select_first();
            }
            Action::SearchBackspace => {
                self.search.pop();
                self.select_first();
            }
            Action::ExportReport | Action::None => {}
        }
    }

    pub fn update_snapshot(&mut self, mut snapshot: SystemSnapshot) {
        snapshot.services = std::mem::take(&mut self.snapshot.services);
        snapshot.sockets = std::mem::take(&mut self.snapshot.sockets);
        self.snapshot = snapshot;
        self.force_refresh = false;
        self.clamp_selections();
        self.record_history();
    }

    pub fn update_services(&mut self, services: Vec<ServiceStatus>) {
        self.snapshot.services = services;
        self.selected_service = self
            .selected_service
            .min(self.snapshot.services.len().saturating_sub(1));
    }

    pub fn update_sockets(&mut self, sockets: Result<Vec<SocketSnapshot>, String>) {
        match sockets {
            Ok(sockets) => {
                self.snapshot.sockets = sockets;
                self.socket_error = None;
                self.selected_socket = self
                    .selected_socket
                    .min(self.snapshot.sockets.len().saturating_sub(1));
            }
            Err(error) => self.socket_error = Some(error),
        }
    }

    pub fn update_logs(&mut self, batch: LogBatch) {
        let was_following = self.logs.is_empty()
            || self.selected_log.saturating_add(1) >= self.visible_logs().len();
        self.log_sources = batch.sources;
        self.logs.extend(batch.entries);
        while self.logs.len() > self.config.log_buffer_lines.max(100) {
            self.logs.pop_front();
        }
        let last = self.visible_logs().len().saturating_sub(1);
        self.selected_log = if was_following {
            last
        } else {
            self.selected_log.min(last)
        };
    }

    pub fn visible_processes(&self) -> Vec<&ProcessSnapshot> {
        let needle = self.search.to_lowercase();
        self.snapshot
            .processes
            .iter()
            .filter(|process| {
                needle.is_empty()
                    || process.name.to_lowercase().contains(&needle)
                    || process.command.to_lowercase().contains(&needle)
                    || process.pid.to_string().contains(&needle)
            })
            .collect()
    }

    pub fn selected_process(&self) -> Option<&ProcessSnapshot> {
        self.visible_processes().get(self.selected_process).copied()
    }

    pub fn selected_service(&self) -> Option<&ServiceStatus> {
        self.snapshot.services.get(self.selected_service)
    }

    pub fn visible_logs(&self) -> Vec<&LogEntry> {
        let needle = self.search.to_lowercase();
        self.logs
            .iter()
            .filter(|entry| {
                needle.is_empty()
                    || entry.source.to_lowercase().contains(&needle)
                    || entry.level.label().to_lowercase().contains(&needle)
                    || entry.line.to_lowercase().contains(&needle)
            })
            .collect()
    }

    pub fn selected_log(&self) -> Option<&LogEntry> {
        self.visible_logs().get(self.selected_log).copied()
    }

    pub fn visible_sockets(&self) -> Vec<&SocketSnapshot> {
        let needle = self.search.to_lowercase();
        self.snapshot
            .sockets
            .iter()
            .filter(|socket| {
                needle.is_empty()
                    || socket.local_address.to_lowercase().contains(&needle)
                    || socket.local_port.to_string().contains(&needle)
                    || socket.state.to_lowercase().contains(&needle)
                    || socket
                        .process_names
                        .iter()
                        .any(|name| name.to_lowercase().contains(&needle))
            })
            .collect()
    }

    pub fn selected_socket(&self) -> Option<&SocketSnapshot> {
        self.visible_sockets().get(self.selected_socket).copied()
    }

    pub fn warnings(&self) -> Vec<String> {
        let mut warnings = Vec::new();
        if self.snapshot.cpu_usage_percent >= self.config.thresholds.cpu_percent {
            warnings.push(format!("CPU {:.0}%", self.snapshot.cpu_usage_percent));
        }
        let memory = format::percent(
            self.snapshot.memory_used_bytes,
            self.snapshot.memory_total_bytes,
        );
        if memory >= self.config.thresholds.memory_percent as f64 {
            warnings.push(format!("Memory {memory:.0}%"));
        }
        for disk in &self.snapshot.disks {
            let used = disk.used_ratio() * 100.0;
            if used >= self.config.thresholds.disk_percent as f64 {
                warnings.push(format!("Disk {} {used:.0}%", disk.mount_point));
            }
        }
        let failed = self
            .snapshot
            .services
            .iter()
            .filter(|service| service.state == HealthState::Unhealthy)
            .count();
        if failed > 0 {
            warnings.push(format!("{failed} service check(s) failing"));
        }
        let unavailable_logs = self
            .log_sources
            .iter()
            .filter(|source| !source.available)
            .count();
        if unavailable_logs > 0 {
            warnings.push(format!("{unavailable_logs} log source(s) unavailable"));
        }
        if self.socket_error.is_some() {
            warnings.push("Port collection unavailable".to_owned());
        }
        warnings
    }

    fn next_tab(&mut self) {
        let index = (self.tab.index() + 1) % Tab::ALL.len();
        self.tab = Tab::ALL[index];
        self.close_transient_ui();
    }

    fn previous_tab(&mut self) {
        let index = self
            .tab
            .index()
            .checked_sub(1)
            .unwrap_or(Tab::ALL.len() - 1);
        self.tab = Tab::ALL[index];
        self.close_transient_ui();
    }

    fn move_up(&mut self) {
        let selection = self.active_selection_mut();
        *selection = selection.saturating_sub(1);
    }

    fn move_down(&mut self) {
        let maximum = self.active_len().saturating_sub(1);
        let selection = self.active_selection_mut();
        *selection = (*selection + 1).min(maximum);
    }

    fn select_first(&mut self) {
        *self.active_selection_mut() = 0;
    }

    fn select_last(&mut self) {
        let maximum = self.active_len().saturating_sub(1);
        *self.active_selection_mut() = maximum;
    }

    fn active_len(&self) -> usize {
        match self.tab {
            Tab::Processes => self.visible_processes().len(),
            Tab::Services => self.snapshot.services.len(),
            Tab::Logs => self.visible_logs().len(),
            Tab::Network => self.snapshot.networks.len(),
            Tab::Ports => self.visible_sockets().len(),
            Tab::Disks => self.snapshot.disks.len(),
            Tab::Overview => self.snapshot.processes.len().min(8),
        }
    }

    fn active_selection_mut(&mut self) -> &mut usize {
        match self.tab {
            Tab::Processes | Tab::Overview => &mut self.selected_process,
            Tab::Services => &mut self.selected_service,
            Tab::Logs => &mut self.selected_log,
            Tab::Network => &mut self.selected_network,
            Tab::Ports => &mut self.selected_socket,
            Tab::Disks => &mut self.selected_disk,
        }
    }

    fn close_transient_ui(&mut self) {
        self.show_help = false;
        self.show_details = false;
        self.input_mode = InputMode::Normal;
        self.search.clear();
    }

    fn clamp_selections(&mut self) {
        self.selected_process = self
            .selected_process
            .min(self.visible_processes().len().saturating_sub(1));
        self.selected_network = self
            .selected_network
            .min(self.snapshot.networks.len().saturating_sub(1));
        self.selected_socket = self
            .selected_socket
            .min(self.visible_sockets().len().saturating_sub(1));
        self.selected_disk = self
            .selected_disk
            .min(self.snapshot.disks.len().saturating_sub(1));
    }

    fn record_history(&mut self) {
        let rx = self
            .snapshot
            .networks
            .iter()
            .map(|network| network.received_bytes)
            .sum();
        let tx = self
            .snapshot
            .networks
            .iter()
            .map(|network| network.transmitted_bytes)
            .sum();
        self.history.push_back(HistoryPoint {
            cpu: self.snapshot.cpu_usage_percent.clamp(0.0, 100.0) as u64,
            memory: format::percent(
                self.snapshot.memory_used_bytes,
                self.snapshot.memory_total_bytes,
            ) as u64,
            network_rx: rx,
            network_tx: tx,
        });
        while self.history.len() > self.config.history_points.max(10) {
            self.history.pop_front();
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::collector::{DemoCollector, SnapshotSource};

    use super::*;

    #[test]
    fn search_filters_processes() {
        let mut collector = DemoCollector::new();
        let mut app = App::new(
            collector.sample(),
            AppConfig::default(),
            None,
            Tab::Processes,
        );
        app.search = "postgres".to_owned();
        assert_eq!(app.visible_processes().len(), 1);
        assert_eq!(app.visible_processes()[0].name, "postgres");
    }

    #[test]
    fn history_is_bounded() {
        let mut collector = DemoCollector::new();
        let config = AppConfig {
            history_points: 10,
            ..AppConfig::default()
        };
        let mut app = App::new(collector.sample(), config, None, Tab::Overview);
        for _ in 0..20 {
            app.update_snapshot(collector.sample());
        }
        assert_eq!(app.history.len(), 10);
    }

    #[test]
    fn log_search_filters_source_level_and_message() {
        let mut collector = DemoCollector::new();
        let mut app = App::new(collector.sample(), AppConfig::default(), None, Tab::Logs);
        app.update_logs(LogBatch {
            entries: vec![LogEntry {
                sequence: 1,
                source: "api".to_owned(),
                level: crate::model::LogLevel::Error,
                line: "database unavailable".to_owned(),
            }],
            sources: Vec::new(),
        });
        app.search = "error".to_owned();
        assert_eq!(app.visible_logs().len(), 1);
        app.search = "worker".to_owned();
        assert!(app.visible_logs().is_empty());
    }
}
