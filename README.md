# Windorion Sentry

Windorion Sentry (`wsentry`) is a fast, keyboard-first monitor for the terminal. It combines live system metrics, process inspection, service health checks, project log tailing, and socket discovery in one local Rust application—without an account, daemon, or hosted control plane.

> Project status: v1.0 stable. Official archives, checksums, and installers are distributed through this repository's GitHub Releases.

## Features

- Eight TUI views: overview, processes, services, logs, events, network, ports, and disks
- Live CPU, memory, load, uptime, disk, network, and process metrics
- Listening-port and active-socket discovery with PID/process attribution
- Incremental multi-file log tailing with level detection, search, bounded buffering, and truncation recovery
- Concurrent HTTP and TCP health checks from `wsentry.toml`
- Non-blocking background collectors, independent refresh intervals, pause, and manual refresh
- Threshold alerts with recovery events, bounded history charts, details overlays, and keyboard navigation
- Validated versioned configuration with automatic reload and last-known-good fallback
- Deterministic demo mode for trying the complete interface safely
- Text/JSON reports, one-shot service checks, and a machine-readable doctor command
- Release automation for Apple Silicon/Intel macOS, x64/ARM64 Linux, and x64 Windows

## Try it

Install Rust 1.97 or newer, then:

```console
git clone https://github.com/windorion/sentry.git
cd sentry
cargo run --release -- demo
```

Monitor the local machine:

```console
cargo run --release
```

Install from the checked-out source:

```console
cargo install --path .
wsentry
```

## Commands

```console
wsentry                         # Open the local monitoring TUI
wsentry /path/to/project        # Load that project's wsentry.toml
wsentry demo                    # Run with simulated metrics, services, logs, and ports
wsentry processes               # Open directly on Processes
wsentry logs                    # Open directly on Logs
wsentry ports                   # Open directly on Ports
wsentry init ./my-project       # Create ./my-project/wsentry.toml
wsentry check                   # Run configured service checks once
wsentry check --json            # Emit service status as JSON
wsentry report                  # Print a point-in-time report
wsentry report --format json    # Emit the full snapshot as JSON
wsentry report --format json --redact # Remove sensitive host and target data
wsentry validate               # Validate wsentry.toml without opening the TUI
wsentry doctor                  # Inspect platform, terminal, config, and collectors
```

Use `wsentry --help` or `wsentry <command> --help` for all options.

## Keyboard controls

| Key | Action |
| --- | --- |
| `↑` / `↓`, `j` / `k` | Move selection |
| `g` / `G`, `Home` / `End` | Select first / last row |
| `Tab` / `Shift+Tab` | Change view |
| `Enter` | Open details for a process, service, log, or socket |
| `/` | Search the active process, log, or port view |
| `Space` | Pause or resume all background updates |
| `r` | Refresh all collectors now |
| `e` | Export a JSON snapshot in the current directory |
| `?` | Show help |
| `Esc` | Close an overlay or clear search |
| `q`, `Ctrl-C` | Quit |

## Project configuration

Run `wsentry init` or copy [`wsentry.example.toml`](wsentry.example.toml) to `wsentry.toml`:

```toml
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

[[log]]
name = "api"
path = "./logs/api.log"

[[log]]
name = "worker"
path = "./logs/worker.log"
```

`wsentry` discovers `wsentry.toml` in the current directory. Passing a project directory selects its configuration explicitly. Relative log paths resolve from the directory containing the config file. While the TUI is running, valid edits are loaded automatically; invalid edits are reported in the footer and the last valid settings remain active. Run `wsentry validate` before deploying configuration changes in automation.

Log files are read incrementally and remain on the local machine. Existing files start from a bounded tail, rotations/truncations are detected, and unavailable sources are reported in the UI instead of stopping other collectors. Health checks run concurrently; a non-success HTTP response or failed TCP connection is unhealthy.

## Installation from releases

After a version tag has produced a GitHub Release, macOS and Linux users can install the matching binary with:

```console
curl --proto '=https' --tlsv1.2 -LsSf \
  https://github.com/windorion/sentry/releases/latest/download/windorion-sentry-installer.sh | sh
```

Windows PowerShell:

```powershell
powershell -ExecutionPolicy Bypass -c "irm https://github.com/windorion/sentry/releases/latest/download/windorion-sentry-installer.ps1 | iex"
```

The generated installers select the correct platform archive and releases include SHA-256 checksums. Private-repository downloads require a GitHub token accepted by the generated installer; the commands work without authentication only if the repository and release assets become public. The 1.0 binaries are checksummed but are not yet Apple-notarized or Authenticode-signed; see [`docs/RELEASING.md`](docs/RELEASING.md).

Homebrew support is the next distribution step. It needs a separate `windorion/homebrew-tap` repository before `brew install windorion/tap/wsentry` can be offered reliably. No Homebrew command is advertised as working yet.

## Architecture

```text
CLI + wsentry.toml
        |
        v
bounded Tokio background runtime
  | system | sockets | log tailers | service checks |
        |
        v
typed updates -> App state/history/filtering -> Ratatui renderer
        |
        +-> text/JSON reports
```

Collectors never render directly. The TUI owns presentation state, while blocking OS and filesystem work runs outside the event loop. Bounded channels and buffers prevent a slow health endpoint or busy log file from making keyboard input unresponsive or memory usage unbounded. See [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) for the module boundaries and extension points, and [`docs/V1_ACCEPTANCE.md`](docs/V1_ACCEPTANCE.md) for the stable-release contract.

## Development

```console
cargo fmt --all -- --check
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
cargo run -- demo
```

Release configuration is generated and validated with cargo-dist 0.31.0. See [`docs/RELEASING.md`](docs/RELEASING.md). A version tag is intentionally not created by ordinary development commits.

## Scope and privacy

The current build monitors one local machine. It does not upload telemetry, persist tailed log content, manage remote agents, or provide a desktop/web/mobile client. JSON reports and TUI exports can contain process command lines, host names, service targets, and socket addresses; use `wsentry report --redact` before sharing a report. See [`docs/PRIVACY.md`](docs/PRIVACY.md).

No open-source license has been selected, so all rights remain reserved. The repository remains private until its owner explicitly chooses a license and public visibility.
