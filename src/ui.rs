use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, Cell, Clear, Gauge, Paragraph, Row, Sparkline, Table,
        TableState, Tabs, Wrap,
    },
};

use crate::{
    app::{App, InputMode, Tab},
    format,
    model::{HealthState, LogLevel, SocketProtocol},
};

const ACCENT: Color = Color::Cyan;
const MUTED: Color = Color::DarkGray;
const GOOD: Color = Color::Green;
const WARN: Color = Color::Yellow;
const BAD: Color = Color::Red;

pub fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();
    if area.width < 64 || area.height < 18 {
        frame.render_widget(
            Paragraph::new(format!(
                "wsentry needs at least 64x18 cells\ncurrent: {}x{}",
                area.width, area.height
            ))
            .alignment(Alignment::Center)
            .block(panel(" Terminal too small ")),
            area,
        );
        return;
    }

    let sections = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(10),
        Constraint::Length(1),
    ])
    .split(area);

    render_header(frame, sections[0], app);
    match app.tab {
        Tab::Overview => render_overview(frame, sections[1], app),
        Tab::Processes => render_processes(frame, sections[1], app),
        Tab::Services => render_services(frame, sections[1], app),
        Tab::Logs => render_logs(frame, sections[1], app),
        Tab::Network => render_network(frame, sections[1], app),
        Tab::Ports => render_ports(frame, sections[1], app),
        Tab::Disks => render_disks(frame, sections[1], app),
    }
    render_footer(frame, sections[2], app);

    if app.show_help {
        render_help(frame, area);
    } else if app.show_details {
        render_details(frame, area, app);
    }
}

fn render_header(frame: &mut Frame, area: Rect, app: &App) {
    let titles = Tab::ALL
        .iter()
        .map(|tab| Line::from(format!(" {} ", tab.title())))
        .collect::<Vec<_>>();
    let status = if app.paused { "PAUSED" } else { "LIVE" };
    let context = app
        .config_path
        .as_deref()
        .and_then(|path| path.parent())
        .and_then(|path| path.file_name())
        .and_then(|name| name.to_str())
        .unwrap_or("local");
    let title = format!(
        " WSENTRY · {} · {} · {status} ",
        app.snapshot.host_name, context
    );
    let tabs = Tabs::new(titles)
        .select(app.tab.index())
        .style(Style::default().fg(MUTED))
        .highlight_style(Style::default().fg(ACCENT).add_modifier(Modifier::BOLD))
        .divider("│")
        .block(panel(&title));
    frame.render_widget(tabs, area);
}

fn render_footer(frame: &mut Frame, area: Rect, app: &App) {
    let line = if app.input_mode == InputMode::Search {
        Line::from(vec![
            Span::styled(" / ", Style::default().fg(ACCENT)),
            Span::raw(&app.search),
            Span::styled("█", Style::default().fg(ACCENT)),
            Span::styled("  Enter apply · Esc clear", Style::default().fg(MUTED)),
        ])
    } else if let Some(message) = &app.message {
        Line::from(vec![
            Span::styled(" • ", Style::default().fg(ACCENT)),
            Span::raw(message),
            Span::styled("  Esc dismiss", Style::default().fg(MUTED)),
        ])
    } else {
        Line::from(vec![
            Span::styled(" ↑↓/jk", Style::default().fg(ACCENT)),
            Span::raw(" move  "),
            Span::styled("Tab", Style::default().fg(ACCENT)),
            Span::raw(" view  "),
            Span::styled("Enter", Style::default().fg(ACCENT)),
            Span::raw(" details  "),
            Span::styled("/", Style::default().fg(ACCENT)),
            Span::raw(" search  "),
            Span::styled("Space", Style::default().fg(ACCENT)),
            Span::raw(" pause  "),
            Span::styled("?", Style::default().fg(ACCENT)),
            Span::raw(" help  "),
            Span::styled("q", Style::default().fg(ACCENT)),
            Span::raw(" quit"),
        ])
    };
    frame.render_widget(Paragraph::new(line), area);
}

