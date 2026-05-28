# caffeine

> Another app you almost certainly don't need, rewritten in Rust.

`caffeine` prevents your Mac from going to sleep. It does precisely what `/usr/bin/caffeinate` — the utility that has shipped with macOS since Mountain Lion — already does, except with a persistent menu bar icon, a live countdown, and the quiet dignity of having been compiled to native machine code by a language that required seventeen crates to link against two system frameworks.

In its defence: the built-in `caffeinate` doesn't put a ☕ emoji in your menu bar. So there's that.

---

## Installation

**Via Homebrew (recommended once you've accepted this decision):**

```bash
brew tap juliocanizalez/tap
brew install caffeine
```

**Via Cargo (for the kind of person who already has Cargo):**

```bash
cargo install --git https://github.com/juliocanizalez/caffeine
```

---

## Usage

```bash
caffeine              # indefinitely — menu bar shows ☕ ∞
caffeine 30           # 30 minutes   — plain number means minutes
caffeine 2h           # 2 hours
caffeine 1h30m        # 1 hour 30 minutes
caffeine 90s          # 90 seconds, for the truly impatient
caffeine --no-display # keeps the system awake but lets the screen dim
```

From another terminal, while caffeine is running:

```bash
caffeine status       # ● Active — 1h 23m remaining
caffeine stop         # gracefully terminates the running instance
```

The menu bar icon counts down live and offers presets (15m / 30m / 1h / 2h / 4h / Indefinite), a Stop/Resume toggle, and a Quit option for when you've come to your senses.

---

## How it works

Two IOKit power assertions are acquired at startup via direct FFI against Apple's `IOKit.framework`:

- **`PreventUserIdleDisplaySleep`** — keeps the screen on
- **`NoIdleSleepAssertion`** — prevents the system from idle sleeping

Both are released the moment the process exits, regardless of how — `SIGTERM`, `SIGKILL`, a well-aimed `killall`, or the heat death of your MacBook's battery. This is an OS-level guarantee: IOKit ties assertion lifetimes to process lifetimes. The RAII `Drop` implementation in the code is purely ceremonial at this point, included as a matter of professional pride.

A note on `"NoDisplaySleep"`: you'll find this string in various forum posts suggesting it's the correct assertion for display sleep prevention. It isn't — not for user-space processes. It requires a special entitlement that Apple reserves for daemons. Attempting to use it returns `kIOReturnNotPrivileged (0xe00002c2)`, which is the OS's polite way of saying no. `"PreventUserIdleDisplaySleep"` is the correct public API, and it's exactly what Activity Monitor reports when apps are preventing display sleep.

---

## Status file

When running, `caffeine` writes its state to:

```
~/Library/Application Support/caffeine/status
```

This is what `caffeine status` reads. If the process is killed without a clean exit, the file may persist; subsequent calls to `status` or `stop` detect the stale PID and clean it up automatically.

---

## What's missing (intentionally)

- **`--pid` mode** (`caffeinate -w`): stay awake while another process runs
- **AC-only mode**: auto-deactivate when unplugged
- **Low-battery cutoff**: release assertions below X%
- **Autostart on login**: `launchctl` plumbing

These would all be reasonable additions. Feel free to open a PR, or simply install Amphetamine from the App Store and spare yourself the whole exercise.

---

## License

MIT. See [LICENSE](LICENSE).
