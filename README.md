# gh-inbox

 A GitHub CLI extension implemented in Rust for clearing a noisy notifications inbox.

 ## Getting Started

```console
$ gh extension install mi2428/gh-inbox
```

```console
$ gh inbox --help
Manage GitHub notifications from the inbox.

Usage: gh inbox <COMMAND>

Commands:
  sweep  Mark matching notifications as done

Options:
  -h, --help     Print help
  -V, --version  Print version

Examples:
  gh inbox sweep
  gh inbox sweep --include-authored
  gh inbox sweep --closed --repo mi2428/helloworld --user renovate
  gh inbox sweep --team-mentioned --no-mentioned
```

By default, `gh inbox sweep` protects notifications for pull requests authored by the authenticated user.
Use `--include-authored` to include them in a sweep.

```console
$ gh inbox sweep --help
Mark matching notifications as done

Usage: gh inbox sweep [OPTIONS]

Options:
      --closed             Only sweep pull request notifications whose pull requests are closed or merged
      --repo <OWNER/REPO>  Only sweep notifications from the given repository
      --user <USER>        Only sweep pull request notifications opened by the given user
      --team-mentioned     Only sweep notifications whose reason is team_mention
      --no-mentioned       Only sweep notifications where the reason is not mention
      --include-authored   Also sweep pull request notifications authored by the authenticated user
  -h, --help               Print help
  -V, --version            Print version
```

## Development

```console
$ make

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
  release      Build all binaries for the version in Cargo.toml on origin/main and publish a GitHub Release

Help
  help         Show this help message

Examples:
  make build
  make clean install
  make dist OS=darwin
  make dist OS=darwin,linux ARCH=amd64,arm64
  make -n release
  make releasee
```

## License

MIT License. See [LICENSE](LICENSE) for details.
