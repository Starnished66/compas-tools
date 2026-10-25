//! Helpers for unit tests: a local HTTP server and a card laid out in a temp dir.

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::Arc;

use sha2::{Digest, Sha256};
use tempfile::TempDir;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use crate::card::Card;

pub fn sha256_hex(data: &[u8]) -> String {
    format!("{:x}", Sha256::digest(data))
}

/// Serves fixed bodies by request path; anything else is a 404.
pub struct TestServer {
    base: String,
}

impl TestServer {
    pub async fn start<P: Into<String>>(routes: impl IntoIterator<Item = (P, Vec<u8>)>) -> Self {
        let routes: Vec<_> = routes.into_iter().map(|(p, b)| (p.into(), b)).collect();
        Self::start_with(|_| routes).await
    }

    /// For bodies that must contain the server's own address.
    pub async fn start_with(routes: impl FnOnce(&str) -> Vec<(String, Vec<u8>)>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let routes: Arc<HashMap<String, Vec<u8>>> = Arc::new(routes(&base).into_iter().collect());
        tokio::spawn(async move {
            loop {
                let Ok((mut socket, _)) = listener.accept().await else {
                    return;
                };
                let routes = routes.clone();
                tokio::spawn(async move {
                    let mut request = Vec::new();
                    let mut buf = [0u8; 1024];
                    while !request.windows(4).any(|w| w == b"\r\n\r\n") {
                        match socket.read(&mut buf).await {
                            Ok(0) | Err(_) => return,
                            Ok(n) => request.extend_from_slice(&buf[..n]),
                        }
                    }
                    let text = String::from_utf8_lossy(&request);
                    let path = text.split_whitespace().nth(1).unwrap_or("/");
                    let (status, body) = match routes.get(path) {
                        Some(body) => ("200 OK", body.as_slice()),
                        None => ("404 Not Found", &b"missing"[..]),
                    };
                    let head = format!(
                        "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        body.len()
                    );
                    let _ = socket.write_all(head.as_bytes()).await;
                    let _ = socket.write_all(body).await;
                    let _ = socket.shutdown().await;
                });
            }
        });
        Self { base }
    }

    pub fn base(&self) -> &str {
        &self.base
    }

    pub fn url(&self, path: &str) -> String {
        format!("{}{path}", self.base)
    }
}

/// A temp directory with `.compas/` and `.plugins/`, opened as a card.
pub fn fake_card() -> (TempDir, Card) {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join(".compas")).unwrap();
    fs::create_dir_all(dir.path().join(".plugins")).unwrap();
    let card = Card::open(dir.path()).unwrap();
    (dir, card)
}

pub fn write_file(root: &Path, rel: &str, data: &[u8]) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, data).unwrap();
}

/// A stand-in `compas-sdtool`: a shell script whose body decides what each
/// command (`$1`) prints. Every call is appended to `calls.log` next to it.
#[cfg(unix)]
pub fn script_tool(dir: &Path, body: &str) -> crate::sdtool::SdTool {
    use std::os::unix::fs::PermissionsExt;

    let path = dir.join("fake-sdtool");
    let log = dir.join("calls.log");
    let script = format!("#!/bin/sh\necho \"$1\" >> '{}'\n{body}\n", log.display());
    fs::write(&path, script).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    crate::sdtool::SdTool::new(path)
}

/// Commands the fake tool was called with, in order.
#[cfg(unix)]
pub fn calls(dir: &Path) -> Vec<String> {
    fs::read_to_string(dir.join("calls.log"))
        .map(|text| text.lines().map(str::to_string).collect())
        .unwrap_or_default()
}
