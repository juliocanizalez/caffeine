use std::time::Duration;

/// Parse human-readable duration strings.
///
/// Supported formats:
/// - `30`      → 30 minutes (plain number = minutes)
/// - `30m`     → 30 minutes
/// - `2h`      → 2 hours
/// - `90s`     → 90 seconds
/// - `1h30m`   → 1 hour 30 minutes
/// - `inf`, `∞`, or omitted → indefinite (returns Ok(None))
pub fn parse(s: &str) -> Result<Option<Duration>, String> {
    let s = s.trim();
    if s.is_empty() || s == "inf" || s == "∞" || s == "forever" || s == "indefinite" {
        return Ok(None);
    }

    let mut total_secs: u64 = 0;
    let mut rest = s;
    let mut parsed_any = false;

    while !rest.is_empty() {
        let (n, after_num) = parse_num(rest)?;
        if after_num.is_empty() {
            // bare number → minutes
            total_secs += n * 60;
            parsed_any = true;
            break;
        }
        let (unit, after_unit) = parse_unit(after_num)?;
        total_secs += match unit {
            'h' => n * 3600,
            'm' => n * 60,
            's' => n,
            _ => unreachable!(),
        };
        parsed_any = true;
        rest = after_unit;
    }

    if !parsed_any {
        return Err(format!("cannot parse duration '{s}'"));
    }
    if total_secs == 0 {
        return Err("duration must be greater than zero".into());
    }

    Ok(Some(Duration::from_secs(total_secs)))
}

fn parse_num(s: &str) -> Result<(u64, &str), String> {
    let end = s.find(|c: char| !c.is_ascii_digit()).unwrap_or(s.len());
    if end == 0 {
        return Err(format!("expected a number, got '{s}'"));
    }
    let n: u64 = s[..end]
        .parse()
        .map_err(|e| format!("number too large: {e}"))?;
    Ok((n, &s[end..]))
}

fn parse_unit(s: &str) -> Result<(char, &str), String> {
    let mut chars = s.chars();
    match chars.next() {
        Some(c @ ('h' | 'm' | 's')) => Ok((c, chars.as_str())),
        Some(c) => Err(format!("unknown unit '{c}', expected h/m/s")),
        None => Err("expected a unit (h/m/s) after the number".into()),
    }
}

/// Format a Duration into a compact human-readable string.
/// Examples: "2h", "1h30m", "23m", "45s"
pub fn fmt(d: Duration) -> String {
    let total = d.as_secs();
    if total == 0 {
        return "0s".into();
    }
    let h = total / 3600;
    let m = (total % 3600) / 60;
    let s = total % 60;

    match (h, m, s) {
        (h, 0, 0) if h > 0 => format!("{h}h"),
        (h, m, 0) if h > 0 => format!("{h}h{m}m"),
        (0, m, 0) if m > 0 => format!("{m}m"),
        (0, m, s) if m > 0 => format!("{m}m{s}s"),
        (0, 0, s) => format!("{s}s"),
        _ => format!("{h}h{m}m"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_plain_number() {
        assert_eq!(parse("30").unwrap(), Some(Duration::from_secs(1800)));
        assert_eq!(parse("1").unwrap(), Some(Duration::from_secs(60)));
    }

    #[test]
    fn test_parse_with_units() {
        assert_eq!(parse("2h").unwrap(), Some(Duration::from_secs(7200)));
        assert_eq!(parse("30m").unwrap(), Some(Duration::from_secs(1800)));
        assert_eq!(parse("90s").unwrap(), Some(Duration::from_secs(90)));
    }

    #[test]
    fn test_parse_combined() {
        assert_eq!(parse("1h30m").unwrap(), Some(Duration::from_secs(5400)));
        assert_eq!(parse("2h15m30s").unwrap(), Some(Duration::from_secs(8130)));
    }

    #[test]
    fn test_parse_indefinite() {
        assert_eq!(parse("inf").unwrap(), None);
        assert_eq!(parse("∞").unwrap(), None);
        assert_eq!(parse("").unwrap(), None);
    }

    #[test]
    fn test_fmt() {
        assert_eq!(fmt(Duration::from_secs(7200)), "2h");
        assert_eq!(fmt(Duration::from_secs(5400)), "1h30m");
        assert_eq!(fmt(Duration::from_secs(1800)), "30m");
        assert_eq!(fmt(Duration::from_secs(90)), "1m30s");
        assert_eq!(fmt(Duration::from_secs(45)), "45s");
    }
}
