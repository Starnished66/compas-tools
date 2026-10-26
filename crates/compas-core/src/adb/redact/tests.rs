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
    let text = "mapping=3\nplayer_version=1.2.0\nvolume=12\nshipping 5 pins\nfree 1.2G of 3.7G\n";
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

#[test]
fn xml_secrets_are_masked_between_their_tags() {
    assert_eq!(
        masked("<account><password>hunter2</password></account>"),
        "<account><password><redacted></password></account>"
    );
    assert_eq!(
        masked(r#"<key name="psk">MyWifiPass123</key>"#),
        r#"<key name="psk"><redacted></key>"#
    );
    assert_eq!(masked("<name>Ana</name>"), "<name>Ana</name>");
}

#[test]
fn a_bare_basic_credential_is_masked_but_prose_is_not() {
    assert_eq!(
        masked("sent Basic YW5hOmh1bnRlcjI= to host"),
        "sent Basic <redacted> to host"
    );
    assert_eq!(
        masked("Basic configuration loaded"),
        "Basic configuration loaded"
    );
}

#[test]
fn curl_credentials_are_masked() {
    assert_eq!(
        masked("curl -u ana:hunter2 https://host/x"),
        "curl -u <redacted> https://host/x"
    );
    assert_eq!(
        masked(r#"curl -s --user "ana:hun ter" https://host/x"#),
        "curl -s --user <redacted> https://host/x"
    );
}

#[test]
fn passwords_after_a_space_are_masked() {
    assert_eq!(
        masked("login --password hunter2 --quiet"),
        "login --password <redacted> --quiet"
    );
    assert_eq!(
        masked("password hunter2 saved"),
        "password <redacted> saved"
    );
    assert_eq!(
        masked("the password is hunter2"),
        "the password is <redacted>"
    );
    assert_eq!(
        masked("password for ana rejected"),
        "password for ana rejected"
    );
}

#[test]
fn the_wpa_passphrase_command_masks_the_name_and_the_passphrase() {
    assert_eq!(
        masked("wpa_passphrase CasaGarita hunter2hunter2"),
        "wpa_passphrase <redacted> <redacted>"
    );
    assert_eq!(
        masked(r#"wpa_passphrase "Casa Garita" "my pass""#),
        "wpa_passphrase <redacted> <redacted>"
    );
}

#[test]
fn pairing_codes_with_a_dash_are_masked() {
    assert_eq!(
        masked("pairing code 482-913 shown"),
        "pairing code <redacted> shown"
    );
    assert_eq!(masked("pairing code 482 913"), "pairing code <redacted>");
}

#[test]
fn hostnames_are_masked() {
    assert_eq!(masked("hostname: casa-garita-r1"), "hostname: <redacted>");
    assert_eq!(
        masked("hostname=r1\nvolume=3"),
        "hostname=<redacted>\nvolume=3"
    );
}
