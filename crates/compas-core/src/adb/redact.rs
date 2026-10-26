//! Removes what must not leave the computer from logs and backups: PINs,
//! tokens, passwords, Wi-Fi names and keys, accounts and network addresses.

use std::sync::LazyLock;

use regex::{Captures, Regex};

const MASK: &str = "<redacted>";

/// Words that follow "password" in ordinary sentences and are not the password.
const STOP_WORDS: [&str; 16] = [
    "for",
    "and",
    "or",
    "the",
    "a",
    "to",
    "not",
    "was",
    "incorrect",
    "required",
    "invalid",
    "expired",
    "rejected",
    "accepted",
    "mismatch",
    "too",
];

/// A quoted string, with backslash escapes (`"hun\"ter2"`).
const DOUBLE: &str = r#""(?:[^"\\\n]|\\.)*""#;
const SINGLE: &str = r#"'(?:[^'\\\n]|\\.)*'"#;

fn pattern(text: String) -> Regex {
    Regex::new(&text).expect("valid pattern")
}

/// Key names that mark a secret. A word counts only between separators, so `remote_pin`
/// matches and `mapping` does not.
const KEY_LOWER: &str = r"(?:[a-z0-9]+[_.-])*(?:pin|passw(?:or)?d|passwd|pass|pwd|passcode|passphrase|passkey|psk|sk|token|secret|api[_-]?key|apikey|authorization|auth|credentials?|session|cookie|email|user[_-]?name|username|login|host[_-]?name)(?:[_.-][a-z0-9]+)*";
/// camelCase keys such as `AccessToken`, `apiKey` or `clientSecret`.
const KEY_CAMEL: &str = r"[A-Za-z0-9_.]*[a-z0-9](?:Token|Key|Secret|Password|Passwd|Pass|Pwd|Pin|Session)[A-Za-z0-9_.]*";

/// `"key": "value"` (quoted, with escapes) or `"key": 1234` (a bare JSON value stops at the comma).
static JSON_LOWER: LazyLock<Regex> = LazyLock::new(|| {
    pattern(format!(
        r#"(?i)("{KEY_LOWER}"\s*:\s*)({DOUBLE}|{SINGLE}|[^\s,}}\]]+)"#
    ))
});
static JSON_CAMEL: LazyLock<Regex> = LazyLock::new(|| {
    pattern(format!(
        r#"("{KEY_CAMEL}"\s*:\s*)({DOUBLE}|{SINGLE}|[^\s,}}\]]+)"#
    ))
});

/// `key = "value"` with a quoted value: only the quoted part is secret.
static QUOTED_LOWER: LazyLock<Regex> = LazyLock::new(|| {
    pattern(format!(
        r"(?i)(^|[^a-z0-9])({KEY_LOWER}\s*[:=]\s*)({DOUBLE}|{SINGLE})"
    ))
});
static QUOTED_CAMEL: LazyLock<Regex> = LazyLock::new(|| {
    pattern(format!(
        r"(^|[^A-Za-z0-9_.])({KEY_CAMEL}\s*[:=]\s*)({DOUBLE}|{SINGLE})"
    ))
});

/// `key=value` lines (settings files, log lines): the secret may hold spaces and
/// punctuation, so everything to the end of the line is masked. Query strings are left to
/// `URL_QUERY`, which stops at `&`.
static LINE_LOWER: LazyLock<Regex> =
    LazyLock::new(|| pattern(format!(r"(?i)(^|[^a-z0-9])({KEY_LOWER}\s*[:=]\s*)([^\n]*)")));
static LINE_CAMEL: LazyLock<Regex> = LazyLock::new(|| {
    pattern(format!(
        r"(^|[^A-Za-z0-9_.])({KEY_CAMEL}\s*[:=]\s*)([^\n]*)"
    ))
});

/// Wi-Fi names may hold spaces, so these keys take the rest of the value:
/// `ssid=`, `bssid=`, and the player's own `wifi_last=` style keys.
static WIFI_NAME: LazyLock<Regex> = LazyLock::new(|| {
    pattern(format!(
        r#"(?i)(\b(?:b?ssid|wifi[_-]?(?:last|ssid|network|name|saved|id)[a-z0-9_.-]*|(?:last|saved)[_-]wifi[a-z0-9_.-]*)"?\s*[:=]\s*)({DOUBLE}|{SINGLE}|[^\n,;}}]*)"#
    ))
});