fn render_overview(frame: &mut Frame, area: Rect, app: &App) {
    let columns =
        Layout::horizontal([Constraint::Percentage(34), Constraint::Percentage(66)]).split(area);
    let left = Layout::vertical([
        Constraint::Length(7),
        Constraint::Length(7),
        Constraint::Min(5),
    ])
    .split(columns[0]);
    render_identity(frame, left[0], app);
    render_resources(frame, left[1], app);
    render_warnings(frame, left[2], app);

    let right = Layout::vertical([Constraint::Length(8), Constraint::Min(6)]).split(columns[1]);
    render_history(frame, right[0], app);
    render_top_processes(frame, right[1], app);
}

fn render_identity(frame: &mut Frame, area: Rect, app: &App) {
    let content = vec![
        Line::from(vec![Span::styled(
            &app.snapshot.host_name,
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        )]),
        Line::from(truncate(
            &app.snapshot.os_name,
            area.width.saturating_sub(4) as usize,
        )),
        Line::from(format!("Kernel  {}", app.snapshot.kernel_version)),
        Line::from(format!(
            "Uptime  {}",
            app.snapshot
                .uptime_seconds
                .map(format::duration)
                .unwrap_or_else(|| "unavailable".to_owned())
        )),
        Line::from(format!(
            "Load    {:.2}  {:.2}  {:.2}",
            app.snapshot.load_average.one,
            app.snapshot.load_average.five,
            app.snapshot.load_average.fifteen
        )),
    ];
    frame.render_widget(Paragraph::new(content).block(panel(" System ")), area);
}

fn render_resources(frame: &mut Frame, area: Rect, app: &App) {
    let rows = Layout::vertical([
        Constraint::Length(2),
        Constraint::Length(2),
        Constraint::Length(2),
    ])
    .split(inner(area));
    frame.render_widget(panel(" Resources "), area);

    let memory = format::percent(
        app.snapshot.memory_used_bytes,
        app.snapshot.memory_total_bytes,
    );
    let disk = app
        .snapshot
        .disks
        .iter()
        .map(|disk| disk.used_ratio() * 100.0)
        .fold(0.0, f64::max);
    render_gauge(frame, rows[0], "CPU", app.snapshot.cpu_usage_percent as f64);
    render_gauge(frame, rows[1], "MEM", memory);
    render_gauge(frame, rows[2], "DISK", disk);
}

fn render_gauge(frame: &mut Frame, area: Rect, label: &str, percent: f64) {
    let color = health_color(percent);
    let gauge = Gauge::default()
        .gauge_style(Style::default().fg(color).bg(Color::Black))
        .label(format!("{label:<4} {percent:>5.1}%"))
        .ratio((percent / 100.0).clamp(0.0, 1.0));
    frame.render_widget(gauge, area);
}

fn render_warnings(frame: &mut Frame, area: Rect, app: &App) {
    let warnings = app.warnings();
    let lines = if warnings.is_empty() {
        vec![Line::from(vec![
            Span::styled("● ", Style::default().fg(GOOD)),
            Span::raw("No active warnings"),
        ])]
    } else {
        warnings
            .into_iter()
            .map(|warning| {
                Line::from(vec![
                    Span::styled("● ", Style::default().fg(WARN)),
                    Span::raw(warning),
                ])
            })
            .collect()
    };
    frame.render_widget(Paragraph::new(lines).block(panel(" Warnings ")), area);
}

