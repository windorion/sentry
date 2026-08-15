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

## Signing status

The 1.0 release archives are integrity-checked with SHA-256 but are not yet Apple-notarized or Authenticode-signed. The shell installer does not request administrator privileges and installs into the user's Cargo binary directory. Windows may display a SmartScreen warning for the unsigned executable. Code signing requires project-owned Apple Developer and Windows signing identities and is tracked as a post-1.0 distribution improvement; never place signing credentials in the repository.

## Preflight

1. Update the package version and `CHANGELOG.md`.
2. Run the full local quality suite from the README.
3. Run `dist generate --mode ci`, review the generated diff, and reapply audited full-SHA Action pins.
4. Run `./scripts/verify-actions-pinned.sh` and `dist plan --output-format=json --no-local-paths --allow-dirty`.
5. Run `wsentry validate --config wsentry.example.toml` and package smoke tests.
6. Push the normal commit and wait for CI to pass on Linux, macOS, and Windows.

## Publish

Publishing is intentionally a separate, explicit operation:

```console
git tag -a vX.Y.Z -m "Windorion Sentry vX.Y.Z"
git push origin vX.Y.Z
```

The tag version must match `Cargo.toml`. Use a signed tag when a configured signing identity is available; otherwise use an annotated tag and rely on the protected GitHub release workflow and checksums. The workflow builds each platform on a native GitHub runner, creates archives/installers/checksums, and then creates the GitHub Release only after all required jobs succeed.

Before tagging, replace the changelog's `Unreleased` marker with the release date. After the workflow completes, download at least one Unix archive and the Windows archive, verify checksums, and run `wsentry --version` plus `wsentry doctor --json`. Only then announce the release.

Do not tag a commit merely to test ordinary changes. Pull requests run the read-only cross-platform CI workflow; the release workflow runs only for version tags so untrusted pull-request code never receives release permissions.

## Homebrew

Create and secure a dedicated `windorion/homebrew-tap` repository first. Then enable cargo-dist's Homebrew installer/publisher, regenerate the workflow, validate on a prerelease, and only then document `brew install windorion/tap/wsentry` as supported.
