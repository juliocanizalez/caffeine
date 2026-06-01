# caffeine

[![CI](https://github.com/juliocanizalez/caffeine/actions/workflows/ci.yml/badge.svg)](https://github.com/juliocanizalez/caffeine/actions/workflows/ci.yml)
[![Latest Release](https://img.shields.io/github/v/release/juliocanizalez/caffeine?color=orange)](https://github.com/juliocanizalez/caffeine/releases/latest)
[![Homebrew Tap](https://img.shields.io/badge/homebrew-juliocanizalez%2Ftap-blue?logo=homebrew)](https://github.com/juliocanizalez/homebrew-tap)
[![License: MIT](https://img.shields.io/badge/license-MIT-green)](LICENSE)
[![Rust](https://img.shields.io/badge/built%20with-Rust-orange?logo=rust)](https://www.rust-lang.org)
[![Platform](https://img.shields.io/badge/platform-macOS-lightgrey?logo=apple)](https://www.apple.com/macos)

Your Mac falls asleep. Your status goes red. Your boss notices. You were just reading a long document, but now you look like you vanished. We've all been there.

`caffeine` keeps your Mac awake, your screen on, and your presence indicator green so the surveillance software your company calls a "collaboration tool" thinks you're absolutely crushing it. It also puts a ☕ icon in your menu bar, which `/usr/bin/caffeinate` does not, and which is arguably the whole point.

Written in Rust because of course it is. Seventeen crates. Two system frameworks. One coffee emoji. Totally worth it.

---

## Installation

**Via Homebrew (recommended once you've accepted this decision):**

```bash
brew tap juliocanizalez/tap
brew install juliocanizalez/tap/caffeine --formula
```

> `--formula` is required because an unrelated GUI app called Caffeine exists in homebrew-cask. Without it, Homebrew will install that instead and leave you confused.

**Via Cargo (for the kind of person who already has Cargo):**

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
caffeine 90s          # 90 seconds, for the truly impatient
caffeine 2h15m30s     # yes, this works too, if you're that kind of person
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
caffeine status       # ● Active  1h 23m remaining
caffeine stop         # gracefully terminates the running instance
```

The `status` output also tells you which sleep mode is active and whether Keep Status Active is on. Stale PID files from unclean exits are detected and cleaned up automatically, so you don't have to think about them.

The menu bar icon is a minimal coffee cup rendered from SVG (filled when active, hollow when inactive) and it adapts to dark mode and wallpaper colour as a proper template image. Click it for quick presets (15m / 30m / 1h / 2h / 4h / Indefinite), a **Keep Status Active** toggle, a Stop / Resume toggle, and a Quit option for when you've come to your senses.

---

## Uninstalling

```bash
caffeine stop
brew uninstall juliocanizalez/tap/caffeine
```

---

## How it works

Two IOKit power assertions are acquired at startup via direct FFI against Apple's `IOKit.framework`:

`PreventUserIdleDisplaySleep` keeps the screen on. `NoIdleSleepAssertion` prevents the system from idle sleeping. Both are released the moment the process exits, regardless of how: `SIGTERM`, `SIGKILL`, a well-aimed `killall`, or the heat death of your MacBook's battery. This is an OS-level guarantee: IOKit ties assertion lifetimes to process lifetimes. The RAII `Drop` implementation in the code is purely ceremonial at this point, included as a matter of professional pride.

A note on `"NoDisplaySleep"`: you'll find this string in various forum posts suggesting it's the correct assertion for display sleep prevention. It isn't, not for user-space processes. It requires a special entitlement that Apple reserves for daemons. Attempting to use it returns `kIOReturnNotPrivileged (0xe00002c2)`, which is the OS's polite way of saying no. `"PreventUserIdleDisplaySleep"` is the correct public API, and it's exactly what Activity Monitor reports when apps are preventing display sleep.

### Keep Status Active (`-k`)

IOKit sleep assertions prevent the system from sleeping, but Teams, Slack, and similar apps determine your "online" status by reading the HID idle timer (seconds since the last real mouse or keyboard event), completely independently of sleep prevention.

When `-k` is active (or toggled on from the menu bar), caffeine periodically posts a tiny synthetic mouse event via `CGEventCreateMouseEvent` + `CGEventPost`, moving the cursor 1 px right and immediately back. This resets the HID idle timer and fools presence detection into thinking you're at your desk.

**Smart pause:** caffeine first checks `CGEventSourceSecondsSinceLastEventType`. If you've been genuinely active within the last 5 minutes, the jiggle is skipped. No unnecessary cursor movement while you're typing. Once real inactivity exceeds 5 minutes, a jiggle fires every 60 seconds until activity resumes or caffeine is stopped.

---

## Performance

Measured on Apple M-series (arm64, debug build, `cargo test -- --nocapture`):

| Operation | Avg latency |
|---|---|
| `idle_seconds()` via `CGEventSourceSecondsSinceLastEventType` | ~30 ns |
| `jiggle()` via two `CGEventPost` calls + cursor restore | ~9 µs |
| RSS growth over 500 jiggle calls | < 500 KB |

The 500 ms event-loop tick means jiggle overhead is immeasurable in practice. The check runs once per tick, does a single CGEventSource query (~30 ns), and only posts events after 5 minutes of verified idleness.

Run `cargo test -- --nocapture` to reproduce these numbers on your machine.

---

## Status file

When running, `caffeine` writes its state to:

```
~/Library/Application Support/caffeine/status
```

This is what `caffeine status` reads. The file contains five fields: PID, start time, expiry timestamp (or 0 for indefinite), display sleep prevention flag, and jiggle flag. If the process is killed without a clean exit, the file may persist. Subsequent calls to `status` or `stop` detect the stale PID and clean it up automatically.

---

## What's missing (intentionally)

`--pid` mode (`caffeinate -w`), AC-only mode, low-battery cutoff, and autostart on login are all reasonable additions. Feel free to open a PR, or simply install Amphetamine from the App Store and spare yourself the whole exercise.

---

## License

MIT. See [LICENSE](LICENSE).