fn render_history(frame: &mut Frame, area: Rect, app: &App) {
    let columns = Layout::horizontal([
        Constraint::Percentage(33),
        Constraint::Percentage(34),
        Constraint::Percentage(33),
    ])
    .split(area);
    let cpu = app
        .history
        .iter()
        .map(|point| point.cpu)
        .collect::<Vec<_>>();
    let memory = app
        .history
        .iter()
        .map(|point| point.memory)
        .collect::<Vec<_>>();
    let network = app
        .history
        .iter()
        .map(|point| point.network_rx.saturating_add(point.network_tx))
        .collect::<Vec<_>>();
    frame.render_widget(
        Sparkline::default()
            .block(panel(" CPU history "))
            .data(&cpu)
            .max(100)
            .style(Style::default().fg(ACCENT)),
        columns[0],
    );
    frame.render_widget(
        Sparkline::default()
            .block(panel(" Memory history "))
            .data(&memory)
            .max(100)
            .style(Style::default().fg(Color::Magenta)),
        columns[1],
    );
    frame.render_widget(
        Sparkline::default()
            .block(panel(" Network I/O "))
            .data(&network)
            .style(Style::default().fg(Color::Blue)),
        columns[2],
    );
}

fn render_top_processes(frame: &mut Frame, area: Rect, app: &App) {
    let rows = app
        .snapshot
        .processes
        .iter()
        .take(area.height.saturating_sub(3) as usize)
        .map(|process| {
            Row::new(vec![
                process.pid.to_string(),
                process.name.clone(),
                format!("{:.1}%", process.cpu_usage_percent),
                format::bytes(process.memory_bytes),
            ])
        });
    let table = Table::new(
        rows,
        [
            Constraint::Length(8),
            Constraint::Percentage(50),
            Constraint::Length(10),
            Constraint::Length(12),
        ],
    )
    .header(table_header(["PID", "PROCESS", "CPU", "MEMORY"]))
    .block(panel(" Top processes "));
    frame.render_widget(table, area);
}

fn render_processes(frame: &mut Frame, area: Rect, app: &App) {
    let visible = app.visible_processes();
    let rows = visible.into_iter().map(process_row);
    let title = if app.search.is_empty() {
        format!(" Processes · {} total ", app.snapshot.processes.len())
    } else {
        format!(" Processes · filter: {} ", app.search)
    };
    let table = Table::new(
        rows,
        [
            Constraint::Length(8),
            Constraint::Percentage(35),
            Constraint::Length(10),
            Constraint::Length(12),
            Constraint::Length(12),
            Constraint::Percentage(35),
        ],
    )
    .header(table_header([
        "PID", "PROCESS", "CPU", "MEMORY", "STATUS", "COMMAND",
    ]))
    .row_highlight_style(Style::default().bg(Color::DarkGray).fg(Color::White))
    .highlight_symbol("› ")
    .block(panel(&title));
    let mut state = TableState::default().with_selected(Some(app.selected_process));
    frame.render_stateful_widget(table, area, &mut state);
}

fn render_services(frame: &mut Frame, area: Rect, app: &App) {
    if app.snapshot.services.is_empty() {
        let message = if app.config.service.is_empty() {
            "No services configured. Run `wsentry init` or add [[service]] entries to wsentry.toml."
        } else {
            "Service checks have not completed yet."
        };
        frame.render_widget(
            Paragraph::new(message)
                .wrap(Wrap { trim: true })
                .alignment(Alignment::Center)
                .block(panel(" Services ")),
            area,
        );
        return;
    }

    let rows = app.snapshot.services.iter().map(|service| {
        let (symbol, color) = match service.state {
            HealthState::Healthy => ("●", GOOD),
            HealthState::Unhealthy => ("●", BAD),
            HealthState::Unknown => ("○", MUTED),
        };
        Row::new(vec![
            Cell::from(Span::styled(symbol, Style::default().fg(color))),
            Cell::from(service.name.clone()),
            Cell::from(format!("{:?}", service.state)),
            Cell::from(
                service
                    .latency_ms
                    .map(|latency| format!("{latency} ms"))
                    .unwrap_or_else(|| "—".to_owned()),
            ),
            Cell::from(
                service
                    .status_code
                    .map(|code| code.to_string())
                    .unwrap_or_else(|| "—".to_owned()),
            ),
            Cell::from(service.target.clone()),
            Cell::from(service.message.clone().unwrap_or_default()),
        ])
    });
    let table = Table::new(
        rows,
        [
            Constraint::Length(3),
            Constraint::Length(18),
            Constraint::Length(12),
            Constraint::Length(12),
            Constraint::Length(8),
            Constraint::Percentage(35),
            Constraint::Percentage(35),
        ],
    )
    .header(table_header([
        "", "SERVICE", "STATE", "LATENCY", "CODE", "TARGET", "DETAIL",
    ]))
    .row_highlight_style(Style::default().bg(Color::DarkGray).fg(Color::White))
    .highlight_symbol("› ")
    .block(panel(" Service health "));
    let mut state = TableState::default().with_selected(Some(app.selected_service));
    frame.render_stateful_widget(table, area, &mut state);
}

