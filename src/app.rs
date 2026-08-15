use std::{collections::VecDeque, path::PathBuf};

use crate::{
    action::Action,
    config::AppConfig,
    format,
    model::{HealthState, ProcessSnapshot, ServiceStatus, SystemSnapshot},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tab {
    Overview,
    Processes,
    Services,
    Network,
    Disks,
}

impl Tab {
    pub const ALL: [Self; 5] = [
        Self::Overview,
        Self::Processes,
        Self::Services,
        Self::Network,
        Self::Disks,
    ];

    pub const fn title(self) -> &'static str {
        match self {
            Self::Overview => "Overview",
            Self::Processes => "Processes",
            Self::Services => "Services",
            Self::Network => "Network",
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
    pub selected_network: usize,
    pub selected_disk: usize,
    pub snapshot: SystemSnapshot,
    pub history: VecDeque<HistoryPoint>,
    pub config: AppConfig,
    pub config_path: Option<PathBuf>,
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
            selected_network: 0,
            selected_disk: 0,
            snapshot,
            history: VecDeque::new(),
            config,
            config_path,
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
                self.show_details = matches!(self.tab, Tab::Processes | Tab::Services);
            }
            Action::StartSearch => {
                self.input_mode = InputMode::Search;
                self.search.clear();
                self.selected_process = 0;
            }
            Action::FinishSearch => self.input_mode = InputMode::Normal,
            Action::ClearSearch => {
                self.input_mode = InputMode::Normal;
                self.search.clear();
                self.selected_process = 0;
            }
            Action::SearchChar(character) => {
                self.search.push(character);
                self.selected_process = 0;
            }
            Action::SearchBackspace => {
                self.search.pop();
                self.selected_process = 0;
            }
            Action::ExportReport | Action::None => {}
        }
    }

    pub fn update_snapshot(&mut self, mut snapshot: SystemSnapshot) {
        snapshot.services = std::mem::take(&mut self.snapshot.services);
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
            Tab::Network => self.snapshot.networks.len(),
            Tab::Disks => self.snapshot.disks.len(),
            Tab::Overview => self.snapshot.processes.len().min(8),
        }
    }

    fn active_selection_mut(&mut self) -> &mut usize {
        match self.tab {
            Tab::Processes | Tab::Overview => &mut self.selected_process,
            Tab::Services => &mut self.selected_service,
            Tab::Network => &mut self.selected_network,
            Tab::Disks => &mut self.selected_disk,
        }
    }

    fn close_transient_ui(&mut self) {
        self.show_help = false;
        self.show_details = false;
        self.input_mode = InputMode::Normal;
    }

    fn clamp_selections(&mut self) {
        self.selected_process = self
            .selected_process
            .min(self.visible_processes().len().saturating_sub(1));
        self.selected_network = self
            .selected_network
            .min(self.snapshot.networks.len().saturating_sub(1));
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
}
