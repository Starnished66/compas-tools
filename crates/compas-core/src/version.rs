//! Dotted numeric versions as the player compares them (`store_version_compare`).

use std::cmp::Ordering;

/// Drops the leading `v`/`V` that release tags and `device.json` may carry (`v1.3.0`).
pub fn strip_v(version: &str) -> &str {
    let version = version.trim();
    version.strip_prefix(['v', 'V']).unwrap_or(version)
}

/// One to four numeric groups, under 32 characters (`store_version_valid`).
pub fn is_valid(version: &str) -> bool {
    let groups: Vec<&str> = version.split('.').collect();
    version.len() < 32
        && groups.len() <= 4
        && groups
            .iter()
            .all(|g| !g.is_empty() && g.bytes().all(|b| b.is_ascii_digit()))
}

/// Compares group by group; a missing group counts as zero.
pub fn compare(a: &str, b: &str) -> Ordering {
    let mut a = a.split('.');
    let mut b = b.split('.');
    loop {
        match (a.next(), b.next()) {
            (None, None) => return Ordering::Equal,
            (x, y) => {
                let ord = compare_group(x.unwrap_or(""), y.unwrap_or(""));
                if ord != Ordering::Equal {
                    return ord;
                }
            }
        }
    }
}

fn compare_group(a: &str, b: &str) -> Ordering {
    let a = a.trim_start_matches('0');
    let b = b.trim_start_matches('0');
    a.len().cmp(&b.len()).then_with(|| a.cmp(b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compares_numerically_not_lexically() {
        assert_eq!(compare("1.10", "1.9"), Ordering::Greater);
        assert_eq!(compare("1.2", "1.2.0"), Ordering::Equal);
        assert_eq!(compare("1.2", "1.2.1"), Ordering::Less);
        assert_eq!(compare("01.002", "1.2"), Ordering::Equal);
        assert_eq!(
            compare("99999999999999999999", "100000000000000000000"),
            Ordering::Less
        );
    }

    #[test]
    fn strips_one_leading_v_only() {
        assert_eq!(strip_v("v1.3.0"), "1.3.0");
        assert_eq!(strip_v("V1.3.0"), "1.3.0");
        assert_eq!(strip_v(" 1.3.0 "), "1.3.0");
        assert!(!is_valid(strip_v("vv1.3.0")));
    }

    #[test]
    fn validates_shape() {
        assert!(is_valid("1.2.3.4"));
        assert!(!is_valid("1.2.3.4.5"));
        assert!(!is_valid("1..2"));
        assert!(!is_valid("1.2a"));
        assert!(!is_valid(""));
    }
}
