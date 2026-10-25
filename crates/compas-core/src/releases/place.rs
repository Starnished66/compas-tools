use std::fs;
use std::path::PathBuf;

use super::{Kind, Release};
use crate::card::{Card, Model};
use crate::error::{IoContext, Result};
use crate::fs::ensure_free;
use crate::net::download_verified;
use crate::progress::{OnProgress, Phase, Progress};

/// Downloads the release file for `model` and puts it where the player looks
/// for it: `.compas/compas_player`, or `<board>.upt` in the card root.
/// Returns the path written.
pub async fn place(
    card: &Card,
    release: &Release,
    kind: Kind,
    model: Model,
    on_progress: OnProgress<'_>,
) -> Result<PathBuf> {
    let file = release.file(kind, model)?;
    let dest = match kind {
        Kind::PlayerOnly => card.compas_dir().join("compas_player"),
        Kind::FullImage => card.root().join(format!("{}.upt", model.board())),
    };
    ensure_free(card.root(), file.size)?;
    download_verified(&file.url, &dest, file.size, &file.sha256, |done, total| {
        on_progress(Progress { phase: Phase::Downloading, done, total });
    })
    .await?;
    Ok(dest)
}

/// `.upt` files in the card root other than the one intended (if any). The
/// player offers whichever image it finds, so strays risk a wrong update.
pub fn other_upts(card: &Card, intended: Option<(Kind, Model)>) -> Result<Vec<String>> {
    let keep = match intended {
        Some((Kind::FullImage, model)) => Some(format!("{}.upt", model.board())),
        _ => None,
    };
    let mut found = Vec::new();
    for entry in fs::read_dir(card.root()).at(card.root())? {
        let name = entry.at(card.root())?.file_name().to_string_lossy().into_owned();
        let is_upt = name.to_ascii_lowercase().ends_with(".upt");
        let is_intended = keep.as_ref().is_some_and(|k| k.eq_ignore_ascii_case(&name));
        if is_upt && !is_intended {
            found.push(name);
        }
    }
    found.sort();
    Ok(found)
}

#[cfg(test)]
mod tests {
    use super::super::test_fixtures::{release, R1_PLAYER};
    use super::*;
    use crate::error::Error;
    use crate::test_support::{fake_card, write_file};

    #[tokio::test]
    async fn places_player_only_and_full_image() {
        let (dir, card) = fake_card();
        let (_server, release) = release("v1.2.0").await;

        let path = place(&card, &release, Kind::PlayerOnly, Model::R1, &|_| {}).await.unwrap();
        assert_eq!(path, dir.path().join(".compas/compas_player"));
        assert_eq!(fs::read(&path).unwrap(), R1_PLAYER);

        let path = place(&card, &release, Kind::FullImage, Model::R3ProII, &|_| {}).await.unwrap();
        assert_eq!(path, dir.path().join("r3proii.upt"));
        assert_eq!(fs::read(path).unwrap(), b"image r3proii");

        let path = place(&card, &release, Kind::PlayerOnly, Model::R3II2025, &|_| {}).await.unwrap();
        assert_eq!(fs::read(path).unwrap(), b"player r3ii_2025");
    }

    #[tokio::test]
    async fn bad_checksum_leaves_nothing_on_the_card() {
        let (dir, card) = fake_card();
        let (_server, mut release) = release("v1.2.0").await;
        release.checksums.insert("r1.upt".into(), "0".repeat(64));
        let err = place(&card, &release, Kind::FullImage, Model::R1, &|_| {}).await.unwrap_err();
        assert!(matches!(err, Error::Verification(_)));
        assert!(!dir.path().join("r1.upt").exists());
    }

    #[test]
    fn reports_stray_images() {
        let (dir, card) = fake_card();
        write_file(dir.path(), "r1.upt", b"");
        write_file(dir.path(), "OLD.UPT", b"");
        write_file(dir.path(), "notes.txt", b"");
        assert_eq!(
            other_upts(&card, Some((Kind::FullImage, Model::R1))).unwrap(),
            ["OLD.UPT"]
        );
        assert_eq!(other_upts(&card, Some((Kind::PlayerOnly, Model::R1))).unwrap(), ["OLD.UPT", "r1.upt"]);
        assert_eq!(other_upts(&card, None).unwrap().len(), 2);
    }
}
