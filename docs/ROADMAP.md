# Roadmap

## Product Contract

ittsy should appear quickly, stay out of the way, and provide a real shell
in a small always-available window. Features must justify their effect on
startup time, memory, binary size, and interaction complexity.

## v0.1 - Usable Core

- [x] Native compact window
- [x] Real PTY-backed `/bin/bash` session
- [x] ANSI/VT screen parsing
- [x] Keyboard input and paste
- [x] Resize propagation to the PTY
- [x] Native text input and readable monospace rendering
- [x] Mouse text selection and clipboard copy
- [x] Mouse and keyboard scrollback navigation
- [x] ANSI 16-color, 256-color, and truecolor rendering
- [x] Corner docking and optional always-on-top behavior
- [x] Native app and bundle icons matching the project site
- [x] Focused unit tests and release-size settings
- [x] CI, SemVer release artifacts, and GitHub Pages deployment
- [x] Standalone macOS `.app` release packaging

## Next

- [ ] Remember window position and size
- [ ] Developer ID sign and notarize the macOS `.app`
- [ ] Add Apple Silicon release artifacts
- [ ] Measure cold startup, idle memory, and input latency
- [ ] Add Linux shell selection and packaging
- [ ] Evaluate Windows ConPTY support

## Known Limitations

- macOS and `/bin/bash` are the supported combination today.
- Mouse-reporting terminal applications are not supported.
- Bold text uses brighter ANSI colors because the bundled monospace font has
  one weight.
- Complex Unicode grapheme widths may not align perfectly.

## Baseline Footprint

Measured on macOS 26.5.1, Intel, on 2026-06-07:

- Release executable: 5.5 MB
- Zipped `.app`: 2.4 MB
- Idle resident memory: approximately 82 MB
- Settled idle application CPU: 0.0% in a point-in-time sample

The broken v0.1.0 framebuffer build was smaller, but sacrificed reliable input
and readable text. The v0.1.1 event-driven native text path is the functional
baseline; input latency still needs a repeatable benchmark.

## Decision Log

### 2026-06-07: Restore eframe for correct input and text

The 15 Hz `minifb` loop sampled keyboard state too slowly and the stretched
8x8 bitmap font was visibly distorted. Restore the earlier `eframe` path for
native text events, keyboard-layout support, readable monospace rendering, and
event-driven idle behavior. Correctness takes priority over the smaller broken
binary.

### 2026-06-07: Make the compact window terminal-complete

Expose the existing bounded VT scrollback through wheel and keyboard input,
render per-cell ANSI colors and attributes, and make bottom-right,
always-on-top placement the default. Keep corner movement and floating-window
control as keyboard shortcuts instead of adding a configuration interface.

### 2026-06-06: Keep one shell per process

Tabs, panes, sessions, and profile management are excluded. They compete with
the core goal and are already handled by full terminal applications.
