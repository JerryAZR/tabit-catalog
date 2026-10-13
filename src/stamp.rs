//! Version + checksum stamping: the artifact's identity for the
//! download channel.

use sha2::{Digest as _, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};

/// The stamp: `<utc-date>-<sha256-prefix>`.
///
/// The date prefix is for humans; the content-hash suffix makes the
/// version content-unique — the same version always means identical
/// artifact bytes (the revision-monotonicity lesson from ZCode's
/// channel). URL-safe, and stable across reruns within a day.
pub fn stamp(artifact: &str) -> (String, String) {
    let sha256 = hex(&Sha256::digest(artifact.as_bytes()));
    let version = format!("{}-{}", utc_date(), &sha256[..10]);
    (version, sha256)
}

/// The `latest.toml` pointer body — exactly the three fields the
/// consumer's `deny_unknown_fields` struct accepts.
pub fn latest_toml(version: &str, sha256: &str) -> String {
    format!(
        "version = \"{version}\"\nsha256 = \"{sha256}\"\nmin_schema = {}\n",
        crate::MIN_SCHEMA
    )
}

fn hex(digest: &[u8]) -> String {
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// UTC `yyyymmdd` from the system clock. The date comes from
/// [`civil_from_days`] — Howard Hinnant's standard algorithm,
/// hand-rolled only to keep a date crate out of a stamp; tested
/// against known dates below.
fn utc_date() -> String {
    let days = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() / 86_400)
        .unwrap_or(0) as i64;
    let (year, month, day) = civil_from_days(days);
    format!("{year:04}{month:02}{day:02}")
}

/// Days since 1970-01-01 → (year, month, day), proleptic Gregorian.
/// Howard Hinnant's `civil_from_days` (chrono's own lineage).
pub fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let year = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if month <= 2 { year + 1 } else { year }, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_dates_round_trip() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(11_016), (2000, 2, 29)); // leap day
        assert_eq!(civil_from_days(20_738), (2026, 10, 12));
        assert_eq!(civil_from_days(-1), (1969, 12, 31)); // pre-epoch
    }

    #[test]
    fn stamp_is_content_addressed() {
        let (version, sha) = stamp("body");
        assert_eq!(&sha[..10], version.rsplit('-').next().unwrap());
        assert_eq!(sha.len(), 64);
        let (other_version, _other_sha) = stamp("body");
        assert_eq!(version, other_version);
        assert_ne!(sha, stamp("different body").1);
    }

    #[test]
    fn latest_toml_shape() {
        let body = latest_toml("20261012-abcdef0123", &"f".repeat(64));
        assert_eq!(
            body,
            format!(
                "version = \"20261012-abcdef0123\"\nsha256 = \"{}\"\nmin_schema = 1\n",
                "f".repeat(64)
            )
        );
    }
}