/// `wpa_cli ... set_network 0 psk "pass"`, `ssid "Home"`, `SSID 'Home'`.
static WPA_QUOTED: LazyLock<Regex> = LazyLock::new(|| {
    pattern(format!(
        r#"(?i)\b(psk|ssid|identity|password|passwd|wep_key\d?)(\s+)({DOUBLE}|{SINGLE})"#
    ))
});

static WPA_HEX_PSK: LazyLock<Regex> =
    LazyLock::new(|| pattern(r"(?i)\b(psk\s+)[0-9a-f]{16,}\b".into()));

/// `associated with "Home"`, `Connected to wifi "Guest"`.
static NETWORK_NAME: LazyLock<Regex> = LazyLock::new(|| {
    pattern(format!(
        r#"(?i)\b((?:associate(?:d)? with|connect(?:ed|ing)? to|join(?:ed|ing))(?:\s+(?:wi-?fi|network|ssid|ap|access point))?\s+)({DOUBLE}|{SINGLE})"#
    ))
});

/// Subsonic and similar URLs: `?u=ana&t=<token>&s=<salt>`.
static URL_QUERY: LazyLock<Regex> = LazyLock::new(|| {
    pattern(
        r#"(?i)([?&](?:u|t|s|p|user|username|pass|password|pwd|token|salt|sk|apikey|api_key|key|auth)=)[^&\s"']*"#
            .into(),
    )
});

/// "token 3f2504e0-…" with no separator.
static LOOSE_TOKEN: LazyLock<Regex> = LazyLock::new(|| {
    pattern(
        r"(?i)\b(token|secret|api[ _-]?key|passw(?:or)?d)\b([\s:=]{1,4})[A-Za-z0-9._~+/=-]{16,}"
            .into(),
    )
});

/// "remote control code 482913", "PIN is 123456".
static LOOSE_PIN: LazyLock<Regex> = LazyLock::new(|| {
    pattern(
        r"(?i)((?:\bremote[ _-]?control\s+(?:pin|code|passcode)|\bpairing\s+(?:pin|code|passkey)|\b(?:pin|passkey|passcode))\b\D{0,20}?)\d(?:[ -]?\d){3,9}\b".into(),
    )
});

/// `nmcli dev wifi connect "Casa Garita"`: the network name follows the verb.
static NMCLI: LazyLock<Regex> = LazyLock::new(|| {
    pattern(format!(
        r#"(?i)(\bnmcli\b[^\n]*?\b(?:connect|up|add)\s+)({DOUBLE}|{SINGLE}|[^\s"']+)"#
    ))
});

/// `<password>x</password>`, `<token>x</token>`, `<key name="psk">x</key>`.
static XML_SECRET: LazyLock<Regex> = LazyLock::new(|| {
    pattern(format!(
        r#"(?i)(<(?:{KEY_LOWER}|key\s+name\s*=\s*["']{KEY_LOWER}["'])(?:\s[^>]*)?>)[^<]+(</)"#
    ))
});

/// A bare `Basic <base64>` credential, with or without its header name.
static BASIC_CREDENTIAL: LazyLock<Regex> =
    LazyLock::new(|| pattern(r"\b(Basic\s+)[A-Za-z0-9+/]{12,}={0,2}".into()));

/// `curl -u user:pass` and `curl --user user:pass`.
static CURL_USER: LazyLock<Regex> = LazyLock::new(|| {
    pattern(format!(
        r#"(\bcurl\b[^\n]*?\s(?:-u|--user)(?:\s+|=))({DOUBLE}|{SINGLE}|\S+)"#
    ))
});

/// `--password x` and `password x`: the value follows after a space, and may be
/// introduced by "is". Plain prose such as "password is wrong" is masked too; masking too
/// much is safer than too little.
static SPACED_PASSWORD: LazyLock<Regex> = LazyLock::new(|| {
    pattern(format!(
        r#"(?i)\b(passw(?:or)?d|passwd|pwd|passphrase)(\s+(?:is\s+)?)({DOUBLE}|{SINGLE}|[^\s"']+)"#
    ))
});

