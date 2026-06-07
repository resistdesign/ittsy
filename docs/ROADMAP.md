# Roadmap

## Product Contract

tinyterm should appear quickly, stay out of the way, and provide a real shell
in a small always-available window. Features must justify their effect on
startup time, memory, binary size, and interaction complexity.

## v0.1 - Usable Core

- [x] Native compact window
- [x] Real PTY-backed `/bin/bash` session
- [x] ANSI/VT screen parsing
- [x] Keyboard input and paste
- [x] Resize propagation to the PTY
- [x] Tiny bitmap-font framebuffer renderer
- [x] Focused unit tests and release-size settings
- [x] CI, SemVer release artifacts, and GitHub Pages deployment

## Next

- [ ] Remember window position and size
- [ ] Add visible scrollback navigation
- [ ] Add an optional always-on-top preference
- [ ] Add mouse text selection and clipboard copy
- [ ] Add keyboard-layout-aware text input
- [ ] Package as a signed macOS `.app`
- [ ] Add Apple Silicon release artifacts
- [ ] Measure cold startup, idle memory, and input latency
- [ ] Improve terminal color and text-attribute rendering
- [ ] Add Linux shell selection and packaging
- [ ] Evaluate Windows ConPTY support

## Known Limitations

- macOS and `/bin/bash` are the supported combination today.
- Mouse-reporting terminal applications are not supported.
- Keyboard text input currently assumes a US layout.
- Clipboard copy and mouse selection are not supported yet.
- Cell colors and text attributes are currently rendered with one compact
  theme rather than full per-cell styling.
- Characters outside the built-in bitmap font render as `?`.

## Baseline Footprint

Measured on macOS 26.5.1, Intel, on 2026-06-06:

- Release executable: 457 KB
- Idle resident memory: approximately 45 MB
- Idle application CPU: approximately 3.2%
- Idle bash CPU: 0%

These are development-machine snapshots, not guarantees. Idle CPU is the main
performance target: `minifb` polls native events at 15 Hz even when the terminal
screen is unchanged.

## Decision Log

### 2026-06-06: Use minifb, portable-pty, and vt100

This combination provides a small native framebuffer, real PTY semantics, and
proven terminal parsing. An earlier `eframe` prototype worked but idled around
84 MB RSS. The framebuffer renderer is deliberately narrow and uses a built-in
bitmap font to keep tinyterm aligned with its product goal.

### 2026-06-06: Keep one shell per process

Tabs, panes, sessions, and profile management are excluded. They compete with
the core goal and are already handled by full terminal applications.
