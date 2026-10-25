use serde::Serialize;

use super::install::{update, Outcome};
use super::state::PluginState;
use super::{entry, Catalog};
use crate::card::Card;
use crate::error::Result;
use crate::progress::{OnProgress, Phase, Progress};

#[derive(Debug, Clone, Serialize)]
pub struct Failure {
    pub id: String,
    pub message: String,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct UpdateAllReport {
    pub updated: Vec<String>,
    /// Plugins whose update would replace a local file; update them one by one.
    pub needs_confirmation: Vec<String>,
    pub failed: Vec<Failure>,
}

const SCALE: u64 = 1000;

/// Updates every compatible plugin that has a newer version, without
/// replacing local files. One failure does not stop the others.
pub async fn update_all(
    card: &Card,
    catalog: &Catalog,
    on_progress: OnProgress<'_>,
) -> Result<UpdateAllReport> {
    let plugin_api = card.device()?.map(|d| d.plugin_api);
    let ids: Vec<String> = entry::list(catalog, card, plugin_api)?
        .into_iter()
        .filter(|e| e.state == PluginState::Update)
        .map(|e| e.id)
        .collect();

    // Each plugin gets SCALE steps; its own download and install fill them.
    let total = ids.len() as u64 * SCALE;
    let mut report = UpdateAllReport::default();
    for (i, id) in ids.into_iter().enumerate() {
        let base = i as u64 * SCALE;
        let step = |p: Progress| {
            let fraction = SCALE * p.done.min(p.total) / p.total.max(1);
            // Downloading fills the first 90% of a plugin's share, installing the rest.
            let within = match p.phase {
                Phase::Downloading => fraction * 9 / 10,
                _ => SCALE * 9 / 10 + fraction / 10,
            };
            let done = base + within;
            on_progress(Progress {
                phase: Phase::Updating,
                done,
                total,
            });
        };
        step(Progress {
            phase: Phase::Updating,
            done: 0,
            total: 1,
        });
        match update(card, catalog, &id, false, &step).await {
            Ok(Outcome::Done) => report.updated.push(id),
            Ok(Outcome::NeedsConfirmation) => report.needs_confirmation.push(id),
            Err(e) => report.failed.push(Failure {
                id,
                message: e.to_string(),
            }),
        }
    }
    on_progress(Progress {
        phase: Phase::Updating,
        done: total,
        total,
    });
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::super::install::install;
    use super::super::test_fixtures::{publish, spec};
    use super::*;
    use crate::test_support::{fake_card, write_file};

    #[tokio::test]
    async fn updates_what_it_can_and_reports_the_rest() {
        let (dir, card) = fake_card();
        let old = [
            spec("a", "1.0", &[(".plugins/A.lua", "a1", false)]),
            spec("b", "1.0", &[(".plugins/B.lua", "b1", false)]),
            spec("c", "1.0", &[(".plugins/C.lua", "c1", false)]),
            spec("d", "1.0", &[(".plugins/D.lua", "d1", false)]),
        ];
        let (_s1, c1) = publish(&old).await;
        for id in ["a", "b", "c", "d"] {
            install(&card, &c1, id, false, &|_| {}).await.unwrap();
        }
        write_file(dir.path(), ".plugins/B.lua", b"edited");

        let new = [
            spec("a", "1.1", &[(".plugins/A.lua", "a2", false)]),
            spec("b", "1.1", &[(".plugins/B.lua", "b2", false)]),
            spec("c", "1.0", &[(".plugins/C.lua", "c1", false)]),
            spec("d", "1.1", &[(".plugins/D.lua", "d2", false)]),
        ];
        let (_server, mut c2) = publish(&new).await;
        c2.plugins[3].files[0].sha256 = "f".repeat(64);

        let seen = std::sync::Mutex::new(Vec::new());
        let report = update_all(&card, &c2, &|p| seen.lock().unwrap().push(p))
            .await
            .unwrap();
        let seen = seen.into_inner().unwrap();
        assert!(seen
            .iter()
            .all(|p| p.phase == Phase::Updating && p.total == 3 * SCALE));
        assert!(
            seen.windows(2).all(|w| w[0].done <= w[1].done),
            "never goes backwards"
        );
        assert_eq!(seen.last().unwrap().done, 3 * SCALE);
        assert_eq!(report.updated, ["a"]);
        assert_eq!(report.needs_confirmation, ["b"]);
        assert_eq!(report.failed.len(), 1);
        assert_eq!(report.failed[0].id, "d");
        assert_eq!(
            std::fs::read_to_string(dir.path().join(".plugins/A.lua")).unwrap(),
            "a2"
        );
        assert_eq!(
            std::fs::read_to_string(dir.path().join(".plugins/B.lua")).unwrap(),
            "edited"
        );
    }
}
