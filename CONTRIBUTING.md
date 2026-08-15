# Contributing to Windorion Sentry

Thank you for helping improve Windorion Sentry. Bug reports, documentation fixes, platform compatibility improvements, and focused feature proposals are welcome.

Participation in this project is governed by the [`CODE_OF_CONDUCT.md`](CODE_OF_CONDUCT.md).

## Before opening an issue

- Search existing issues first.
- Include the operating system, `wsentry --version`, and concise reproduction steps.
- Run `wsentry doctor --json` when environment details are relevant.
- Do not post secrets, private log lines, or unredacted diagnostic reports. Use `wsentry report --format json --redact` before sharing report output.
- Report security vulnerabilities according to [`SECURITY.md`](SECURITY.md), not in a public issue.

## Development setup

Install Rust 1.96 or newer, clone the repository, and run:

```console
cargo run -- demo
```

Before submitting a pull request, run the same quality checks used by CI:

```console
cargo fmt --all -- --check
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
```

Keep changes focused, add tests for behavior changes, and update user-facing documentation when commands, configuration, or output change. Update `dist-workspace.toml` and regenerate `.github/workflows/release.yml` with cargo-dist when release behavior changes. The generated workflow intentionally carries audited, immutable Action SHAs; reapply the reviewed pins after regeneration and run `./scripts/verify-actions-pinned.sh`.

## Pull requests

Explain the problem, the chosen approach, and how the change was tested. Maintainers may ask for changes to preserve cross-platform behavior, bounded resource usage, terminal responsiveness, or configuration compatibility.

By submitting a contribution, you agree that it is licensed under the [Apache License 2.0](LICENSE).
