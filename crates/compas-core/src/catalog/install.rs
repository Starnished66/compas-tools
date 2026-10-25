use std::fs;

use serde::Serialize;

use super::commit;
use super::index::PluginFile;
use super::record::{Record, MAX_RECORDED};
use super::state::is_incompatible;
use super::Catalog;
use crate::card::Card;
use crate::error::{Error, Result};
use crate::fs::{ensure_free, StageDir};
use crate::net::download_verified;
use crate::progress::{OnProgress, Phase, Progress};
use crate::version;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Done,
    /// A local file would be replaced; call again with `force` once the user agrees.
    NeedsConfirmation,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Install,
    Update,
}

pub async fn install(
    card: &Card,
    catalog: &Catalog,
    id: &str,
    force: bool,
    on_progress: OnProgress<'_>,
) -> Result<Outcome> {
    apply(Mode::Install, card, catalog, id, force, on_progress).await
}

pub async fn update(
    card: &Card,
    catalog: &Catalog,
    id: &str,
    force: bool,
    on_progress: OnProgress<'_>,
) -> Result<Outcome> {
    apply(Mode::Update, card, catalog, id, force, on_progress).await
}

async fn apply(
    mode: Mode,
    card: &Card,
    catalog: &Catalog,
    id: &str,
    force: bool,
    on_progress: OnProgress<'_>,
) -> Result<Outcome> {
    let plugin = catalog.get(id)?;
    if is_incompatible(plugin, card.device()?.map(|d| d.plugin_api)) {
        return Err(Error::Rejected(
            "this plugin needs a newer player version".into(),
        ));
    }
    let record = Record::read(card)?;
    let installed = record.get(id);
    match (mode, installed) {
        (Mode::Install, Some(_)) if !force => {
            return Err(Error::Rejected("this plugin is already installed".into()))
        }
        (Mode::Update, Some(old)) if version::compare(&plugin.version, &old.version).is_gt() => {}
        (Mode::Update, _) => return Err(Error::Rejected("this plugin has no update".into())),
        (Mode::Install, None) if record.plugins.len() >= MAX_RECORDED => {
            return Err(Error::Rejected("too many plugins are installed".into()))
        }
        _ => {}
    }
    if !force && commit::conflict(card, plugin, &record)? {
        return Ok(Outcome::NeedsConfirmation);
    }

    let wanted: Vec<(usize, &PluginFile)> = plugin
        .files
        .iter()
        .enumerate()
        .filter(|(_, f)| !(f.keep && fs::symlink_metadata(card.root().join(&f.dest)).is_ok()))
        .collect();
    let total: u64 = wanted.iter().map(|(_, f)| f.size).sum();
    ensure_free(card.root(), total)?;

    let stage = StageDir::create(&card.compas_dir().join("store"), &plugin.id)?;
    let mut finished = 0;
    for (i, file) in wanted {
        let part = stage.path().join(format!("{i}.part"));
        let base = finished;
        download_verified(
            &catalog.asset_url(&file.asset),
            &part,
            file.size,
            &file.sha256,
            |done, _| {
                on_progress(Progress {
                    phase: Phase::Downloading,
                    done: base + done,
                    total,
                });
            },
        )
        .await?;
        finished += file.size;
    }

    on_progress(Progress {
        phase: Phase::Installing,
        done: 0,
        total: 1,
    });
    let outcome = commit::apply(card, stage, plugin, force, &record)?;
    on_progress(Progress {
        phase: Phase::Installing,
        done: 1,
        total: 1,
    });
    Ok(outcome)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::sync::Mutex;

    use super::super::test_fixtures::{publish, spec};
    use super::*;
    use crate::test_support::{fake_card, write_file};

    const NOOP: OnProgress<'static> = &|_| {};

    fn read(dir: &tempfile::TempDir, rel: &str) -> String {
        fs::read_to_string(dir.path().join(rel)).unwrap()
    }

    #[tokio::test]
    async fn installs_files_records_them_and_cleans_the_stage() {
        let (dir, card) = fake_card();
        let s = spec(
            "p",
            "1.0",
            &[
                (".plugins/P.lua", "lua", false),
                ("Data/p/d.txt", "data", false),
            ],
        );
        let (_server, catalog) = publish(&[s]).await;
        let seen = Mutex::new(Vec::new());
        let outcome = install(&card, &catalog, "p", false, &|p| {
            seen.lock().unwrap().push(p)
        })
        .await
        .unwrap();

        assert_eq!(outcome, Outcome::Done);
        assert_eq!(read(&dir, ".plugins/P.lua"), "lua");
        assert_eq!(read(&dir, "Data/p/d.txt"), "data");
        let record = Record::read(&card).unwrap();
        assert_eq!(record.get("p").unwrap().files.len(), 2);
        assert!(!dir.path().join(".compas/store").exists(), "stage removed");
        let seen = seen.lock().unwrap();
        assert!(seen
            .iter()
            .any(|p| p.phase == Phase::Downloading && p.done == 7 && p.total == 7));
        assert_eq!(seen.last().unwrap().phase, Phase::Installing);
    }

    #[tokio::test]
    async fn local_file_needs_confirmation_and_force_replaces_it() {
        let (dir, card) = fake_card();
        write_file(dir.path(), ".plugins/P.lua", b"mine");
        let (_server, catalog) =
            publish(&[spec("p", "1.0", &[(".plugins/P.lua", "store", false)])]).await;

        assert_eq!(
            install(&card, &catalog, "p", false, NOOP).await.unwrap(),
            Outcome::NeedsConfirmation
        );
        assert_eq!(read(&dir, ".plugins/P.lua"), "mine");
        assert!(Record::read(&card).unwrap().plugins.is_empty());

        assert_eq!(
            install(&card, &catalog, "p", true, NOOP).await.unwrap(),
            Outcome::Done
        );
        assert_eq!(read(&dir, ".plugins/P.lua"), "store");
    }

    #[tokio::test]
    async fn rejects_double_install_and_unneeded_update_and_incompatible() {
        let (dir, card) = fake_card();
        let (_server, catalog) =
            publish(&[spec("p", "1.0", &[(".plugins/P.lua", "a", false)])]).await;
        install(&card, &catalog, "p", false, NOOP).await.unwrap();
        assert!(matches!(
            install(&card, &catalog, "p", false, NOOP).await,
            Err(Error::Rejected(_))
        ));
        assert!(matches!(
            update(&card, &catalog, "p", false, NOOP).await,
            Err(Error::Rejected(_))
        ));
        assert!(matches!(
            install(&card, &catalog, "nope", false, NOOP).await,
            Err(Error::NotFound(_))
        ));

        write_file(
            dir.path(),
            ".compas/device.json",
            br#"{"schema":1,"model":"r1","player_version":"1.0.0","plugin_api":5}"#,
        );
        let (_s2, catalog2) = publish(&[spec("q", "1.0", &[(".plugins/Q.lua", "q", false)])]).await;
        assert!(matches!(
            install(&card, &catalog2, "q", false, NOOP).await,
            Err(Error::Rejected(_))
        ));
    }

    #[tokio::test]
    async fn update_replaces_files_and_removes_dropped_ones_but_not_edited_ones() {
        let (dir, card) = fake_card();
        let v1 = spec(
            "p",
            "1.0",
            &[
                (".plugins/P.lua", "v1", false),
                ("Data/old.txt", "old", false),
                ("Data/edited.txt", "orig", false),
                ("Data/kept.txt", "k1", true),
            ],
        );
        let (_s1, c1) = publish(&[v1]).await;
        install(&card, &c1, "p", false, NOOP).await.unwrap();
        write_file(dir.path(), "Data/edited.txt", b"user edit");
        write_file(dir.path(), "Data/kept.txt", b"user kept");

        let v2 = spec(
            "p",
            "1.1",
            &[
                (".plugins/P.lua", "v2", false),
                ("Data/kept.txt", "k2", true),
            ],
        );
        let (_s2, c2) = publish(&[v2]).await;
        assert_eq!(
            update(&card, &c2, "p", false, NOOP).await.unwrap(),
            Outcome::Done
        );

        assert_eq!(read(&dir, ".plugins/P.lua"), "v2");
        assert!(!dir.path().join("Data/old.txt").exists());
        assert_eq!(read(&dir, "Data/edited.txt"), "user edit");
        assert_eq!(read(&dir, "Data/kept.txt"), "user kept");
        assert_eq!(
            Record::read(&card).unwrap().get("p").unwrap().version,
            "1.1"
        );
    }

    #[tokio::test]
    async fn failed_download_leaves_the_card_untouched() {
        let (dir, card) = fake_card();
        let s = spec(
            "p",
            "1.0",
            &[
                (".plugins/P.lua", "lua", false),
                ("Data/p.txt", "data", false),
            ],
        );
        let (_server, mut catalog) = publish(&[s]).await;
        // Second asset now points at a hash the server cannot satisfy.
        catalog.plugins[0].files[1].sha256 = "0".repeat(64);

        let err = install(&card, &catalog, "p", false, NOOP)
            .await
            .unwrap_err();
        assert!(matches!(err, Error::Verification(_)));
        assert!(!dir.path().join(".plugins/P.lua").exists());
        assert!(!dir.path().join(".compas/store").exists());
        assert!(Record::read(&card).unwrap().plugins.is_empty());
    }

    #[tokio::test]
    async fn record_failure_rolls_back_replaced_files() {
        let (dir, card) = fake_card();
        let (_server, catalog) =
            publish(&[spec("p", "1.0", &[(".plugins/P.lua", "new", false)])]).await;
        install(&card, &catalog, "p", false, NOOP).await.unwrap();
        let (_s2, c2) = publish(&[spec("p", "1.1", &[(".plugins/P.lua", "newer", false)])]).await;

        // A directory where the record file belongs makes the record write fail.
        fs::remove_file(dir.path().join(".plugins/.store_installed")).unwrap();
        fs::create_dir(dir.path().join(".plugins/.store_installed")).unwrap();
        write_file(dir.path(), ".plugins/P.lua", b"new");
        let result = install(&card, &c2, "p", true, NOOP).await;
        assert!(result.is_err());
        assert_eq!(read(&dir, ".plugins/P.lua"), "new");
    }
}
