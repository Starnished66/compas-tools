//! Removes what must not leave the computer from logs and backups: PINs,
//! tokens, passwords, Wi-Fi names and keys, accounts and network addresses.

use std::sync::LazyLock;

use regex::{Captures, Regex};

const MASK: &str = "<redacted>";

/// A quoted string, with backslash escapes (`"hun\"ter2"`).
const DOUBLE: &str = r#""(?:[^"\\\n]|\\.)*""#;
const SINGLE: &str = r#"'(?:[^'\\\n]|\\.)*'"#;

fn pattern(text: String) -> Regex {
    Regex::new(&text).expect("valid pattern")
}

/// Key names that mark a secret. A word counts only between separators, so `remote_pin`
/// matches and `mapping` does not.
const KEY_LOWER: &str = r"(?:[a-z0-9]+[_.-])*(?:pin|passw(?:or)?d|passwd|pass|pwd|passcode|passphrase|passkey|psk|sk|token|secret|api[_-]?key|apikey|authorization|auth|credentials?|session|cookie|email|user[_-]?name|username|login)(?:[_.-][a-z0-9]+)*";
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
        r"(?i)((?:\bremote[ _-]?control\s+(?:pin|code|passcode)|\bpairing\s+(?:pin|code|passkey)|\b(?:pin|passkey|passcode))\b\D{0,20}?)\d{4,8}\b".into(),
    )
});

