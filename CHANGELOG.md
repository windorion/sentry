# Changelog

All notable changes to Windorion Sentry are documented here.

## [Unreleased]

### Added

- Dependabot and scheduled RustSec auditing, structured issue forms, and a Contributor Covenant code of conduct
- Direct private security-reporting and project-independence guidance

### Changed

- Limited release automation to version tags and pinned GitHub Actions to immutable revisions
- Added package discovery metadata for terminal monitoring and development tooling

## [1.0.1] - 2026-08-15

### Added

- Apache License 2.0, contribution guidelines, and public-repository documentation

## [1.0.0] - 2026-08-15

### Added

- Security policy and privacy documentation
- Stable local-monitoring product scope and configuration compatibility policy
- Automatic validated configuration reloads without restarting the TUI
- Bounded alert and recovery event history
- Configuration validation command and privacy-preserving report redaction
- Cross-platform release CLI smoke tests for Linux, macOS, and Windows

### Changed

- Stabilized `schema_version = 1`, strict unknown-field handling, and per-service health-check schedules
- Improved failed health-check output with actionable error details

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

[Unreleased]: https://github.com/windorion/sentry/compare/v1.0.1...HEAD
[1.0.1]: https://github.com/windorion/sentry/compare/v1.0.0...v1.0.1
[1.0.0]: https://github.com/windorion/sentry/compare/v0.2.0...v1.0.0
[0.2.0]: https://github.com/windorion/sentry/compare/v0.1.0...v0.2.0
