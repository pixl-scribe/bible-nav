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

/// The path to a decompressed copy of `resources/eng_kjv.db.gz`.
pub fn fixture_path() -> PathBuf {
    FIXTURE
        .get_or_init(|| {
            let source = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/eng_kjv.db.gz");
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
