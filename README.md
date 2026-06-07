# ittsy

A tiny, corner-friendly bash terminal written in Rust.

ittsy opens one lightweight native window and one real `/bin/bash` session.
It deliberately avoids tabs, panes, profiles, plugins, and configuration
screens.

## Run

Requirements: a current Rust toolchain and macOS.

```sh
cargo run --release
```

Keyboard shortcuts:

- `Ctrl+C` interrupts the running command.
- `Cmd+V` pastes into the shell.
- `Cmd+Q` quits.
- Arrow keys, Home, End, Page Up, Page Down, Tab, Backspace, Escape, and the
  usual `Ctrl+letter` shell controls are supported. Keyboard text input uses a
  US layout in this first release.

## Status

This is an intentionally narrow first release. See [docs/ROADMAP.md](docs/ROADMAP.md)
for current limitations and planned work.

## Releases

ittsy uses Semantic Versioning. Tagged releases build and publish a
checksummed Intel macOS archive automatically. See [RELEASE.md](RELEASE.md).

Project site: [ittsy.resist.design](https://ittsy.resist.design)

## License

MIT
