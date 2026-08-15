# Architecture

`wsentry` is a single binary with deliberately separated acquisition, state, and presentation layers.

## Runtime flow

1. `cli` parses a command and optional project path.
2. `config` discovers, parses, validates, and watches versioned `wsentry.toml` files.
3. `terminal` creates the initial snapshot and starts `runtime`.
4. `runtime` schedules system, socket, log, and health collectors independently.
5. Typed `RuntimeUpdate` messages update `App` state on the TUI loop.
6. `ui` renders immutable `App` state; `action` maps terminal events into state transitions.

## Module boundaries

| Module | Responsibility |
| --- | --- |
| `main` / `cli` | Command dispatch and non-interactive output |
| `config` | Discovery, versioned TOML schema, defaults, validation, hot-reload detection |
| `collector` | System/process/disk/network snapshots and demo data |
| `sockets` | Native socket discovery and normalization |
| `logs` | Stateful incremental file tailing and level detection |
| `health` | Concurrent HTTP/TCP checks with timeouts |
| `runtime` | Background scheduling, bounded channels, pause/refresh/shutdown |
| `app` | TUI state, selection, filtering, alert/recovery events, bounded history |
| `ui` | Ratatui rendering only |
| `report` / `doctor` | Scriptable diagnostics and environment checks |

## Concurrency model

The terminal loop remains responsible only for input, state updates, and drawing. Blocking system sampling, socket enumeration, and file I/O use Tokio blocking tasks. Network health checks are asynchronous and scheduled per service, so one slow target does not delay unrelated checks. Updates travel through a bounded channel, so producers cannot create an unbounded queue if rendering falls behind.

Pause stops scheduled collection without exiting. Refresh resets every collector timer so all data classes update promptly. A valid config change rebuilds timers and log tailers; an invalid change never replaces the active configuration. Stale results for services removed during reload are ignored. Shutdown is sent through the command channel and awaited before terminal restoration.

## Data and privacy

All collection is local. Log lines and alert events live in bounded in-memory deques and are not included in exported reports. JSON reports can contain process command lines and local socket metadata, so users should review them before sharing or use `wsentry report --redact`.

## Extension points

- A remote agent can implement `SnapshotSource` or feed the same typed update model.
- Structured logs can be added behind the existing `LogEntry` boundary.
- A desktop or web client should consume a new versioned API rather than importing TUI state.
- Platform-specific socket collectors should normalize into `SocketSnapshot`.
- Alert sinks should consume warnings/events outside the renderer.