fn render_network(frame: &mut Frame, area: Rect, app: &App) {
    let rows = app.snapshot.networks.iter().map(|network| {
        Row::new(vec![
            network.interface.clone(),
            format::bytes(network.received_bytes),
            format::bytes(network.transmitted_bytes),
            format::bytes(network.total_received_bytes),
            format::bytes(network.total_transmitted_bytes),
            network.errors_received.to_string(),
            network.errors_transmitted.to_string(),
        ])
    });
    let table = Table::new(
        rows,
        [
            Constraint::Percentage(20),
            Constraint::Length(14),
            Constraint::Length(14),
            Constraint::Length(16),
            Constraint::Length(16),
            Constraint::Length(10),
            Constraint::Length(10),
        ],
    )
    .header(table_header([
        "INTERFACE",
        "RX NOW",
        "TX NOW",
        "RX TOTAL",
        "TX TOTAL",
        "RX ERR",
        "TX ERR",
    ]))
    .row_highlight_style(Style::default().bg(Color::DarkGray).fg(Color::White))
    .highlight_symbol("› ")
    .block(panel(" Network interfaces "));
    let mut state = TableState::default().with_selected(Some(app.selected_network));
    frame.render_stateful_widget(table, area, &mut state);
}

fn render_logs(frame: &mut Frame, area: Rect, app: &App) {
    if app.config.log.is_empty() && app.logs.is_empty() {
        frame.render_widget(
            Paragraph::new(
                "No logs configured. Add [[log]] entries to wsentry.toml, then set name and path.",
            )
            .wrap(Wrap { trim: true })
            .alignment(Alignment::Center)
            .block(panel(" Logs ")),
            area,
        );
        return;
    }

    let unavailable = app
        .log_sources
        .iter()
        .filter(|source| !source.available)
        .count();
    let visible = app.visible_logs();
    let rows = visible.iter().map(|entry| {
        Row::new(vec![
            entry.sequence.to_string(),
            entry.source.clone(),
            entry.level.label().to_owned(),
            entry.line.clone(),
        ])
        .style(Style::default().fg(log_level_color(entry.level)))
    });
    let filter = if app.search.is_empty() {
        String::new()
    } else {
        format!(" · filter: {}", app.search)
    };
    let issues = if unavailable == 0 {
        String::new()
    } else {
        format!(" · {unavailable} source issue(s)")
    };
    let title = format!(" Logs · {} buffered{filter}{issues} ", app.logs.len());
    let table = Table::new(
        rows,
        [
            Constraint::Length(8),
            Constraint::Length(16),
            Constraint::Length(8),
            Constraint::Percentage(100),
        ],
    )
    .header(table_header(["SEQ", "SOURCE", "LEVEL", "MESSAGE"]))
    .row_highlight_style(Style::default().bg(Color::DarkGray).fg(Color::White))
    .highlight_symbol("› ")
    .block(panel(&title));
    let mut state = TableState::default().with_selected(Some(app.selected_log));
    frame.render_stateful_widget(table, area, &mut state);
}

