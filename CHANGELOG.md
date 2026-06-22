# Changelog

All notable changes to this project will be documented in this file.

<!-- next-release -->

## [Unreleased]

### Features
- feat(config): add customizable menu duration presets

---

## [v0.3.5] - 2025-05-01

### Bug Fixes
- fix(jiggle): post F16 key-up so jiggle registers as user input

---

## [v0.3.4] - 2025-04-28

### Bug Fixes
- fix(jiggle): use null flags-changed event to avoid browser media controls

---

## [v0.3.3] - 2025-04-20

### Chores
- chore: track Cargo.lock for reproducible builds

---

## [v0.3.2] - 2025-04-15

### Chores
- chore(formula): remove local Formula/ dir managed by homebrew-tap repo

---

## [v0.3.1] - 2025-04-10

### Bug Fixes
- fix(config): resolve config path to ~/.config on macOS

---

## [v0.3.0] - 2025-04-05

### Features
- feat(update): check GitHub releases for newer version on startup
- feat(login-item): add Launch at Login toggle via launchd plist
- feat(battery): auto-deactivate on low battery via IOKit
- feat(cli): add completions subcommand and status --json flag
- feat(config): add config.toml support with CaffeineConfig

---

## [v0.2.2] - 2025-03-20

### Features
- feat(tray): replace PNG icons with SVG rendered via resvg

### Refactors
- refactor: migrate to clean architecture with domain/application/infrastructure layers
- refactor(ipc): remove unused started_at field from SessionInfo

### CI
- ci(release): auto-update homebrew-tap after build

---

## [v0.2.0] - 2025-03-10

### Features
- feat(tray): add template icon, active/inactive states, and TTY daemon
- feat(tray): embed real SF Symbol PNGs as template icons

### Bug Fixes
- fix(tray): re-export icons as real PNG (sips wrote TIFF by default)
- fix(assets): center-crop icons to 1:1 before scaling to 44x44

---

## [v0.1.1] - 2025-03-01

### Build
- build(release): ship pre-built binaries for arm64 and x86_64

---

## [v0.1.0] - 2025-02-25

### Features
- feat(caffeine): initial implementation

### CI
- ci: add CI and release workflows