/// `wpa_passphrase <ssid> <passphrase>`: both words are private.
static WPA_PASSPHRASE_COMMAND: LazyLock<Regex> = LazyLock::new(|| {
    pattern(format!(
        r#"(?i)(\bwpa_passphrase\s+)({DOUBLE}|{SINGLE}|\S+)(\s+)({DOUBLE}|{SINGLE}|\S+)"#
    ))
});

static BEARER: LazyLock<Regex> =
    LazyLock::new(|| pattern(r"(?i)(Bearer\s+)[A-Za-z0-9._~+/=-]{8,}".into()));

static URL_CREDENTIALS: LazyLock<Regex> =
    LazyLock::new(|| pattern(r"(://)[^/\s:@]+:[^/\s@]+@".into()));

static EMAIL: LazyLock<Regex> = LazyLock::new(|| {
    pattern(r"[A-Za-z0-9._%+-]+@[A-Za-z0-9-]+(?:\.[A-Za-z0-9-]+)*\.[A-Za-z]{2,}".into())
});

static MAC: LazyLock<Regex> =
    LazyLock::new(|| pattern(r"\b(?:[0-9A-Fa-f]{2}[:-]){5}[0-9A-Fa-f]{2}\b".into()));

static IPV4: LazyLock<Regex> = LazyLock::new(|| {
    pattern(r"\b(?:(?:25[0-5]|2[0-4]\d|1?\d?\d)\.){3}(?:25[0-5]|2[0-4]\d|1?\d?\d)\b".into())
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

    for json in [&JSON_LOWER, &JSON_CAMEL] {
        apply(json, &|c| format!("{}{MASK}", &c[1]));
    }
    for quoted in [&QUOTED_LOWER, &QUOTED_CAMEL] {
        apply(quoted, &|c| format!("{}{}{MASK}", &c[1], &c[2]));
    }
    for line in [&LINE_LOWER, &LINE_CAMEL] {
        apply(line, &|c| {
            let in_query = matches!(&c[1], "?" | "&");
            if in_query || c[3].trim().is_empty() || c[3].starts_with(MASK) {
                c[0].to_string()
            } else {
                format!("{}{}{MASK}", &c[1], &c[2])
            }
        });
    }
    apply(&WIFI_NAME, &|c| format!("{}{MASK}", &c[1]));
    apply(&WPA_QUOTED, &|c| format!("{}{}{MASK}", &c[1], &c[2]));
    apply(&WPA_HEX_PSK, &|c| format!("{}{MASK}", &c[1]));
    apply(&NETWORK_NAME, &|c| format!("{}{MASK}", &c[1]));
    apply(&NMCLI, &|c| format!("{}{MASK}", &c[1]));
    apply(&URL_QUERY, &|c| format!("{}{MASK}", &c[1]));
    apply(&LOOSE_TOKEN, &|c| format!("{}{}{MASK}", &c[1], &c[2]));
    apply(&LOOSE_PIN, &|c| format!("{}{MASK}", &c[1]));
    apply(&XML_SECRET, &|c| format!("{}{MASK}{}", &c[1], &c[2]));
    apply(&BASIC_CREDENTIAL, &|c| {
        // "Basic configuration" is prose; a credential holds a digit or a base64 symbol.
        if c[0].contains(|ch: char| ch.is_ascii_digit() || matches!(ch, '+' | '/' | '=')) {
            format!("{}{MASK}", &c[1])
        } else {
            c[0].to_string()
        }
    });
    apply(&CURL_USER, &|c| format!("{}{MASK}", &c[1]));
    apply(&SPACED_PASSWORD, &|c| {
        let introduced = c[2].trim() == "is";
        if c[3].starts_with(MASK)
            || (!introduced && STOP_WORDS.contains(&c[3].to_ascii_lowercase().as_str()))
        {
            c[0].to_string()
        } else {
            format!("{}{}{MASK}", &c[1], &c[2])
        }
    });
    apply(&WPA_PASSPHRASE_COMMAND, &|c| {
        format!("{}{MASK}{}{MASK}", &c[1], &c[3])
    });
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
mod tests;
