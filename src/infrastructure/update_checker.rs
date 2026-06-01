/// Checks the GitHub releases API for a version newer than the current build.
/// Returns `Some("vX.Y.Z")` if a newer release exists, `None` otherwise (or on error).
pub fn check_for_update() -> Option<String> {
    let response =
        ureq::get("https://api.github.com/repos/juliocanizalez/caffeine/releases/latest")
            .set(
                "User-Agent",
                concat!("caffeine/", env!("CARGO_PKG_VERSION")),
            )
            .call()
            .ok()?;

    let json: serde_json::Value = response.into_json().ok()?;
    let tag = json.get("tag_name")?.as_str()?;

    let latest = tag.trim_start_matches('v');
    let current = env!("CARGO_PKG_VERSION");

    if semver_gt(latest, current) {
        Some(tag.to_string())
    } else {
        None
    }
}

fn semver_gt(a: &str, b: &str) -> bool {
    let parse = |v: &str| -> (u32, u32, u32) {
        let mut p = v.split('.');
        let major = p.next().and_then(|s| s.parse().ok()).unwrap_or(0);
        let minor = p.next().and_then(|s| s.parse().ok()).unwrap_or(0);
        let patch = p.next().and_then(|s| s.parse().ok()).unwrap_or(0);
        (major, minor, patch)
    };
    parse(a) > parse(b)
}
