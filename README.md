# gh-inbox

`gh-inbox` is a precompiled GitHub CLI extension for clearing a noisy notifications inbox.

It provides a small command surface that is intentionally aligned with the way `gh` subcommands read and behave:

- `gh inbox list`
- `gh inbox sweep`
- `gh inbox save`

## Commands

### `gh inbox list`

Lists notifications that are still in the inbox. This includes both unread and already-read notifications that have not been marked as done yet.

The output also shows whether a notification is protected by the local save registry.

### `gh inbox sweep`

Marks matching notifications as done.

Supported filters:

- `--read`: only notifications that are already read
- `--closed`: only closed or merged pull requests
- `--repo OWNER/REPO`: only a single repository
- `--user LOGIN`: only pull requests opened by the given user
- `--team-mentioned`: only notifications whose reason is `team_mention`
- `--no-mentioned`: only notifications where the reason is not `mention`

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
```

### `gh inbox save`

Saves a pull request locally so future `sweep` runs skip it.

Example:

```console
gh inbox save --repo cli/cli --pr 123
```

## Important note about `save`

GitHub's web inbox has a native Saved state, but the public Notifications API does not expose a matching save endpoint. Because of that, `gh inbox save` uses a local save registry instead of toggling the Saved state in the GitHub web UI.

The registry is stored in the standard user config directory:

- macOS: `~/Library/Application Support/gh-inbox/saved-pull-requests.json`
- Linux: `${XDG_CONFIG_HOME:-~/.config}/gh-inbox/saved-pull-requests.json`

## Installation

Once a release exists, install the extension with:

```console
gh extension install OWNER/gh-inbox
```

This repository publishes precompiled Darwin binaries for:

- `darwin-amd64`
- `darwin-arm64`
- `linux-amd64`
- `linux-arm64`

## Development

The project uses:

- Rust 2024 edition
- `clap` for the CLI surface
- `reqwest` for the GitHub API client
- unit tests for the important filtering and save-registry logic

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

.github/workflows/release.yml runs on every push to `main`, verifies formatting, linting, and tests on Ubuntu, builds Darwin assets on macOS, builds Linux assets on Ubuntu via Docker and QEMU, then creates or updates a rolling release named `main-<short-sha>`.

The workflow uploads the four precompiled binaries that `gh extension install` expects for the supported Darwin and Linux architectures.
