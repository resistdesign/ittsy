# ittsy Agent Guide

## Mission

Build a genuinely small, fast, corner-friendly terminal application. Keep the
default experience focused: one window, one shell, no tabs, no profiles, and no
configuration UI unless real usage demonstrates a need.

## Current Architecture

- `eframe`/`egui` owns the native window, native text input, and monospace
  rendering.
- `portable-pty` starts and communicates with the shell through a real PTY.
- `vt100` maintains terminal screen state from ANSI/VT output.
- PTY reads happen on a background thread and request UI repaints only when
  shell output arrives.

The first target is macOS and `/bin/bash`. Keep platform-specific policy in
small functions so Linux and Windows support can follow without rewriting the
terminal core.

## Working Agreement

1. Read `docs/ROADMAP.md` before starting larger work.
2. Keep `cargo fmt`, `cargo test`, and `cargo clippy --all-targets -- -D warnings`
   passing.
3. Test pure behavior such as key encoding and size calculations directly.
4. Prefer removing scope over adding settings or abstractions.
5. Update the roadmap when behavior, architecture, or priorities change.
6. Never commit generated `target/` contents or credentials.
7. Keep `Cargo.toml`, site structured data, and release tags on the same
   Semantic Version.

## Common Commands

```sh
cargo run
cargo fmt --check
cargo test
cargo clippy --all-targets -- -D warnings
cargo build --release
```

Release and deployment details live in `RELEASE.md`. The static project site is
in `site/` and deploys to GitHub Pages from `main`.

On this development machine, `/usr/local/bin/cargo` does not expose component
subcommands. If `cargo fmt` or `cargo clippy` is missing, prepend the active
toolchain bin directory to `PATH` rather than changing project checks.

## Definition Of Done

A change is done when it works in the actual app, automated checks pass, and
the durable docs describe any new constraint or remaining follow-up.
