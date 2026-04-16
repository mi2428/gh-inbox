# gh-inbox

`gh-inbox` is a GitHub CLI extension implemented in Rust for clearing a noisy notifications inbox.

This repository is intended to be built and installed from a local checkout.

The extension currently provides two commands:

- `gh inbox list`
- `gh inbox sweep`

By default, `gh inbox sweep` protects notifications for pull requests authored by the authenticated user. Use `--include-authored` to include them in a sweep.

## Local installation

Build the host binary, create the repository-root entrypoint that `gh extension install .` expects, then install the extension from the current checkout:

```console
make install-local
gh extension install .
gh inbox --help
```

After source changes, run `make build` to refresh `bin/gh-inbox`. Run `make install-local` again if you want to recreate the repository-root entrypoint.

## Local binary builds

Use the `dist` targets when you want local cross-built binaries under `dist/`:

```console
make dist-darwin
make dist-linux
make dist
```

The generated binaries use filenames such as `dist/gh-inbox_darwin-amd64` and `dist/gh-inbox_linux-arm64`.

## CLI help

```console
$ cargo run -- --help
Manage GitHub notifications from the inbox.

Usage: gh inbox <COMMAND>

Commands:
  list   List notifications that are still in the inbox
  sweep  Mark matching notifications as done

Options:
  -h, --help     Print help
  -V, --version  Print version

Examples:
  gh inbox list
  gh inbox sweep
  gh inbox sweep --read
  gh inbox sweep --include-authored
  gh inbox sweep --closed --repo cli/cli --user monalisa
  gh inbox sweep --team-mentioned --no-mentioned
```

```console
$ cargo run -- sweep --help
Mark matching notifications as done

Usage: gh inbox sweep [OPTIONS]

Options:
      --read               Only sweep notifications that are already marked as read
      --closed             Only sweep pull request notifications whose pull requests are closed or merged
      --repo <OWNER/REPO>  Only sweep notifications from the given repository
      --user <LOGIN>       Only sweep pull request notifications opened by the given user
      --team-mentioned     Only sweep notifications whose reason is team_mention
      --no-mentioned       Only sweep notifications where the reason is not mention
      --include-authored   Also sweep pull request notifications authored by the authenticated user
  -h, --help               Print help
  -V, --version            Print version
```

## Development help

```console
$ make help

Development
  build           Build the host binary into bin/
  install-local   Build the host binary and create the repo-root entrypoint for gh extension install .
  fmt             Format the Rust sources
  fmt-check       Verify formatting without changing files
  lint            Run clippy with warnings treated as errors
  test            Run the unit test suite
  docker-check    Verify that Docker is available for Linux cross-builds
  dist-darwin     Build all Darwin binaries into dist/
  dist-linux      Build all Linux binaries into dist/

Distribution
  dist            Build all local cross-platform binaries into dist/
  clean           Remove build artifacts and the local gh extension entrypoint

Help
  help            Show this help message

Darwin Architectures:
  amd64 -> x86_64-apple-darwin
  arm64 -> aarch64-apple-darwin

Linux Architectures:
  amd64 -> linux/amd64
  arm64 -> linux/arm64

Examples:
  make build
  make install-local
  make test
  make dist-darwin
  make dist-linux
  make dist
```

`make dist-darwin` requires `rustup` so the Darwin Rust targets can be installed automatically.

`make dist-linux` requires Docker. It builds Linux binaries inside the official Rust container and supports both `linux/amd64` and `linux/arm64`.
