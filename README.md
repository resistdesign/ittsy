# ittsy

A tiny, corner-friendly terminal written in Rust.

ittsy opens one lightweight native window and one real shell session.
It deliberately avoids tabs, panes, profiles, plugins, and configuration
screens.

## Run

Download the macOS, Linux, or Windows archive from the latest release. Building
from source requires a current Rust toolchain:

```sh
cargo run --release
```

Keyboard shortcuts:

- `Cmd+C` copies selected terminal text on macOS; use `Ctrl+Shift+C` on Linux
  and Windows.
- `Ctrl+C` interrupts the running command.
- `Cmd+V` pastes into the shell on macOS; use `Ctrl+Shift+V` on Linux and
  Windows.
- Mouse wheel and `Shift+Page Up` / `Shift+Page Down` navigate scrollback.
- `Cmd+Option+Arrow` on macOS or `Ctrl+Alt+Arrow` elsewhere docks the window to
  that screen edge; combine directions to choose any corner.
- `Cmd+Option+T` on macOS or `Ctrl+Alt+T` elsewhere toggles always-on-top.
- `Cmd+Q` quits.
- Arrow keys, Home, End, Page Up, Page Down, Tab, Backspace, Escape, and the
  usual `Ctrl+letter` shell controls are supported.

## Status

This is an intentionally narrow first release. See [docs/ROADMAP.md](docs/ROADMAP.md)
for current limitations and planned work.

## Releases

ittsy uses Semantic Versioning. Tagged releases build and publish checksummed
macOS, Linux, and Windows archives automatically. See [RELEASE.md](RELEASE.md).

Project site: [ittsy.resist.design](https://ittsy.resist.design)

## License

MIT
