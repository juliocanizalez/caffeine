# Contributing to caffeine

Thanks for taking the time to contribute! Here's everything you need to get started.

## Requirements

- Rust (latest stable via `rustup`)
- macOS (IOKit and CoreGraphics are macOS-only)

## Getting started

```bash
git clone https://github.com/juliocanizalez/caffeine.git
cd caffeine
cargo build
cargo test
```

## Making changes

This project follows **Clean Architecture** — dependencies only flow inward:

```
domain ← application ← infrastructure ← presentation (main.rs)
```

- New I/O or FFI → new trait in `src/domain/ports.rs` + impl in `src/infrastructure/`
- No FFI outside `src/infrastructure/`
- No file I/O outside `src/infrastructure/ipc.rs`
- All `CaffeineService` deps injected via constructor (`Box<dyn Trait>`)

## Before submitting a PR

```bash
cargo fmt          # format
cargo clippy --all-targets  # lint (warnings = errors)
cargo test         # all tests must pass
```

## Commit messages

This project uses [Conventional Commits](https://www.conventionalcommits.org):

```
feat: add duration flag to status command
fix: prevent duplicate assertions on rapid toggle
docs: update install instructions
```

## Reporting bugs

Use the [bug report template](.github/ISSUE_TEMPLATE/bug_report.md) and include your macOS version and `caffeine --version` output.
