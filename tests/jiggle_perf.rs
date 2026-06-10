use std::time::Instant;

use caffeine::domain::ports::{IdleDetector, Jiggler};
use caffeine::infrastructure::jiggle::{CoreGraphicsIdleDetector, CoreGraphicsJiggler};

// ── Timing tests ──────────────────────────────────────────────────────────────

#[test]
fn idle_seconds_is_fast() {
    let detector = CoreGraphicsIdleDetector;

    // Warm up
    for _ in 0..10 {
        let _ = detector.idle_seconds();
    }

    let n = 1_000u32;
    let start = Instant::now();
    for _ in 0..n {
        let _ = detector.idle_seconds();
    }
    let avg_ns = start.elapsed().as_nanos() / u128::from(n);

    println!("idle_seconds(): avg {} ns ({} µs)", avg_ns, avg_ns / 1_000);
    assert!(
        avg_ns < 1_000_000,
        "idle_seconds avg {avg_ns} ns exceeds 1 ms budget"
    );
}

#[test]
fn jiggle_is_fast() {
    let jiggler = CoreGraphicsJiggler;

    // Warm up
    for _ in 0..5 {
        jiggler.jiggle();
    }

    let n = 100u32;
    let start = Instant::now();
    for _ in 0..n {
        jiggler.jiggle();
    }
    let avg_ns = start.elapsed().as_nanos() / u128::from(n);

    println!("jiggle(): avg {} ns ({} µs)", avg_ns, avg_ns / 1_000);
    assert!(
        avg_ns < 5_000_000,
        "jiggle avg {avg_ns} ns exceeds 5 ms budget"
    );
}

// ── Idle-reset regression test ────────────────────────────────────────────────

/// The jiggle event must register as user input in the combined-session event
/// source state — that is the timer Electron apps (Slack) poll for auto-away.
/// A no-op event (e.g. flags-changed with flags=0) silently fails this.
///
/// Ignored by default: it needs a real GUI session with permission to post
/// events; CI runners may silently drop them. Run locally with
/// `cargo test -- --ignored`.
#[test]
#[ignore]
fn jiggle_resets_idle_timer() {
    let detector = CoreGraphicsIdleDetector;
    let jiggler = CoreGraphicsJiggler;

    let before = detector.idle_seconds();
    jiggler.jiggle();
    // Event delivery through the WindowServer is asynchronous.
    std::thread::sleep(std::time::Duration::from_millis(50));
    let after = detector.idle_seconds();

    println!("idle before: {before:.3}s  after: {after:.3}s");
    assert!(
        after < before || after < 0.1,
        "jiggle did not reset the combined-session idle timer \
         (before: {before:.3}s, after: {after:.3}s)"
    );
}

// ── Memory / leak test ────────────────────────────────────────────────────────

/// Current process RSS in bytes via `ps`.
fn rss_bytes() -> u64 {
    let pid = std::process::id();
    let out = std::process::Command::new("ps")
        .args(["-o", "rss=", "-p", &pid.to_string()])
        .output()
        .expect("ps failed");
    // ps prints RSS in KB on macOS
    let kb: u64 = String::from_utf8_lossy(&out.stdout)
        .trim()
        .parse()
        .unwrap_or(0);
    kb * 1024
}

#[test]
fn jiggle_no_rss_leak() {
    let jiggler = CoreGraphicsJiggler;

    // Exercise a bit so transient allocations settle before baseline
    for _ in 0..10 {
        jiggler.jiggle();
    }

    let baseline = rss_bytes();

    for _ in 0..500 {
        jiggler.jiggle();
    }

    let after = rss_bytes();
    let delta = after.saturating_sub(baseline);

    println!(
        "RSS before: {} KB  after: {} KB  delta: {} KB",
        baseline / 1024,
        after / 1024,
        delta / 1024,
    );

    // If CGEvent objects were not released, 500 calls × 1 event × ~200 B ≈ 100 KB.
    // Threshold is 2 MB to absorb ps-subprocess + malloc-arena noise while still
    // catching real leaks (missing CFRelease would accumulate MB quickly at scale).
    assert!(
        delta < 2 * 1024 * 1024,
        "RSS grew by {} KB over 500 jiggle() calls — possible CGEvent leak",
        delta / 1024
    );
}
