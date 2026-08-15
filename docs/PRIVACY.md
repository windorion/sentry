# Privacy and local data

Windorion Sentry 1.0 runs locally and has no account, analytics SDK, remote control plane, or telemetry upload. It reads operating-system metrics, process metadata, configured log files, local socket information, and the health-check targets explicitly listed in `wsentry.toml`.

## Data lifetime

- Metrics history, tailed log lines, and alert events are kept only in bounded memory and disappear when `wsentry` exits.
- Application diagnostic logs are stored in the operating system's per-user application-data directory, rotate daily, and retain at most seven files.
- The `e` keyboard shortcut writes a full JSON snapshot to the current directory. It does not include tailed log lines or alert history, but it may include host names, process command lines, service targets, and socket addresses.
- `wsentry report --format json --redact` replaces those sensitive fields before printing a report suitable for review or sharing.

Health checks connect only to configured HTTP, HTTPS, or TCP targets. Log paths and configuration remain local. Users are responsible for protecting exported reports and for ensuring that monitoring configured services is permitted in their environment.