/// `nmcli dev wifi connect "Casa Garita"`: the network name follows the verb.
static NMCLI: LazyLock<Regex> = LazyLock::new(|| {
    pattern(format!(
        r#"(?i)(\bnmcli\b[^\n]*?\b(?:connect|up|add)\s+)({DOUBLE}|{SINGLE}|[^\s"']+)"#
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
    fn wpa_cli_commands_and_supplicant_messages_are_masked() {
        assert_eq!(
            masked(r#"wpa_cli -i wlan0 set_network 0 psk "MyWifiPass123""#),
            "wpa_cli -i wlan0 set_network 0 psk <redacted>"
        );
        assert_eq!(
            masked(r#"wpa_cli -i wlan0 set_network 0 ssid "Casa Garita 5G""#),
            "wpa_cli -i wlan0 set_network 0 ssid <redacted>"
        );
        assert_eq!(
            masked("Trying to associate with SSID 'Casa Garita'"),
            "Trying to associate with SSID <redacted>"
        );
        assert_eq!(
            masked(r#"wlan0: associated with "CasaGarita""#),
            "wlan0: associated with <redacted>"
        );
        assert_eq!(
            masked(r#"Connected to wifi "Starbucks Guest""#),
            "Connected to wifi <redacted>"
        );
        assert_eq!(
            masked("set_network 0 psk 0123456789abcdef0123456789abcdef"),
            "set_network 0 psk <redacted>"
        );
    }

    #[test]
    fn the_players_own_wifi_keys_take_the_whole_name() {
        assert_eq!(
            masked("wifi_last=Casa Garita 5G\nvolume=3"),
            "wifi_last=<redacted>\nvolume=3"
        );
        assert_eq!(masked("wifi_ssid = \"Home Net\""), "wifi_ssid = <redacted>");
        assert_eq!(
            masked("wifi_enabled=1\nwifi_status=connected"),
            "wifi_enabled=1\nwifi_status=connected"
        );
    }

    #[test]
    fn subsonic_urls_keep_no_user_token_or_salt() {
        let url = "GET /rest/ping.view?u=ana&t=0123abcdef&s=salt99&v=1.16.1&c=compas";
        let out = masked(url);
        assert_eq!(
            out,
            "GET /rest/ping.view?u=<redacted>&t=<redacted>&s=<redacted>&v=1.16.1&c=compas"
        );
        assert_eq!(
            masked("stream.view?id=5&p=enc:70617373"),
            "stream.view?id=5&p=<redacted>"
        );
    }

    #[test]
    fn tokens_in_json_and_camel_case_keys_are_masked() {
        assert_eq!(
            masked(r#"{"AccessToken":"abc.def.ghi","UserId":"7"}"#),
            r#"{"AccessToken":<redacted>,"UserId":"7"}"#
        );
        assert_eq!(masked("refreshToken=zzz111"), "refreshToken=<redacted>");
        assert_eq!(masked(r#""apiKey": "k-123""#), r#""apiKey": <redacted>"#);
        assert_eq!(masked("clientSecret: s3cr3t"), "clientSecret: <redacted>");
        assert_eq!(
            masked("sk=1234\npass=abc\npwd=xyz"),
            "sk=<redacted>\npass=<redacted>\npwd=<redacted>"
        );
    }

    #[test]
    fn line_based_values_are_masked_to_the_end_of_the_line() {
        assert_eq!(
            masked("subsonic_password=pa ss word\nvolume=3"),
            "subsonic_password=<redacted>\nvolume=3"
        );
        assert_eq!(
            masked("subsonic_username=Maria Lopez"),
            "subsonic_username=<redacted>"
        );
        assert_eq!(
            masked("subsonic_password=p;a,s}s"),
            "subsonic_password=<redacted>"
        );
        assert_eq!(
            masked("wpa_passphrase=Hunter Two"),
            "wpa_passphrase=<redacted>"
        );
        // Quoted and JSON values keep the shorter cut.
        assert_eq!(
            masked(r#"password="p a;ss" name=x"#),
            r#"password=<redacted> name=x"#
        );
        assert_eq!(
            masked(r#"{"password": 12345, "name": "x"}"#),
            r#"{"password": <redacted>, "name": "x"}"#
        );
        // A query string stops at the next parameter.
        assert_eq!(
            masked("GET /x?token=abc123&v=1"),
            "GET /x?token=<redacted>&v=1"
        );
    }

    #[test]
    fn authorization_headers_are_masked_whatever_the_scheme() {
        for header in [
            "Authorization: Basic YW5hOmh1bnRlcjI=",
            "Authorization: Bearer abcdefgh12345",
            "Authorization: Digest username=\"ana\", realm=\"x\", response=\"abc\"",
            "authorization=Negotiate YIIB",
        ] {
            let out = masked(header);
            assert!(out.ends_with("<redacted>"), "{out}");
            assert!(
                !out.contains("YW5h")
                    && !out.contains("abcdefgh")
                    && !out.contains("YIIB")
                    && !out.contains("response="),
                "{out}"
            );
        }
        assert_eq!(
            masked("Authorization: Basic YW5hOmh1bnRlcjI="),
            "Authorization: <redacted>"
        );
    }

    #[test]
    fn bluetooth_passkeys_and_nmcli_networks_are_masked() {
        assert_eq!(
            masked("Bluetooth pairing passkey 123456"),
            "Bluetooth pairing passkey <redacted>"
        );
        assert_eq!(
            masked("pairing code 654321 shown"),
            "pairing code <redacted> shown"
        );
        assert_eq!(
            masked(r#"nmcli dev wifi connect "Casa Garita" password "x y""#),
            "nmcli dev wifi connect <redacted> password <redacted>"
        );
        assert_eq!(
            masked("nmcli connection up CasaGarita"),
            "nmcli connection up <redacted>"
        );
    }

    #[test]
    fn a_uuid_after_the_word_token_and_remote_codes_are_masked() {
        assert_eq!(
            masked("using token 3f2504e0-4f89-11d3-9a0c-0305e82c3301 for sync"),
            "using token <redacted> for sync"
        );
        assert_eq!(
            masked("Remote control code 482913"),
            "Remote control code <redacted>"
        );
        assert_eq!(
            masked("remote control pin: 1234"),
            "remote control pin: <redacted>"
        );
    }

    #[test]
    fn escaped_quotes_inside_a_value_do_not_end_it() {
        let out = masked(r#"{"password":"hun\"ter2","name":"x"}"#);
        assert_eq!(out, r#"{"password":<redacted>,"name":"x"}"#);
        assert!(!out.contains("ter2"));
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
