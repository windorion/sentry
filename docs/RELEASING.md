# Releasing

Releases use cargo-dist 0.31.0 and the generated `.github/workflows/release.yml` workflow.

## Supported artifacts

- Apple Silicon macOS (`aarch64-apple-darwin`)
- Intel macOS (`x86_64-apple-darwin`)
- ARM64 Linux GNU (`aarch64-unknown-linux-gnu`)
- x64 Linux GNU (`x86_64-unknown-linux-gnu`)
- x64 Windows MSVC (`x86_64-pc-windows-msvc`)
- Shell and PowerShell installers
- Per-archive and unified SHA-256 checksums

## Preflight

1. Update the package version and `CHANGELOG.md`.
2. Run the full local quality suite from the README.
3. Run `dist generate` and confirm it leaves no unexpected diff.
4. Run `dist plan --output-format=json --no-local-paths`.
5. Push the normal commit and wait for CI to pass.

## Publish

Publishing is intentionally a separate, explicit operation:

```console
git tag -s v0.2.0 -m "Windorion Sentry v0.2.0"
git push origin v0.2.0
```

The tag version must match `Cargo.toml`. The Release workflow builds each platform on a native GitHub runner, creates archives/installers/checksums, and then creates the GitHub Release only after all required jobs succeed.

Do not tag a commit merely to test ordinary changes. Pull requests exercise the generated release plan without publishing a release.

## Homebrew

Create and secure a dedicated `windorion/homebrew-tap` repository first. Then enable cargo-dist's Homebrew installer/publisher, regenerate the workflow, validate on a prerelease, and only then document `brew install windorion/tap/wsentry` as supported.
