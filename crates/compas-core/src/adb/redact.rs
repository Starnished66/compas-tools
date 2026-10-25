//! Removes what must not leave the computer from logs and backups: PINs,
//! tokens, passwords, Wi-Fi names and keys, accounts and network addresses.

use std::sync::LazyLock;

use regex::{Captures, Regex};

const MASK: &str = "<redacted>";

/// `key = value` and `"key": "value"` where the key names a secret. A key word
/// counts only between separators, so `remote_pin` matches and `mapping` does not.
static SECRET_VALUE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?i)(^|[^a-z0-9])((?:[a-z0-9]+[_.-])*(?:pin|passw(?:or)?d|passwd|psk|token|secret|api[_-]?key|apikey|authorization|auth|credentials?|session|cookie|email|user[_-]?name|username|login)(?:[_.-][a-z0-9]+)*)("?\s*[:=]\s*)("[^"\n]*"|'[^'\n]*'|(?:Bearer\s+)?[^\s,;}\]"']+)"#,
    )
    .expect("valid pattern")
});

/// Wi-Fi names may hold spaces, so they run to the end of the value.
static SSID: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?i)(\bb?ssid\b"?\s*[:=]\s*)("[^"\n]*"|[^\n,;}]*)"#).expect("valid pattern")
});

static LOOSE_PIN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)(\bpin\b\D{0,20}?)\d{4,8}\b").expect("valid pattern"));

static BEARER: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)(Bearer\s+)[A-Za-z0-9._~+/=-]{8,}").expect("valid pattern"));

static URL_CREDENTIALS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(://)[^/\s:@]+:[^/\s@]+@").expect("valid pattern"));

static EMAIL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"[A-Za-z0-9._%+-]+@[A-Za-z0-9-]+(?:\.[A-Za-z0-9-]+)*\.[A-Za-z]{2,}")
        .expect("valid pattern")
});

static MAC: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b(?:[0-9A-Fa-f]{2}[:-]){5}[0-9A-Fa-f]{2}\b").expect("valid pattern")
});

static IPV4: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b(?:(?:25[0-5]|2[0-4]\d|1?\d?\d)\.){3}(?:25[0-5]|2[0-4]\d|1?\d?\d)\b")
        .expect("valid pattern")
});

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Redacted {
    pub text: String,
    /// How many places were masked.
    pub count: usize,
}

pub fn redact(input: &str) -> Redacted {
    let mut count = 0;
    let mut text = input.to_string();
    let mut apply = |regex: &Regex, replace: &dyn Fn(&Captures) -> String| {
        text = regex
            .replace_all(&text, |caps: &Captures| {
                let replacement = replace(caps);
                if replacement != caps[0] {
                    count += 1;
                }
                replacement
            })
            .into_owned();
    };

    apply(&SECRET_VALUE, &|c| {
        format!("{}{}{}{MASK}", &c[1], &c[2], &c[3])
    });
    apply(&SSID, &|c| format!("{}{MASK}", &c[1]));
    apply(&LOOSE_PIN, &|c| format!("{}{MASK}", &c[1]));
    apply(&BEARER, &|c| format!("{}{MASK}", &c[1]));
    apply(&URL_CREDENTIALS, &|c| format!("{}{MASK}@", &c[1]));
    apply(&EMAIL, &|_| "<email>".into());
    apply(&MAC, &|_| "xx:xx:xx:xx:xx:xx".into());
    apply(&IPV4, &|c| {
        if c[0].starts_with("127.") || &c[0] == "0.0.0.0" {
            c[0].to_string()
        } else {
            "<ip>".into()
        }
    });
    Redacted { text, count }
}

/// Whether a file or folder name suggests secrets (Wi-Fi settings, PINs,
/// tokens); such entries are left out of a backup altogether.
pub fn is_secret_name(name: &str) -> bool {
    const WORDS: [&str; 14] = [
        "pin",
        "pins",
        "wpa",
        "wifi",
        "psk",
        "password",
        "passwd",
        "token",
        "tokens",
        "secret",
        "credential",
        "credentials",
        "pairing",
        "auth",
    ];
    name.to_ascii_lowercase()
        .split(|c: char| !c.is_ascii_alphanumeric())
        .any(|word| WORDS.contains(&word))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn masked(input: &str) -> String {
        redact(input).text
    }

    #[test]
    fn secrets_in_key_value_forms_are_masked() {
        assert_eq!(masked("pin=1234"), "pin=<redacted>");
        assert_eq!(
            masked("remote_control_pin = 998877"),
            "remote_control_pin = <redacted>"
        );
        assert_eq!(
            masked(r#"{"pin": "4321", "volume": 12}"#),
            r#"{"pin": <redacted>, "volume": 12}"#
        );
        assert_eq!(masked("psk=hunter2"), "psk=<redacted>");
        assert_eq!(masked("password: 'p w'"), "password: <redacted>");
        assert_eq!(
            masked("lastfm.session_key=abc123"),
            "lastfm.session_key=<redacted>"
        );
        assert_eq!(masked("wifi_passwd=xyz"), "wifi_passwd=<redacted>");
        assert_eq!(
            masked("Authorization: Bearer abcdefgh12345"),
            "Authorization: <redacted>"
        );
    }

    #[test]
    fn wifi_names_keep_no_trace() {
        assert_eq!(masked(r#"ssid="My Home Wi-Fi""#), r#"ssid=<redacted>"#);
        assert_eq!(masked("SSID: Coffee Shop Guest"), "SSID: <redacted>");
        assert_eq!(masked("bssid=aa:bb:cc:dd:ee:ff"), "bssid=<redacted>");
    }

    #[test]
    fn loose_mentions_and_embedded_values_are_masked() {
        assert_eq!(
            masked("remote control PIN is 123456 now"),
            "remote control PIN is <redacted> now"
        );
        assert_eq!(
            masked("Bearer abcdefgh12345 sent"),
            "Bearer <redacted> sent"
        );
        assert_eq!(
            masked("GET https://bob:s3cret@host/x"),
            "GET https://<redacted>@host/x"
        );
        assert_eq!(masked("mail ana@example.com done"), "mail <email> done");
        assert_eq!(
            masked("wlan0 aa:bb:cc:dd:ee:ff up"),
            "wlan0 xx:xx:xx:xx:xx:xx up"
        );
        assert_eq!(
            masked("lease 192.168.1.23 on lo 127.0.0.1"),
            "lease <ip> on lo 127.0.0.1"
        );
    }

    #[test]
    fn ordinary_diagnostics_survive() {
        let text =
            "mapping=3\nplayer_version=1.2.0\nvolume=12\nshipping 5 pins\nfree 1.2G of 3.7G\n";
        let out = redact(text);
        assert_eq!(out.text, text);
        assert_eq!(out.count, 0);
    }

    #[test]
    fn counts_every_masked_place() {
        assert_eq!(redact("pin=1\ntoken=2\nssid=x\n").count, 3);
    }

    #[test]
    fn secret_looking_names_are_recognised() {
        for name in [
            "wpa_supplicant.conf",
            "wifi",
            "remote_pin",
            "PIN",
            "api-token.json",
            "pairing.db",
            "auth.json",
        ] {
            assert!(is_secret_name(name), "{name}");
        }
        for name in [
            "settings.ini",
            "mapping.json",
            "pinned_albums",
            "player_crash.log",
            "spinner.png",
        ] {
            assert!(!is_secret_name(name), "{name}");
        }
    }
}
