use std::{
    io::{self, Stdout},
    path::PathBuf,
    time::{Duration, Instant},
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
    health, report, ui,
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
    if !app.config.service.is_empty() {
        let statuses = health::check_all(&app.config.service).await;
        app.update_services(statuses);
    }

    let mut terminal = init_terminal()?;
    let result = run_loop(&mut terminal, &mut app, source.as_mut()).await;
    restore_terminal()?;
    result
}

async fn run_loop(
    terminal: &mut Tui,
    app: &mut App,
    source: &mut dyn SnapshotSource,
) -> Result<()> {
    let refresh_interval = Duration::from_millis(app.config.refresh_interval_ms.max(250));
    let service_interval = app
        .config
        .service
        .iter()
        .filter_map(|service| humantime::parse_duration(&service.interval).ok())
        .min()
        .unwrap_or_else(|| Duration::from_secs(30))
        .max(Duration::from_secs(2));
    let mut last_refresh = Instant::now();
    let mut last_service_check = Instant::now();

    while app.running {
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
                        app.apply(action);
                    }
                }
                Event::Resize(_, _) => terminal.autoresize()?,
                _ => {}
            }
        }

        if app.force_refresh || (!app.paused && last_refresh.elapsed() >= refresh_interval) {
            app.update_snapshot(source.sample());
            last_refresh = Instant::now();
        }

        if !app.paused
            && !app.config.service.is_empty()
            && last_service_check.elapsed() >= service_interval
        {
            let statuses = health::check_all(&app.config.service).await;
            app.update_services(statuses);
            last_service_check = Instant::now();
        }
    }
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
