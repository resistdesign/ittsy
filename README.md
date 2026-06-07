# ittsy

A tiny, corner-friendly bash terminal written in Rust.

ittsy opens one lightweight native window and one real `/bin/bash` session.
It deliberately avoids tabs, panes, profiles, plugins, and configuration
screens.

## Run

Download the macOS `.app` from the latest release, unzip it, and open it
directly. Building from source requires a current Rust toolchain and macOS:

```sh
cargo run --release
```

Keyboard shortcuts:

- `Cmd+C` copies selected terminal text.
- `Ctrl+C` interrupts the running command.
- `Cmd+V` pastes into the shell.
- Mouse wheel and `Shift+Page Up` / `Shift+Page Down` navigate scrollback.
- `Cmd+Option+Arrow` docks the window to that screen edge; combine a horizontal
  and vertical direction to choose any corner.
- `Cmd+Option+T` toggles whether the window stays above other windows.
- `Cmd+Q` quits.
- Arrow keys, Home, End, Page Up, Page Down, Tab, Backspace, Escape, and the
  usual `Ctrl+letter` shell controls are supported.

## Status

This is an intentionally narrow first release. See [docs/ROADMAP.md](docs/ROADMAP.md)
for current limitations and planned work.

## Releases

ittsy uses Semantic Versioning. Tagged releases build and publish a
checksummed Intel macOS `.app` automatically. See [RELEASE.md](RELEASE.md).

Project site: [ittsy.resist.design](https://ittsy.resist.design)

## License

MIT
