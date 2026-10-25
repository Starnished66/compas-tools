//! Builds catalogs and a server hosting their assets for tests.

use serde_json::json;

use super::{Catalog, Plugin, PluginFile, Preview};
use crate::test_support::{sha256_hex, TestServer};

pub const TAG: &str = "v1";

pub struct Spec {
    pub plugin: Plugin,
    bodies: Vec<Vec<u8>>,
    preview_body: Option<Vec<u8>>,
}

impl Spec {
    pub fn with_preview(mut self, image: &[u8]) -> Self {
        self.plugin.preview = Some(Preview {
            asset: format!("{}-preview", self.plugin.id),
            sha256: sha256_hex(image),
            size: image.len() as u64,
            width: 10,
            height: 10,
            variants: Vec::new(),
        });
        self.preview_body = Some(image.to_vec());
        self
    }
}

/// A plugin whose files are `(dest, content, keep)`; needs one `.plugins/*.lua`.
pub fn spec(id: &str, version: &str, files: &[(&str, &str, bool)]) -> Spec {
    let plugin_files: Vec<PluginFile> = files
        .iter()
        .enumerate()
        .map(|(i, (dest, content, keep))| PluginFile {
            asset: format!("{id}-{i}"),
            dest: (*dest).into(),
            sha256: sha256_hex(content.as_bytes()),
            size: content.len() as u64,
            keep: *keep,
        })
        .collect();
    Spec {
        bodies: files.iter().map(|f| f.1.as_bytes().to_vec()).collect(),
        preview_body: None,
        plugin: Plugin {
            id: id.into(),
            name: id.into(),
            version: version.into(),
            api_min: 10,
            description: String::new(),
            category: String::new(),
            author: String::new(),
            size: plugin_files.iter().map(|f| f.size).sum(),
            preview: None,
            files: plugin_files,
        },
    }
}

/// Serves the specs' assets and returns the catalog that lists them.
pub async fn publish(specs: &[Spec]) -> (TestServer, Catalog) {
    let mut routes = Vec::new();
    for s in specs {
        for (f, body) in s.plugin.files.iter().zip(&s.bodies) {
            routes.push((format!("/{TAG}/{}", f.asset), body.clone()));
        }
        if let (Some(p), Some(body)) = (&s.plugin.preview, &s.preview_body) {
            routes.push((format!("/{TAG}/{}", p.asset), body.clone()));
        }
    }
    let server = TestServer::start(routes).await;
    let plugins: Vec<&Plugin> = specs.iter().map(|s| &s.plugin).collect();
    let index = json!({ "schema": 1, "tag": TAG, "plugins": plugins });
    let catalog = Catalog::parse(index.to_string().as_bytes(), server.base()).unwrap();
    (server, catalog)
}