fn render_ports(frame: &mut Frame, area: Rect, app: &App) {
    if app.snapshot.sockets.is_empty() {
        let message = app.socket_error.as_deref().unwrap_or(
            "No listening ports or active sockets were discovered for visible processes.",
        );
        frame.render_widget(
            Paragraph::new(message)
                .wrap(Wrap { trim: true })
                .alignment(Alignment::Center)
                .block(panel(" Ports and sockets ")),
            area,
        );
        return;
    }

    let visible = app.visible_sockets();
    let rows = visible.iter().map(|socket| {
        let protocol = match socket.protocol {
            SocketProtocol::Tcp => "TCP",
            SocketProtocol::Udp => "UDP",
        };
        Row::new(vec![
            protocol.to_owned(),
            socket.local_address.clone(),
            socket.local_port.to_string(),
            socket.state.clone(),
            socket
                .associated_pids
                .iter()
                .map(u32::to_string)
                .collect::<Vec<_>>()
                .join(","),
            socket.process_names.join(", "),
        ])
    });
    let filter = if app.search.is_empty() {
        String::new()
    } else {
        format!(" · filter: {}", app.search)
    };
    let title = format!(" Ports and sockets{filter} ");
    let table = Table::new(
        rows,
        [
            Constraint::Length(7),
            Constraint::Percentage(25),
            Constraint::Length(9),
            Constraint::Length(16),
            Constraint::Length(12),
            Constraint::Percentage(45),
        ],
    )
    .header(table_header([
        "PROTO", "ADDRESS", "PORT", "STATE", "PID", "PROCESS",
    ]))
    .row_highlight_style(Style::default().bg(Color::DarkGray).fg(Color::White))
    .highlight_symbol("› ")
    .block(panel(&title));
    let mut state = TableState::default().with_selected(Some(app.selected_socket));
    frame.render_stateful_widget(table, area, &mut state);
}

fn render_disks(frame: &mut Frame, area: Rect, app: &App) {
    let rows = app.snapshot.disks.iter().map(|disk| {
        let used_percent = disk.used_ratio() * 100.0;
        Row::new(vec![
            disk.name.clone(),
            disk.mount_point.clone(),
            disk.file_system.clone(),
            disk.kind.clone(),
            format::bytes(disk.used_bytes()),
            format::bytes(disk.total_bytes),
            format!("{used_percent:.1}%"),
        ])
        .style(Style::default().fg(health_color(used_percent)))
    });
    let table = Table::new(
        rows,
        [
            Constraint::Percentage(20),
            Constraint::Percentage(25),
            Constraint::Length(12),
            Constraint::Length(10),
            Constraint::Length(14),
            Constraint::Length(14),
            Constraint::Length(10),
        ],
    )
    .header(table_header([
        "DISK",
        "MOUNT",
        "FILESYSTEM",
        "KIND",
        "USED",
        "TOTAL",
        "USAGE",
    ]))
    .row_highlight_style(Style::default().bg(Color::DarkGray).fg(Color::White))
    .highlight_symbol("› ")
    .block(panel(" Disks "));
    let mut state = TableState::default().with_selected(Some(app.selected_disk));
    frame.render_stateful_widget(table, area, &mut state);
}

fn render_help(frame: &mut Frame, area: Rect) {
    let popup = centered_rect(62, 68, area);
    let lines = vec![
        Line::from(Span::styled(
            "Navigation",
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        )),
        Line::from("  ↑/↓ or j/k     Move selection"),
        Line::from("  Tab/Shift+Tab  Change view"),
        Line::from("  Enter          Open selected details"),
        Line::from("  Home/End       First/last item"),
        Line::from(""),
        Line::from(Span::styled(
            "Actions",
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        )),
        Line::from("  /              Search the current view"),
        Line::from("  Space          Pause/resume live updates"),
        Line::from("  r              Refresh now"),
        Line::from("  e              Export JSON diagnostic report"),
        Line::from("  Esc            Close overlay"),
        Line::from("  q / Ctrl-C     Quit"),
    ];
    frame.render_widget(Clear, popup);
    frame.render_widget(
        Paragraph::new(lines)
            .block(panel(" Help · Esc to close ").border_style(Style::default().fg(ACCENT)))
            .wrap(Wrap { trim: false }),
        popup,
    );
}

