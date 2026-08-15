# Windorion Sentry

**Windorion Sentry** (`wsentry`) is a terminal-native monitoring application for developers. It brings system resources, processes, service health checks, and live logs into one fast, keyboard-first TUI.

> [!IMPORTANT]
> The project is currently in the design and prototyping stage. No runnable release is available yet.

## Vision

Install one native binary, run `wsentry`, and immediately understand what is happening on your machine or inside the current project—without opening a browser, creating an account, or configuring a monitoring stack.

`wsentry` is intended to answer four questions quickly:

1. Is the machine or project healthy?
2. Which process or service is causing the problem?
3. What changed recently?
4. What can I safely inspect or do next?

## Planned experience

```console
$ wsentry
```

The default command will open the local monitoring dashboard. No daemon, account, or remote server will be required.

Planned command modes:

```console
wsentry                  # Open the local monitoring TUI
wsentry .                # Monitor services in the current project
wsentry init             # Create a project configuration
wsentry demo             # Run the TUI with simulated data
wsentry processes        # Open the process view
wsentry logs api         # Follow logs for a configured service
wsentry check            # Run health checks once
wsentry report           # Export a diagnostic report
wsentry doctor           # Check permissions and local capabilities
```

Remote monitoring over SSH is planned for a later release:

```console
wsentry connect user@example-host
```

## v0.1 scope

The first release will focus on a small, reliable local experience:

- CPU, memory, disk, and network monitoring
- Searchable and sortable process list
- Short in-memory history for resource charts
- HTTP and TCP service health checks
- Project configuration through `wsentry.toml`
- Simple threshold-based warnings
- Demo mode with deterministic sample data
- Diagnostic report export
- Native macOS and Windows binaries

The first release will not include accounts, a hosted control plane, Prometheus, Kubernetes, mobile clients, or AI-generated root-cause analysis.

## Main views

| View | Purpose |
| --- | --- |
| Overview | Overall health, resource trends, active warnings, and top processes |
| Processes | Process search, sorting, resource usage, ports, and process details |
| Services | HTTP/TCP checks and configured project services |
| Logs | Searchable, pausable live logs with level highlighting |
| Network | Throughput, connections, and listening ports |
| Disks | Capacity, usage, and read/write activity |

## Keyboard model

The interface will support both standard arrow keys and Vim-style navigation.

| Key | Action |
| --- | --- |
| `Up` / `Down`, `j` / `k` | Move selection |
| `Tab` / `Shift+Tab` | Change view |
| `Enter` | Open details |
| `/` | Search |
| `f` | Filter |
| `Space` | Pause or resume updates |
| `r` | Refresh |
| `e` | Export a diagnostic report |
| `?` | Show help |
| `Esc` | Go back |
| `q` | Quit |

Destructive actions, when introduced, will always show the exact target, explain the impact, require confirmation, and be recorded locally.

## Project configuration

A repository can describe its local services in `wsentry.toml`:

```toml
[[service]]
name = "api"
health = "http://localhost:8080/health"
interval = "10s"
timeout = "3s"

[[service]]
name = "postgres"
tcp = "localhost:5432"
interval = "30s"
```

Running `wsentry .` from that repository will open a project-oriented view of its services, processes, ports, checks, and logs.

## Technical direction

The application will be written in Rust.

| Concern | Planned technology |
| --- | --- |
| Terminal UI | [Ratatui](https://ratatui.rs/) |
| Terminal events | Crossterm |
| Async runtime | Tokio |
| System information | sysinfo |
| CLI parsing | Clap |
| Configuration | Serde + TOML |
| Diagnostics | tracing + tracing-appender |
| Distribution | cargo-dist + GitHub Actions |

The internal data flow will remain unidirectional:

```text
collectors -> bounded channels -> actions -> application state -> Ratatui render
```

Collectors, application state, and rendering will remain separate so the TUI can be tested deterministically and new collectors can be added without coupling them to the interface.

## Planned repository structure

```text
src/
├── main.rs
├── cli.rs
├── app.rs
├── action.rs
├── event.rs
├── config.rs
├── collector/
│   ├── system.rs
│   ├── process.rs
│   ├── network.rs
│   └── health.rs
└── ui/
    ├── overview.rs
    ├── processes.rs
    ├── services.rs
    ├── logs.rs
    └── help.rs
```

## Distribution goals

The installed command will always be:

```console
wsentry
```

Planned installation channels:

```console
# macOS
brew install windorion/tap/wsentry

# Windows
winget install Windorion.WSentry
```

Direct shell and PowerShell installers may be offered alongside checksummed release archives.

## Contributing

The project is not yet ready for external contributions. Contribution guidelines and development setup instructions will be added with the first runnable prototype.

## License

No license has been selected yet. Until a license is added, all rights are reserved.
