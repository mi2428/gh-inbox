# gh-inbox

`gh-inbox` is a precompiled GitHub CLI extension for clearing a noisy notifications inbox.

It provides a small command surface that is intentionally aligned with the way `gh` subcommands read and behave:

- `gh inbox list`
- `gh inbox sweep`

## Commands

### `gh inbox list`

Lists notifications that are still in the inbox. This includes both unread and already-read notifications that have not been marked as done yet.

### `gh inbox sweep`

Marks matching notifications as done.

By default, notifications for pull requests authored by the authenticated user are protected and are not marked as done. Use `--include-authored` to include them in a sweep.

Supported filters:

- `--read`: only notifications that are already read
- `--closed`: only closed or merged pull requests
- `--repo OWNER/REPO`: only a single repository
- `--user LOGIN`: only pull requests opened by the given user
- `--team-mentioned`: only notifications whose reason is `team_mention`
- `--no-mentioned`: only notifications where the reason is not `mention`
- `--include-authored`: also sweep pull request notifications authored by the authenticated user

Filters are combined with logical AND.

Examples:

```console
gh inbox sweep
gh inbox sweep --read
gh inbox sweep --closed
gh inbox sweep --repo cli/cli
gh inbox sweep --read --closed --repo cli/cli --user monalisa
gh inbox sweep --team-mentioned
gh inbox sweep --no-mentioned
gh inbox sweep --include-authored
```

## Installation

Once a release exists, install the extension with:

```console
gh extension install OWNER/gh-inbox
```

This repository publishes precompiled binaries for:

- `darwin-amd64`
- `darwin-arm64`
- `linux-amd64`
- `linux-arm64`

## Development

The project uses:

- Rust 2024 edition
- `clap` for the CLI surface
- `reqwest` for the GitHub API client
- unit tests for the important filtering logic

Common tasks:

```console
make help
make build
make test
make dist-darwin RELEASE_TAG=main-$(git rev-parse --short=12 HEAD)
make dist-linux RELEASE_TAG=main-$(git rev-parse --short=12 HEAD)
make dist RELEASE_TAG=main-$(git rev-parse --short=12 HEAD)
make publish-release TAG=v0.1.0
make release TAG=v0.1.0
make release-main
```

`make dist-darwin` requires `rustup` so the Darwin Rust targets can be installed automatically.

`make dist-linux` requires Docker. It builds Linux binaries inside the official Rust container and supports both `linux/amd64` and `linux/arm64`.

## Release automation

[.github/workflows/release.yml](.github/workflows/release.yml) runs on every push to `main`, verifies formatting, linting, and tests on Ubuntu, builds Darwin assets on macOS, builds Linux assets on Ubuntu via Docker and QEMU, then creates or updates a rolling release named `main-<short-sha>`.

The workflow uploads the four precompiled binaries that `gh extension install` expects for the supported Darwin and Linux architectures.