fn render_details(frame: &mut Frame, area: Rect, app: &App) {
    let popup = centered_rect(72, 58, area);
    let (title, lines) = match app.tab {
        Tab::Processes => match app.selected_process() {
            Some(process) => (
                format!(" Process {} ", process.pid),
                vec![
                    Line::from(vec![Span::styled(
                        &process.name,
                        Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
                    )]),
                    Line::from(format!("PID       {}", process.pid)),
                    Line::from(format!("Status    {}", process.status)),
                    Line::from(format!("CPU       {:.1}%", process.cpu_usage_percent)),
                    Line::from(format!("Memory    {}", format::bytes(process.memory_bytes))),
                    Line::from(format!(
                        "Virtual   {}",
                        format::bytes(process.virtual_memory_bytes)
                    )),
                    Line::from(format!(
                        "Runtime   {}",
                        format::duration(process.run_time_seconds)
                    )),
                    Line::from(""),
                    Line::from(Span::styled("Command", Style::default().fg(MUTED))),
                    Line::from(if process.command.is_empty() {
                        "—".to_owned()
                    } else {
                        process.command.clone()
                    }),
                ],
            ),
            None => (
                " Process ".to_owned(),
                vec![Line::from("No process selected")],
            ),
        },
        Tab::Services => match app.selected_service() {
            Some(service) => (
                format!(" Service {} ", service.name),
                vec![
                    Line::from(format!("State     {:?}", service.state)),
                    Line::from(format!("Target    {}", service.target)),
                    Line::from(format!(
                        "Latency   {}",
                        service
                            .latency_ms
                            .map(|value| format!("{value} ms"))
                            .unwrap_or_else(|| "—".to_owned())
                    )),
                    Line::from(format!(
                        "HTTP      {}",
                        service
                            .status_code
                            .map(|value| value.to_string())
                            .unwrap_or_else(|| "—".to_owned())
                    )),
                    Line::from(format!("Checked   {}", service.checked_at.to_rfc3339())),
                    Line::from(""),
                    Line::from(
                        service
                            .message
                            .clone()
                            .unwrap_or_else(|| "No errors".to_owned()),
                    ),
                ],
            ),
            None => (
                " Service ".to_owned(),
                vec![Line::from("No service selected")],
            ),
        },
        Tab::Logs => match app.selected_log() {
            Some(entry) => (
                format!(" Log {} ", entry.sequence),
                vec![
                    Line::from(format!("Source    {}", entry.source)),
                    Line::from(format!("Level     {}", entry.level.label())),
                    Line::from(""),
                    Line::from(entry.line.clone()),
                ],
            ),
            None => (" Log ".to_owned(), vec![Line::from("No log selected")]),
        },
        Tab::Ports => match app.selected_socket() {
            Some(socket) => (
                format!(" Port {} ", socket.local_port),
                vec![
                    Line::from(format!("Protocol  {:?}", socket.protocol)),
                    Line::from(format!(
                        "Local     {}:{}",
                        socket.local_address, socket.local_port
                    )),
                    Line::from(format!("State     {}", socket.state)),
                    Line::from(format!(
                        "PID       {}",
                        socket
                            .associated_pids
                            .iter()
                            .map(u32::to_string)
                            .collect::<Vec<_>>()
                            .join(", ")
                    )),
                    Line::from(format!("Process   {}", socket.process_names.join(", "))),
                ],
            ),
            None => (" Port ".to_owned(), vec![Line::from("No socket selected")]),
        },
        _ => return,
    };
    frame.render_widget(Clear, popup);
    frame.render_widget(
        Paragraph::new(lines).wrap(Wrap { trim: false }).block(
            panel(&format!("{title}· Esc to close ")).border_style(Style::default().fg(ACCENT)),
        ),
        popup,
    );
}

fn process_row(process: &crate::model::ProcessSnapshot) -> Row<'static> {
    Row::new(vec![
        process.pid.to_string(),
        process.name.clone(),
        format!("{:.1}%", process.cpu_usage_percent),
        format::bytes(process.memory_bytes),
        process.status.clone(),
        process.command.clone(),
    ])
}

