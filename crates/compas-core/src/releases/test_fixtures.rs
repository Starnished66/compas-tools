//! A fake GitHub releases API for tests.

use serde_json::json;

use super::{fetch_latest_from, Release};
use crate::test_support::{sha256_hex, TestServer};

/// Content of `compas_player` (the R1 build), for assertions.
pub const R1_PLAYER: &[u8] = b"player r1";

pub async fn release(tag: &str) -> (TestServer, Release) {
    release_with(tag, true).await
}

/// `player_only: false` is a firmware-only release: just the `<board>.upt` images.
pub async fn release_with(tag: &str, player_only: bool) -> (TestServer, Release) {
    let mut files: Vec<(String, Vec<u8>)> = Vec::new();
    if player_only {
        files.push(("compas_player".into(), R1_PLAYER.to_vec()));
    }
    for board in ["r1", "r3proii", "r3ii_2025"] {
        if board != "r1" && player_only {
            files.push((
                format!("compas_player_{board}"),
                format!("player {board}").into_bytes(),
            ));
        }
        files.push((
            format!("{board}.upt"),
            format!("image {board}").into_bytes(),
        ));
    }
    let sums: String = files
        .iter()
        .map(|(name, body)| format!("{}  {name}\n", sha256_hex(body)))
        .collect();
    let tag = tag.to_string();
    let server = TestServer::start_with(|base| {
        let mut assets: Vec<_> = files
            .iter()
            .map(|(name, body)| json!({ "name": name, "size": body.len(), "browser_download_url": format!("{base}/dl/{name}") }))
            .collect();
        assets.push(json!({ "name": "SHA256SUMS", "size": sums.len(), "browser_download_url": format!("{base}/dl/SHA256SUMS") }));
        let api = json!({
            "tag_name": tag, "name": "Compás", "body": "Notes", "published_at": "2026-10-17T17:45:54Z",
            "html_url": "https://example.test/release", "assets": assets
        });
        let mut routes = vec![
            ("/repos/Starnished66/compas-player/releases/latest".to_string(), api.to_string().into_bytes()),
            ("/dl/SHA256SUMS".to_string(), sums.clone().into_bytes()),
        ];
        routes.extend(files.iter().map(|(n, b)| (format!("/dl/{n}"), b.clone())));
        routes
    })
    .await;
    let release = fetch_latest_from(server.base()).await.unwrap();
    (server, release)
}
