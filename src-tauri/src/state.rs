use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use compas_core::card::Card;
use compas_core::catalog::Catalog;
use compas_core::releases::Release;
use tokio::sync::Mutex as AsyncMutex;

use crate::error::{CommandError, CommandResult};

const RELEASE_TTL: Duration = Duration::from_secs(10 * 60);

/// The selected card, plus the last fetched catalog and release so commands
/// act on exactly what the user was shown.
#[derive(Default)]
pub struct AppState {
    card: Mutex<Option<Card>>,
    catalog: Mutex<Option<Arc<Catalog>>>,
    release: Mutex<Option<(Instant, Arc<Release>)>>,
    /// Held while a command changes the card, so two never overlap.
    busy: AsyncMutex<()>,
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

impl AppState {
    pub fn card(&self) -> CommandResult<Card> {
        lock(&self.card)
            .clone()
            .ok_or_else(|| CommandError::new("no_card", "No card selected."))
    }

    pub fn selected_card(&self) -> Option<Card> {
        lock(&self.card).clone()
    }

    pub fn set_card(&self, card: Option<Card>) {
        *lock(&self.card) = card;
    }

    pub fn catalog(&self) -> CommandResult<Arc<Catalog>> {
        lock(&self.catalog)
            .clone()
            .ok_or_else(|| CommandError::new("no_catalog", "The plugin list is not loaded."))
    }

    pub fn set_catalog(&self, catalog: Arc<Catalog>) {
        *lock(&self.catalog) = Some(catalog);
    }

    /// The release shown to the user, whatever its age.
    pub fn release(&self) -> CommandResult<Arc<Release>> {
        lock(&self.release)
            .clone()
            .map(|(_, release)| release)
            .ok_or_else(|| CommandError::new("no_release", "No player release is loaded."))
    }

    /// The cached release, if it was fetched recently; GitHub limits
    /// anonymous requests to 60 an hour.
    pub fn recent_release(&self) -> Option<Arc<Release>> {
        let cached = lock(&self.release);
        let (fetched, release) = cached.as_ref()?;
        (fetched.elapsed() < RELEASE_TTL).then(|| release.clone())
    }

    pub fn set_release(&self, release: Arc<Release>) {
        *lock(&self.release) = Some((Instant::now(), release));
    }

    pub fn try_busy(&self) -> CommandResult<tokio::sync::MutexGuard<'_, ()>> {
        self.busy
            .try_lock()
            .map_err(|_| CommandError::new("busy", "Another operation is still running."))
    }
}
