//! Talks to the real plugin catalog on GitHub.
//! Run with: cargo test -p compas-core -- --ignored

use compas_core::card::Card;
use compas_core::catalog::{self, Catalog, Outcome, PluginState};

#[tokio::test]
#[ignore = "needs network access"]
async fn installs_the_smallest_real_plugin_into_a_temp_card() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join(".compas")).unwrap();
    let card = Card::open(dir.path()).unwrap();

    let catalog = Catalog::fetch().await.unwrap();
    assert!(!catalog.plugins.is_empty());
    let smallest = catalog.plugins.iter().min_by_key(|p| p.size).unwrap();
    println!("installing {} {} ({} bytes)", smallest.id, smallest.version, smallest.size);

    let outcome = catalog::install(&card, &catalog, &smallest.id, false, &|_| {}).await.unwrap();
    assert_eq!(outcome, Outcome::Done);
    let script = smallest.script().unwrap();
    assert!(dir.path().join(".plugins").join(script).is_file());

    let rows = catalog::list(&catalog, &card, None).unwrap();
    let row = rows.iter().find(|r| r.id == smallest.id).unwrap();
    assert_eq!(row.state, PluginState::Installed);

    catalog::set_enabled(&card, script, false).unwrap();
    catalog::uninstall(&card, &smallest.id, &|_| {}).unwrap();
    assert!(!dir.path().join(".plugins").join(script).exists());
}
