//! Shared test fixture: the bundled KJV module, decompressed once into a
//! temporary folder that's deleted when the tests finish.

use std::fs::File;
use std::io::copy;
use std::path::PathBuf;
use std::sync::OnceLock;

use flate2::read::GzDecoder;
use tempfile::TempDir;

use super::db::ModuleDb;

static FIXTURE: OnceLock<(TempDir, PathBuf)> = OnceLock::new();

/// The bundled KJV module, `resources/eng_kjv-<version>.db.gz`.
pub fn bundled_kjv_gz() -> PathBuf {
    let resources = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources");
    std::fs::read_dir(&resources)
        .expect("resources folder")
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .find(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("eng_kjv-") && name.ends_with(".db.gz"))
        })
        .expect("a bundled eng_kjv-<version>.db.gz")
}

/// The path to a decompressed copy of the bundled KJV module.
pub fn fixture_path() -> PathBuf {
    FIXTURE
        .get_or_init(|| {
            let source = bundled_kjv_gz();
            let dir = tempfile::tempdir().expect("temporary folder");
            let path = dir.path().join("eng_kjv.db");
            let mut decoder = GzDecoder::new(File::open(source).expect("bundled module"));
            copy(
                &mut decoder,
                &mut File::create(&path).expect("fixture file"),
            )
            .expect("decompress");
            (dir, path)
        })
        .1
        .clone()
}

/// The fixture, opened through the same checks the app uses.
pub fn kjv() -> ModuleDb {
    ModuleDb::open(&fixture_path()).expect("the bundled KJV should open")
}
