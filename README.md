# caffeine

[![CI](https://github.com/juliocanizalez/caffeine/actions/workflows/ci.yml/badge.svg)](https://github.com/juliocanizalez/caffeine/actions/workflows/ci.yml)
[![Latest Release](https://img.shields.io/github/v/release/juliocanizalez/caffeine?color=orange)](https://github.com/juliocanizalez/caffeine/releases/latest)
[![Homebrew Tap](https://img.shields.io/badge/homebrew-juliocanizalez%2Ftap-blue?logo=homebrew)](https://github.com/juliocanizalez/homebrew-tap)
[![License: MIT](https://img.shields.io/badge/license-MIT-green)](LICENSE)
[![Rust](https://img.shields.io/badge/built%20with-Rust-orange?logo=rust)](https://www.rust-lang.org)
[![Platform](https://img.shields.io/badge/platform-macOS-lightgrey?logo=apple)](https://www.apple.com/macos)

Your Mac falls asleep. Your status goes red. Your boss notices. You were just reading a long document, but now you look like you vanished. We've all been there.

`caffeine` keeps your Mac awake, your screen on, and your presence indicator green so the surveillance software your company calls a "collaboration tool" thinks you're absolutely crushing it. It also puts a coffee icon in your menu bar, which `/usr/bin/caffeinate` does not, and which is arguably the whole point.

Written in Rust because of course it is.

---

## Installation

**Via Homebrew (recommended):**

```bash
brew tap juliocanizalez/tap
brew install juliocanizalez/tap/caffeine --formula
```

> `--formula` is required because an unrelated GUI app called Caffeine exists in homebrew-cask. Without it, Homebrew will install that instead and leave you confused.

**Via Cargo:**

```bash
cargo install --git https://github.com/juliocanizalez/caffeine
```

---

## Usage

```bash
caffeine              # starts in background, menu bar shows active icon
caffeine 30           # 30 minutes (plain number means minutes)
caffeine 2h           # 2 hours
caffeine 1h30m        # 1 hour 30 minutes
caffeine 90s          # 90 seconds
caffeine 2h15m30s     # combined duration
caffeine --no-display # keeps the system awake but lets the screen dim
caffeine -k           # also keeps Teams/Slack status active (see below)
caffeine -k 2h        # keep status active for 2 hours
```

When run from a terminal, caffeine detaches automatically and prints the PID:

```
Caffeine started (PID 12345)
```

From another terminal, while caffeine is running:

```bash
caffeine status            # prints human-readable status
caffeine status --json     # outputs status as JSON for scripting
caffeine stop              # gracefully terminates the running instance
```

The `status` output tells you which sleep mode is active, time remaining, and whether Keep Status Active is on. Stale PID files from unclean exits are detected and cleaned up automatically.

### Shell completions

Generate a completion script for your shell and source it:

```bash
# zsh
caffeine completions zsh >> ~/.zshrc

# fish
caffeine completions fish > ~/.config/fish/completions/caffeine.fish

# bash
caffeine completions bash >> ~/.bashrc
```

Supported shells: `bash`, `zsh`, `fish`, `elvish`, `powershell`.

### Menu bar

The menu bar icon is a minimal coffee cup (filled when active, hollow when inactive) that adapts to dark mode as a template image. Click it to access:

- **Status line** showing time remaining (updates every 500ms)
- **Duration presets**: 15m / 30m / 1h / 2h / 4h / Indefinite
- **Keep Status Active** toggle (jiggle mode)
- **Launch at Login** toggle
- **Update available** banner when a newer release is detected
- **Stop / Resume** toggle
- **Quit**

---

## Configuration

caffeine reads `~/.config/caffeine/config.toml` on startup. All fields are optional and CLI flags override config values.

```toml
# ~/.config/caffeine/config.toml

# Prevent display sleep in addition to system sleep (default: true)
prevent_display = true

# Enable jiggle mode by default (default: false)
keep_status_active = false

# Auto-deactivate when battery drops below this percentage (0 = disabled)
battery_threshold = 20

# Check GitHub for newer releases on startup (default: true)
check_for_updates = true
```

---

## Keep Status Active (`-k`)

IOKit sleep assertions prevent the system from sleeping, but Teams, Slack, and similar apps determine your "online" status by reading the HID idle timer independently of sleep prevention.

When `-k` is active (or toggled on from the menu bar), caffeine periodically posts a tiny synthetic mouse event via `CGEventCreateMouseEvent` + `CGEventPost`, moving the cursor 1px right and immediately back. This resets the HID idle timer and keeps presence detection from marking you away.

**Smart pause:** caffeine first checks `CGEventSourceSecondsSinceLastEventType`. If you have been genuinely active within the last 5 minutes, the jiggle is skipped. Once real inactivity exceeds 5 minutes, a jiggle fires every 60 seconds until activity resumes or caffeine is stopped.

---

## Battery guard

When `battery_threshold` is set in the config, caffeine checks the battery level every 30 seconds via `IOPSCopyPowerSourcesInfo`. If the level drops below the threshold, caffeine deactivates automatically and prints a message to stderr. Set to `0` (the default) to disable this behaviour entirely.

---

## Launch at Login

The **Launch at Login** toggle in the menu installs or removes a launchd plist at `~/Library/LaunchAgents/com.juliocanizalez.caffeine.plist`. The plist uses `RunAtLoad: false`, so caffeine starts on your next login rather than immediately. Removing the toggle unloads the agent and deletes the plist.

---

## How it works

Two IOKit power assertions are acquired at startup via direct FFI against `IOKit.framework`:

`PreventUserIdleDisplaySleep` keeps the screen on. `NoIdleSleepAssertion` prevents the system from idle sleeping. Both are released the moment the process exits, regardless of how it exits. This is an OS-level guarantee: IOKit ties assertion lifetimes to process lifetimes.

A note on `"NoDisplaySleep"`: you'll find this string in forum posts suggesting it's the correct assertion for display sleep prevention. It isn't, not for user-space processes. It requires a special entitlement that Apple reserves for daemons. Attempting to use it returns `kIOReturnNotPrivileged (0xe00002c2)`. `"PreventUserIdleDisplaySleep"` is the correct public API, and it's exactly what Activity Monitor reports when apps are preventing display sleep.

---

## Performance

Measured on Apple M-series (arm64, debug build):

| Operation | Avg latency |
|---|---|
| `idle_seconds()` via `CGEventSourceSecondsSinceLastEventType` | ~30 ns |
| `jiggle()` via two `CGEventPost` calls + cursor restore | ~9 us |
| RSS growth over 500 jiggle calls | < 500 KB |

Run `cargo test -- --nocapture` to reproduce these numbers on your machine.

---

## Status file

When running, `caffeine` writes its state to:

```
~/Library/Application Support/caffeine/status
```

This is what `caffeine status` reads. The file contains the PID, expiry timestamp (or 0 for indefinite), display sleep prevention flag, and jiggle flag. If the process is killed without a clean exit, the file may persist. Subsequent calls to `status` or `stop` detect the stale PID and clean it up automatically.

---

## Uninstalling

```bash
caffeine stop
brew uninstall juliocanizalez/tap/caffeine
```

---

## License

MIT. See [LICENSE](LICENSE).
