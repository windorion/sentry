use std::{
    io::{self, Stdout},
    path::PathBuf,
    time::Duration,
};

use color_eyre::eyre::Result;
use crossterm::{
    cursor::Show,
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};

use crate::{
    action::{self, Action},
    app::{App, Tab},
    collector::{DemoCollector, LocalCollector, SnapshotSource},
    config::AppConfig,
    report,
    runtime::{self, RuntimeCommand, RuntimeUpdate},
    ui,
};

type Tui = Terminal<CrosstermBackend<Stdout>>;

#[derive(Clone, Copy, Debug)]
pub enum RunMode {
    Local,
    Demo,
}

pub async fn run(
    mode: RunMode,
    config: AppConfig,
    config_path: Option<PathBuf>,
    initial_tab: Tab,
) -> Result<()> {
    let mut source: Box<dyn SnapshotSource> = match mode {
        RunMode::Local => Box::new(LocalCollector::new()),
        RunMode::Demo => Box::new(DemoCollector::new()),
    };
    let snapshot = source.sample();
    let mut app = App::new(snapshot, config, config_path, initial_tab);
    let base_directory = app
        .config_path
        .as_deref()
        .and_then(|path| path.parent())
        .map(PathBuf::from)
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| PathBuf::from("."));
    let worker = runtime::start(
        source,
        app.config.clone(),
        base_directory,
        matches!(mode, RunMode::Demo),
    );

    let mut terminal = init_terminal()?;
    let result = run_loop(&mut terminal, &mut app, worker).await;
    restore_terminal()?;
    result
}

async fn run_loop(
    terminal: &mut Tui,
    app: &mut App,
    mut worker: runtime::RuntimeHandle,
) -> Result<()> {
    while app.running {
        for update in worker.drain() {
            match update {
                RuntimeUpdate::Snapshot(snapshot) => app.update_snapshot(*snapshot),
                RuntimeUpdate::Services(services) => app.update_services(services),
                RuntimeUpdate::Sockets(sockets) => app.update_sockets(sockets),
                RuntimeUpdate::Logs(logs) => app.update_logs(logs),
                RuntimeUpdate::Error(error) => app.message = Some(error),
            }
        }
        terminal.draw(|frame| ui::render(frame, app))?;

        if event::poll(Duration::from_millis(50))? {
            match event::read()? {
                Event::Key(key)
                    if matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) =>
                {
                    let action = action::from_key(key, app.input_mode);
                    if action == Action::ExportReport {
                        match report::write_json(&app.snapshot) {
                            Ok(path) => {
                                app.message = Some(format!("Report saved to {}", path.display()))
                            }
                            Err(error) => app.message = Some(format!("Export failed: {error}")),
                        }
                    } else {
                        app.apply(action.clone());
                        match action {
                            Action::TogglePause => {
                                worker.command(RuntimeCommand::SetPaused(app.paused));
                            }
                            Action::Refresh => {
                                app.force_refresh = false;
                                worker.command(RuntimeCommand::Refresh);
                            }
                            _ => {}
                        }
                    }
                }
                Event::Resize(_, _) => terminal.autoresize()?,
                _ => {}
            }
        }
    }
    worker.stop().await;
    Ok(())
}

fn init_terminal() -> Result<Tui> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    Terminal::new(CrosstermBackend::new(stdout)).map_err(Into::into)
}

pub fn restore_terminal() -> Result<()> {
    disable_raw_mode()?;
    execute!(
        io::stdout(),
        LeaveAlternateScreen,
        DisableMouseCapture,
        Show
    )?;
    Ok(())
}

pub fn install_panic_hook() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = restore_terminal();
        previous(info);
    }));
}
