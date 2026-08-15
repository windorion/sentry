# Changelog

All notable changes to Windorion Sentry are documented here.

## [Unreleased]

## [0.2.0] - 2026-08-15

### Added

- Incremental multi-source project log tailing with filtering and level detection
- Listening-port and socket discovery with process attribution
- Dedicated Logs and Ports TUI views plus direct CLI entry commands
- Independent background schedules for system, socket, log, and service collection
- Cross-platform cargo-dist release workflow, shell/PowerShell installers, and checksums
- Configuration validation for refresh intervals, buffers, service targets, and duplicate names
- Architecture and release documentation

### Changed

- Moved blocking collection away from the terminal event/render loop
- Extended text and JSON reports with socket data
- Made unavailable uptime and local app-log storage degrade gracefully
- Expanded automated state, collector, log-tailer, navigation, and rendering tests

[Unreleased]: https://github.com/windorion/sentry/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/windorion/sentry/compare/v0.1.0...v0.2.0
