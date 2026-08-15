# Windorion Sentry

Windorion Sentry (`wsentry`) is a fast, keyboard-first system and service monitor for the terminal. It is a working Rust/Ratatui prototype that runs locally without an account, daemon, or hosted control plane.

## What works

- Live CPU, memory, disk, network, uptime, and load monitoring
- Process table with search, selection, and details
- HTTP and TCP service health checks from `wsentry.toml`
- Threshold warnings and bounded in-memory history charts
- Deterministic demo mode for trying the interface safely
- Text and JSON diagnostic reports
- Script-friendly one-shot checks and machine-readable doctor output
- macOS, Windows, and Linux source builds

The current prototype does not yet collect logs, monitor remote hosts, or ship prebuilt Homebrew/WinGet packages.

## Quick start

Install the stable Rust toolchain, then build and run:

```console
git clone https://github.com/windorion/sentry.git
cd sentry
cargo run --release -- demo
```

Run against the local machine:

```console
cargo run --release
```

Install the command into Cargo's binary directory:

```console
cargo install --path .
wsentry
```

## Commands

```console
wsentry                         # Open the local monitoring TUI
wsentry /path/to/project        # Use that project's wsentry.toml
wsentry demo                    # Run with simulated data
wsentry processes               # Open directly on the process view
wsentry init ./my-project       # Create ./my-project/wsentry.toml
wsentry check                   # Run configured service checks once
wsentry check --json            # Emit service status as JSON
wsentry report                  # Print a point-in-time report
wsentry report --format json    # Emit the full snapshot as JSON
wsentry doctor                  # Inspect platform, terminal, and config
```

Use `wsentry --help` or `wsentry <command> --help` for all options.

## Keyboard controls

| Key | Action |
| --- | --- |
| `↑` / `↓`, `j` / `k` | Move selection |
| `Tab` / `Shift+Tab` | Change view |
| `Enter` | Open process or service details |
| `/` | Search processes |
| `Space` | Pause or resume updates |
| `r` | Refresh now |
| `e` | Export a JSON report in the current directory |
| `?` | Show help |
| `Esc` | Close an overlay or clear search |
| `q`, `Ctrl-C` | Quit |

## Project configuration

Run `wsentry init` or copy [wsentry.example.toml](wsentry.example.toml) to `wsentry.toml`:

```toml
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
```

`wsentry` discovers `wsentry.toml` in the current directory. Passing a project directory selects its config explicitly. Health checks run concurrently; a non-success HTTP response or failed TCP connection is shown as unhealthy.

## Development

The project requires Rust 1.97 or newer.

```console
cargo fmt --all -- --check
cargo check --all-targets
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
cargo run -- demo
```

The main modules are deliberately separated:

```text
CLI/config -> collectors and health checks -> App state -> Ratatui renderer
```

This keeps live system access out of rendering code and makes demo data and state transitions deterministic in tests.

## Distribution roadmap

The installed executable is always named `wsentry`. Planned release artifacts are native binaries for Apple Silicon and Intel macOS, x86-64 and ARM64 Windows, and common Linux targets. Once signed release automation is ready, the intended channels are:

```console
brew install windorion/tap/wsentry
winget install Windorion.WSentry
```

A `curl` installer should only be published together with checksums and signature verification. Until those release assets exist, build from source with Cargo.

## Project status and license

This is an early private prototype. No license has been selected; all rights are reserved until the repository owner chooses one. Keeping it private during the rapid architecture phase is sensible. Open-sourcing it later becomes valuable once releases, security policy, contribution rules, and a stable first-run experience are in place.
