# Version 1.0 acceptance

Windorion Sentry 1.0 is a stable local monitoring TUI. Remote agents and graphical clients are intentionally 1.x work so the first stable release has a finite, testable contract.

## Product contract

- One native `wsentry` executable with no required daemon or account
- Local system, process, service, log, network, socket, and disk monitoring
- Validated `wsentry.toml` with backward-compatible defaults for new fields
- Automatic configuration reload that keeps the last valid configuration on errors
- Bounded in-memory history, logs, alerts, and runtime channels
- Text/JSON automation commands that do not require an interactive terminal
- Graceful behavior when metrics, logs, sockets, terminal features, or app-log storage are unavailable

## Release acceptance

- Format, check, Clippy with warnings denied, and all tests pass
- Native release builds pass on Linux, macOS, and Windows
- Apple Silicon/Intel macOS, ARM64/x64 Linux, and x64 Windows archives are configured in the generated release plan
- Shell and PowerShell installers plus SHA-256 checksums are generated
- The packaged executable passes `--version` and `doctor`
- README, configuration example, changelog, architecture, privacy, and release documentation match behavior
- Local `HEAD`, `origin/main`, and the release tag identify the same reviewed commit
- The GitHub Release workflow completes before 1.0 is declared published

## Compatibility policy

Patch releases preserve the configuration schema and CLI behavior. Minor 1.x releases may add optional fields and commands, but existing valid 1.0 configuration continues to load. Breaking configuration or CLI changes require a new major version.