fn table_header<const N: usize>(labels: [&'static str; N]) -> Row<'static> {
    Row::new(labels)
        .style(Style::default().fg(ACCENT).add_modifier(Modifier::BOLD))
        .bottom_margin(1)
}

fn panel(title: &str) -> Block<'_> {
    Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::DarkGray))
}

fn inner(area: Rect) -> Rect {
    Rect {
        x: area.x.saturating_add(1),
        y: area.y.saturating_add(1),
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(2),
    }
}

fn centered_rect(width_percent: u16, height_percent: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - height_percent) / 2),
            Constraint::Percentage(height_percent),
            Constraint::Percentage((100 - height_percent) / 2),
        ])
        .split(area);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - width_percent) / 2),
            Constraint::Percentage(width_percent),
            Constraint::Percentage((100 - width_percent) / 2),
        ])
        .split(vertical[1])[1]
}

fn health_color(percent: f64) -> Color {
    if percent >= 90.0 {
        BAD
    } else if percent >= 75.0 {
        WARN
    } else {
        GOOD
    }
}

fn log_level_color(level: LogLevel) -> Color {
    match level {
        LogLevel::Error => BAD,
        LogLevel::Warn => WARN,
        LogLevel::Info => Color::White,
        LogLevel::Debug => ACCENT,
        LogLevel::Trace | LogLevel::Unknown => MUTED,
    }
}

fn truncate(value: &str, maximum: usize) -> String {
    if value.chars().count() <= maximum {
        value.to_owned()
    } else if maximum > 1 {
        format!("{}…", value.chars().take(maximum - 1).collect::<String>())
    } else {
        "…".to_owned()
    }
}

#[cfg(test)]
mod tests {
    use ratatui::{Terminal, backend::TestBackend};

    use crate::{
        app::{App, Tab},
        collector::{DemoCollector, SnapshotSource},
        config::AppConfig,
        logs::LogBatch,
        model::{LogEntry, LogLevel},
    };

    use super::*;

    #[test]
    fn renders_demo_overview_in_a_test_terminal() {
        let mut collector = DemoCollector::new();
        let app = App::new(
            collector.sample(),
            AppConfig::default(),
            None,
            Tab::Overview,
        );
        let backend = TestBackend::new(120, 36);
        let mut terminal = Terminal::new(backend).expect("test terminal");

        terminal.draw(|frame| render(frame, &app)).expect("draw");

        let rendered = rendered_text(&terminal);
        assert!(rendered.contains("WSENTRY"));
        assert!(rendered.contains("Top processes"));
        assert!(rendered.contains("demo-workstation"));
    }

    #[test]
    fn renders_demo_ports() {
        let mut collector = DemoCollector::new();
        let app = App::new(collector.sample(), AppConfig::default(), None, Tab::Ports);
        let backend = TestBackend::new(120, 36);
        let mut terminal = Terminal::new(backend).expect("test terminal");

        terminal.draw(|frame| render(frame, &app)).expect("draw");

        let rendered = rendered_text(&terminal);
        assert!(rendered.contains("Ports and sockets"));
        assert!(rendered.contains("8080"));
        assert!(rendered.contains("api-server"));
    }

    #[test]
    fn renders_tailed_logs() {
        let mut collector = DemoCollector::new();
        let mut app = App::new(collector.sample(), AppConfig::default(), None, Tab::Logs);
        app.update_logs(LogBatch {
            entries: vec![LogEntry {
                sequence: 1,
                source: "api".to_owned(),
                level: LogLevel::Warn,
                line: "queue depth is high".to_owned(),
            }],
            sources: Vec::new(),
        });
        let backend = TestBackend::new(120, 36);
        let mut terminal = Terminal::new(backend).expect("test terminal");

        terminal.draw(|frame| render(frame, &app)).expect("draw");

        let rendered = rendered_text(&terminal);
        assert!(rendered.contains("Logs"));
        assert!(rendered.contains("WARN"));
        assert!(rendered.contains("queue depth is high"));
    }

    fn rendered_text(terminal: &Terminal<TestBackend>) -> String {
        terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect()
    }
}
