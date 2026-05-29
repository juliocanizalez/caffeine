# caffeine

[![CI](https://github.com/juliocanizalez/caffeine/actions/workflows/ci.yml/badge.svg)](https://github.com/juliocanizalez/caffeine/actions/workflows/ci.yml)

Another app you almost certainly don't need, rewritten in Rust.

`caffeine` prevents your Mac from going to sleep. It does precisely what `/usr/bin/caffeinate` — the utility that has shipped with macOS since Mountain Lion — already does, except with a persistent menu bar icon, a live countdown, and the quiet dignity of having been compiled to native machine code by a language that required seventeen crates to link against two system frameworks.

In its defence: the built-in `caffeinate` doesn't put a ☕ emoji in your menu bar. So there's that.

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
caffeine              # starts in background — menu bar shows active icon
caffeine 30           # 30 minutes   — plain number means minutes
caffeine 2h           # 2 hours
caffeine 1h30m        # 1 hour 30 minutes
caffeine 90s          # 90 seconds, for the truly impatient
caffeine --no-display # keeps the system awake but lets the screen dim
```

When run from a terminal, caffeine detaches automatically and prints the PID:

```
Caffeine started (PID 12345)
```

From another terminal, while caffeine is running:

```bash
caffeine status       # ● Active — 1h 23m remaining
caffeine stop         # gracefully terminates the running instance
```

The menu bar icon is a minimal coffee cup that adapts to dark mode and wallpaper colour — filled when active, hollow when inactive. Click it for presets (15m / 30m / 1h / 2h / 4h / Indefinite), a Stop/Resume toggle, and a Quit option for when you've come to your senses.

---

## Upgrading

```bash
brew upgrade juliocanizalez/tap/caffeine
```

If caffeine is already running, stop it first:

```bash
caffeine stop && brew upgrade juliocanizalez/tap/caffeine
```

---

## Uninstalling

```bash
caffeine stop
brew uninstall juliocanizalez/tap/caffeine
```

---

## Releases & checksums

Each tagged release (`v*`) builds pre-compiled binaries for `aarch64-apple-darwin` and `x86_64-apple-darwin` via GitHub Actions. A `.sha256` sidecar file is uploaded alongside each tarball.

### Verifying a download

```bash
curl -LO https://github.com/juliocanizalez/caffeine/releases/download/v0.1.1/caffeine-aarch64-apple-darwin.tar.gz
curl -LO https://github.com/juliocanizalez/caffeine/releases/download/v0.1.1/caffeine-aarch64-apple-darwin.tar.gz.sha256
shasum -a 256 -c caffeine-aarch64-apple-darwin.tar.gz.sha256
```

### Bumping a release (maintainers)

1. Tag and push: `git tag v0.x.y && git push origin v0.x.y`
2. CI builds both targets and uploads tarballs + `.sha256` sidecars automatically.
3. Copy the new checksums from the release page (or the `.sha256` files).
4. Update `Formula/caffeine.rb`:
   - `version "0.x.y"`
   - `sha256` under `on_arm` — paste the `aarch64` checksum
   - `sha256` under `on_intel` — paste the `x86_64` checksum
5. Commit: `git commit Formula/caffeine.rb -m "build(release): bump formula to v0.x.y"`

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
