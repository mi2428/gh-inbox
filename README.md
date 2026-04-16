# gh-inbox

`gh-inbox` is a GitHub CLI extension implemented in Rust for clearing a noisy notifications inbox.

This repository is intended to be built locally by developers. Maintainers can also publish a remote-installable GitHub Release directly from the `Makefile`.

The extension currently provides two commands:

- `gh inbox list`
- `gh inbox sweep`

By default, `gh inbox sweep` protects notifications for pull requests authored by the authenticated user. Use `--include-authored` to include them in a sweep.

## Local installation

Install the extension from the current checkout with:

```console
make install
gh inbox --help
```

`make install` builds `bin/gh-inbox`, refreshes the repository-root launcher, and runs `gh extension install . --force`.

To remove the local extension registration and the repository-root launcher:

```console
make uninstall
```

## Remote installation

Maintainers can publish a release for the latest commit on `origin/main` with:

```console
make release
```

That command fetches `origin/main`, builds Darwin and Linux binaries for all supported architectures from that commit in a temporary worktree, and uploads all of them to a GitHub Release. After that, users can install the extension remotely with:

```console
gh extension install OWNER/gh-inbox
```

## Local binary builds

Use `make dist` when you want local cross-built binaries under `dist/`:

```console
make dist OS=darwin
make dist OS=linux ARCH=arm64
make dist OS=darwin,linux ARCH=amd64,arm64
```

The generated binaries use filenames such as `dist/gh-inbox-darwin-amd64` and `dist/gh-inbox-linux-arm64`.

## CLI help

```console
$ gh inbox --help
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
  gh inbox sweep --closed --repo mi2428/helloworld --user renovate
  gh inbox sweep --team-mentioned --no-mentioned
```

## Development help

```console
$ make help

Development
  build        Build the host binary into bin/
  install      Build the host binary, create the repo-root launcher, and install the local gh extension
  uninstall    Remove the local gh extension and delete the repo-root launcher
  fmt          Format the Rust sources
  fmt-check    Verify formatting without changing files
  lint         Run clippy with warnings treated as errors
  test         Run the unit test suite

Distribution
  dist         Build binaries into dist/. Use OS=darwin,linux and ARCH=amd64,arm64.
  clean        Remove build artifacts and the local launcher

Release
  release      Build all binaries from the latest origin/main commit and publish a GitHub Release

Help
  help         Show this help message

Darwin Architectures:
  amd64        x86_64-apple-darwin
  arm64        aarch64-apple-darwin

Linux Architectures:
  amd64        linux/amd64
  arm64        linux/arm64

Examples:
  make build
  make install
  make uninstall
  make test
  make dist OS=darwin
  make dist OS=linux ARCH=arm64
  make dist OS=darwin,linux ARCH=amd64,arm64
  make -n release
  make release
```

If `OS` includes `darwin`, `make dist` requires `rustup` so the Darwin Rust targets can be installed automatically.

If `OS` includes `linux`, `make dist` requires Docker. It builds Linux binaries inside the official Rust container and supports both `linux/amd64` and `linux/arm64`.

`make release` requires a GitHub repository remote named `origin`, a `main` branch on that remote, and an authenticated `gh` session with permission to create releases.
