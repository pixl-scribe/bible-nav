//! The modules folder on disk: choosing it, installing the bundled module,
//! finding modules, and moving them to a new folder.
//!
//! Modules live in a folder people can see, such as Documents/BibleNav, so
//! they can find, back up, and share them. This file only works with the
//! paths it's given, so it can be tested with temporary folders. `folder.rs`
//! connects it to the app.

use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};

use flate2::read::GzDecoder;

use super::db::ModuleDb;
use super::error::{ModuleError, Result};
use super::queries;

/// The folder name used inside Documents (or the home folder).
pub const DEFAULT_FOLDER_NAME: &str = "BibleNav";
/// A folder with this name next to the app marks a portable copy.
pub const PORTABLE_FOLDER_NAME: &str = "modules";

const README_NAME: &str = "README.txt";
const README_TEXT: &str = "\
Bible Nav modules
=================

The .db files in this folder are Bible modules for Bible Nav
(https://github.com/pixl-scribe/bible-nav). Each one holds a Bible text in
an ordinary SQLite database, named by module and version: for example,
eng_kjv-1.0.0.db is the King James Version (1769).

You're welcome to copy and share these files, to keep God's word in many
hands. Bible Nav opens them read-only and never changes them.

Bible Nav checks each module's format when it opens it. Checking that a
module's text hasn't been altered is planned for a future version.
";

/// Where modules live, and whether this is a portable copy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FolderChoice {
    pub path: PathBuf,
    /// A `modules` folder next to the app, as on a thumb drive. Portable
    /// folders may be read-only, so the app never writes to them.
    pub portable: bool,
}

/// Decides where modules live, in this order:
/// 1. a `modules` folder next to the app (portable and thumb-drive copies);
/// 2. the folder chosen earlier;
/// 3. a `BibleNav` folder in Documents, or in the home folder if there's no
///    Documents folder.
///
/// Returns `None` only if none of these exist.
pub fn choose_folder(
    app_dir: Option<&Path>,
    saved: Option<PathBuf>,
    documents: Option<PathBuf>,
    home: Option<PathBuf>,
) -> Option<FolderChoice> {
    if let Some(portable) = app_dir.map(|dir| dir.join(PORTABLE_FOLDER_NAME)) {
        if portable.is_dir() {
            return Some(FolderChoice {
                path: portable,
                portable: true,
            });
        }
    }
    saved
        .or_else(|| documents.or(home).map(|dir| dir.join(DEFAULT_FOLDER_NAME)))
        .map(|path| FolderChoice {
            path,
            portable: false,
        })
}

/// A module found in the folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FoundModule {
    pub id: String,
    pub version: String,
    pub name: String,
    pub path: PathBuf,
}

/// What `install_bundled` did.
#[derive(Debug, PartialEq, Eq)]
pub enum InstallOutcome {
    /// The module was copied into the folder.
    Installed(PathBuf),
    /// A valid copy of this module and version was already there, so it was
    /// left alone.
    AlreadyPresent(PathBuf),
    /// A file with this name was there but failed the checks. It was renamed
    /// to `set_aside`, not deleted, and the bundled module installed.
    ReplacedInvalid { path: PathBuf, set_aside: PathBuf },
}

/// Installs one bundled module, a gzip file named `<id>-<version>.db.gz`,
/// into the folder as `<id>-<version>.db`, uncompressed, so people can open
/// it with ordinary SQLite tools. Never overwrites a valid existing copy.
///
/// The version is in the bundled file's name, so startup can see a copy is
/// already installed without unpacking anything. When it does install, it
/// checks that the name matches the module's own id and version.
pub fn install_bundled(bundled_gz: &Path, folder: &Path) -> Result<InstallOutcome> {
    let file_name = bundled_target_name(bundled_gz)?;
    let target = folder.join(&file_name);
    if target.exists() && ModuleDb::open(&target).is_ok() {
        return Ok(InstallOutcome::AlreadyPresent(target));
    }

    fs::create_dir_all(folder)?;
    // Unpack to a hidden temporary file, then rename it into place, so a
    // half-written module never appears under a real name.
    let temp = folder.join(".installing-module.tmp");
    let result = unpack_and_place(bundled_gz, &temp, &target, &file_name);
    if temp.exists() {
        let _ = fs::remove_file(&temp);
    }
    result
}

fn unpack_and_place(
    bundled_gz: &Path,
    temp: &Path,
    target: &Path,
    file_name: &str,
) -> Result<InstallOutcome> {
    let mut decoder = GzDecoder::new(File::open(bundled_gz)?);
    io::copy(&mut decoder, &mut File::create(temp)?)?;

    // The block ends here, so the module is closed before it's renamed.
    {
        let db = ModuleDb::open(temp)?;
        let actual = module_file_name(&queries::module_info(db.conn())?)?;
        if actual != file_name {
            return Err(ModuleError::Unsupported(format!(
                "the bundled file {file_name}.gz holds {actual}"
            )));
        }
    }

    // The target was checked above: if it exists, it failed to open.
    if target.exists() {
        let set_aside = target.with_extension("db.rejected");
        fs::rename(target, &set_aside)?;
        fs::rename(temp, target)?;
        return Ok(InstallOutcome::ReplacedInvalid {
            path: target.to_path_buf(),
            set_aside,
        });
    }
    fs::rename(temp, target)?;
    Ok(InstallOutcome::Installed(target.to_path_buf()))
}

/// `eng_kjv-1.0.0.db` from a bundled file named `eng_kjv-1.0.0.db.gz`.
fn bundled_target_name(bundled_gz: &Path) -> Result<String> {
    let name = bundled_gz
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    name.strip_suffix(".gz")
        .filter(|stem| stem.ends_with(".db") && stem.contains('-'))
        .map(String::from)
        .ok_or_else(|| {
            ModuleError::Unsupported(format!(
                "the bundled file {name} isn't named <id>-<version>.db.gz"
            ))
        })
}

/// Installs every bundled module (`*.db.gz`) found in `resources`. Each
/// file's result is returned separately, so one bad file doesn't stop the
/// others.
pub fn install_all_bundled(
    resources: &Path,
    folder: &Path,
) -> Result<Vec<(PathBuf, Result<InstallOutcome>)>> {
    let mut bundled = Vec::new();
    for entry in fs::read_dir(resources)? {
        let path = entry?.path();
        let is_module = path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.ends_with(".db.gz"));
        if is_module {
            bundled.push(path);
        }
    }
    bundled.sort();
    Ok(bundled
        .into_iter()
        .map(|path| {
            let result = install_bundled(&path, folder);
            (path, result)
        })
        .collect())
}

/// `<id>-<version>.db`, from the module's own metadata.
///
/// The id and version come from inside the module, so they're checked to
/// contain only safe characters. Otherwise a crafted module could name
/// itself `../../something` and be written outside the folder.
fn module_file_name(info: &queries::ModuleInfo) -> Result<String> {
    let id = info
        .fetch_id
        .as_deref()
        .ok_or_else(|| ModuleError::Unsupported("the module has no id (fetchId)".into()))?;
    for (label, value) in [("id", id), ("version", info.version.as_str())] {
        let safe = !value.is_empty()
            && !value.starts_with('.')
            && value
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'));
        if !safe {
            return Err(ModuleError::Unsupported(format!(
                "the module's {label} \"{value}\" can't be used in a file name"
            )));
        }
    }
    Ok(format!("{id}-{}.db", info.version))
}

/// Writes `README.txt` if the folder doesn't have one, leaving any existing
/// one (perhaps edited or translated by its owner) alone.
pub fn write_readme(folder: &Path) -> Result<()> {
    let path = folder.join(README_NAME);
    if !path.exists() {
        fs::write(path, README_TEXT)?;
    }
    Ok(())
}

/// A file in the folder that isn't a usable module, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Skipped {
    pub path: PathBuf,
    pub reason: String,
}

/// Finds the modules in a folder. When one module id has several versions,
/// only the newest is returned. Files that aren't valid modules are listed
/// in `skipped` rather than causing an error.
pub fn scan(folder: &Path) -> Result<(Vec<FoundModule>, Vec<Skipped>)> {
    let mut newest: BTreeMap<String, FoundModule> = BTreeMap::new();
    let mut skipped = Vec::new();

    for path in module_files(folder)? {
        let found = ModuleDb::open(&path).and_then(|db| {
            let info = queries::module_info(db.conn())?;
            let id = info
                .fetch_id
                .ok_or_else(|| ModuleError::Unsupported("the module has no id (fetchId)".into()))?;
            Ok(FoundModule {
                id,
                version: info.version,
                name: info.name,
                path: path.clone(),
            })
        });
        match found {
            Ok(module) => {
                let is_newer = newest.get(&module.id).is_none_or(|current| {
                    compare_versions(&module.version, &current.version) == Ordering::Greater
                });
                if is_newer {
                    newest.insert(module.id.clone(), module);
                }
            }
            Err(error) => skipped.push(Skipped {
                path,
                reason: error.to_string(),
            }),
        }
    }
    Ok((newest.into_values().collect(), skipped))
}

/// The `.db` files directly in a folder, ignoring hidden files, sorted by name.
fn module_files(folder: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for entry in fs::read_dir(folder)? {
        let path = entry?.path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        let is_db = path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("db"));
        if is_db && !name.starts_with('.') && path.is_file() {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}

/// Compares versions like "1.10.0" and "1.9.2" by their numeric parts, so
/// 1.10 is newer than 1.9. Parts that aren't numbers compare as text.
pub fn compare_versions(a: &str, b: &str) -> Ordering {
    let mut left = a.split('.');
    let mut right = b.split('.');
    loop {
        match (left.next(), right.next()) {
            (None, None) => return Ordering::Equal,
            (Some(_), None) => return Ordering::Greater,
            (None, Some(_)) => return Ordering::Less,
            (Some(x), Some(y)) => {
                let order = match (x.parse::<u64>(), y.parse::<u64>()) {
                    (Ok(x), Ok(y)) => x.cmp(&y),
                    _ => x.cmp(y),
                };
                if order != Ordering::Equal {
                    return order;
                }
            }
        }
    }
}

/// What `move_modules` did, by file name.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct MoveReport {
    pub moved: Vec<String>,
    /// Modules left in the old folder, because the new folder already had a
    /// file with that name, or the copy failed its checks.
    pub left_behind: Vec<String>,
}

/// Moves every valid module from one folder to another. A module is only
/// removed from the old folder once its new copy has opened successfully.
/// Files that aren't modules, and the README, are left where they are.
pub fn move_modules(from: &Path, to: &Path) -> Result<MoveReport> {
    let mut report = MoveReport::default();
    if !from.is_dir() || same_folder(from, to) {
        return Ok(report);
    }
    fs::create_dir_all(to)?;

    for source in module_files(from)? {
        if ModuleDb::open(&source).is_err() {
            continue;
        }
        let name = source
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let target = to.join(&name);
        if target.exists() {
            report.left_behind.push(name);
            continue;
        }
        // Renaming is instant on the same drive. Across drives it fails, so
        // copy instead, and check the copy before removing the original.
        if fs::rename(&source, &target).is_ok() {
            report.moved.push(name);
        } else if fs::copy(&source, &target).is_ok() && ModuleDb::open(&target).is_ok() {
            fs::remove_file(&source)?;
            report.moved.push(name);
        } else {
            let _ = fs::remove_file(&target);
            report.left_behind.push(name);
        }
    }
    Ok(report)
}

fn same_folder(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => a == b,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::test_support::{bundled_kjv_gz, fixture_path};
    use std::io::Write;

    fn bundled_gz() -> PathBuf {
        bundled_kjv_gz()
    }

    fn kjv_file_name() -> String {
        let db = ModuleDb::open(&fixture_path()).unwrap();
        let info = queries::module_info(db.conn()).unwrap();
        format!("eng_kjv-{}.db", info.version)
    }

    #[test]
    fn prefers_a_portable_folder_next_to_the_app() {
        let app = tempfile::tempdir().unwrap();
        fs::create_dir(app.path().join("modules")).unwrap();
        let choice = choose_folder(
            Some(app.path()),
            Some("/saved".into()),
            Some("/docs".into()),
            None,
        )
        .unwrap();
        assert_eq!(choice.path, app.path().join("modules"));
        assert!(choice.portable);
    }

    #[test]
    fn then_the_saved_folder_then_documents_then_home() {
        let app = tempfile::tempdir().unwrap();
        let pick = |saved: Option<&str>, docs: Option<&str>, home: Option<&str>| {
            choose_folder(
                Some(app.path()),
                saved.map(Into::into),
                docs.map(Into::into),
                home.map(Into::into),
            )
        };
        let saved = pick(Some("/saved"), Some("/docs"), Some("/home")).unwrap();
        assert_eq!(
            (saved.path, saved.portable),
            (PathBuf::from("/saved"), false)
        );
        assert_eq!(
            pick(None, Some("/docs"), Some("/home")).unwrap().path,
            PathBuf::from("/docs/BibleNav")
        );
        assert_eq!(
            pick(None, None, Some("/home")).unwrap().path,
            PathBuf::from("/home/BibleNav")
        );
        assert_eq!(pick(None, None, None), None);
    }

    #[test]
    fn installs_the_bundled_module_with_its_version_in_the_name() {
        let folder = tempfile::tempdir().unwrap();
        let outcome = install_bundled(&bundled_gz(), folder.path()).unwrap();
        let expected = folder.path().join(kjv_file_name());
        assert_eq!(outcome, InstallOutcome::Installed(expected.clone()));
        assert!(ModuleDb::open(&expected).is_ok());
        assert!(!folder.path().join(".installing-module.tmp").exists());
    }

    #[test]
    fn never_overwrites_a_valid_copy() {
        let folder = tempfile::tempdir().unwrap();
        install_bundled(&bundled_gz(), folder.path()).unwrap();
        let path = folder.path().join(kjv_file_name());
        let modified = fs::metadata(&path).unwrap().modified().unwrap();

        let outcome = install_bundled(&bundled_gz(), folder.path()).unwrap();
        assert_eq!(outcome, InstallOutcome::AlreadyPresent(path.clone()));
        assert_eq!(fs::metadata(&path).unwrap().modified().unwrap(), modified);
    }

    #[test]
    fn sets_aside_an_invalid_copy_instead_of_deleting_it() {
        let folder = tempfile::tempdir().unwrap();
        let path = folder.path().join(kjv_file_name());
        fs::write(&path, "damaged").unwrap();

        let outcome = install_bundled(&bundled_gz(), folder.path()).unwrap();
        let set_aside = path.with_extension("db.rejected");
        assert_eq!(
            outcome,
            InstallOutcome::ReplacedInvalid {
                path: path.clone(),
                set_aside: set_aside.clone()
            }
        );
        assert_eq!(fs::read_to_string(set_aside).unwrap(), "damaged");
        assert!(ModuleDb::open(&path).is_ok());
    }

    #[test]
    fn skips_unpacking_when_a_valid_copy_exists() {
        let folder = tempfile::tempdir().unwrap();
        fs::copy(fixture_path(), folder.path().join(kjv_file_name())).unwrap();
        // A bundled file with the same name but unreadable contents: if the
        // installer tried to unpack it, this would fail.
        let bundled = tempfile::tempdir().unwrap();
        let fake = bundled.path().join(format!("{}.gz", kjv_file_name()));
        fs::write(&fake, "not gzip").unwrap();

        let outcome = install_bundled(&fake, folder.path()).unwrap();
        assert_eq!(
            outcome,
            InstallOutcome::AlreadyPresent(folder.path().join(kjv_file_name()))
        );
    }

    #[test]
    fn refuses_a_bundled_file_whose_name_does_not_match_its_contents() {
        let bundled = tempfile::tempdir().unwrap();
        let misnamed = bundled.path().join("eng_kjv-9.9.9.db.gz");
        fs::copy(bundled_gz(), &misnamed).unwrap();
        let folder = tempfile::tempdir().unwrap();

        let error = install_bundled(&misnamed, folder.path()).unwrap_err();
        assert!(error.to_string().contains("holds eng_kjv-"), "{error}");
        assert!(!folder.path().join("eng_kjv-9.9.9.db").exists());
        assert!(!folder.path().join(".installing-module.tmp").exists());
    }

    #[test]
    fn refuses_bundled_files_without_a_version_in_the_name() {
        let folder = tempfile::tempdir().unwrap();
        let error =
            install_bundled(Path::new("/resources/eng_kjv.db.gz"), folder.path()).unwrap_err();
        assert!(error.to_string().contains("isn't named"), "{error}");
    }

    #[test]
    fn installs_every_bundled_module() {
        let resources = tempfile::tempdir().unwrap();
        fs::copy(
            bundled_gz(),
            resources.path().join(format!("{}.gz", kjv_file_name())),
        )
        .unwrap();
        fs::write(resources.path().join("notes.txt"), "not a module").unwrap();
        let folder = tempfile::tempdir().unwrap();

        let results = install_all_bundled(resources.path(), folder.path()).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(
            results[0].1.as_ref().unwrap(),
            &InstallOutcome::Installed(folder.path().join(kjv_file_name()))
        );
    }

    #[test]
    fn rejects_unsafe_ids_and_versions_in_file_names() {
        let info = |id: Option<&str>, version: &str| queries::ModuleInfo {
            name: "Test".into(),
            description: String::new(),
            module_type: "bible".into(),
            fetch_id: id.map(Into::into),
            language: "English".into(),
            language_code: "eng".into(),
            license: "Public Domain".into(),
            version: version.into(),
            format_version: 1,
        };
        assert_eq!(
            module_file_name(&info(Some("eng_kjv"), "1.0.0")).unwrap(),
            "eng_kjv-1.0.0.db"
        );
        for (id, version) in [
            (Some("../../evil"), "1.0.0"),
            (Some("eng_kjv"), "1.0/../../x"),
            (Some(".hidden"), "1.0.0"),
            (Some("eng kjv"), "1.0.0"),
            (None, "1.0.0"),
        ] {
            assert!(
                module_file_name(&info(id, version)).is_err(),
                "{id:?} {version}"
            );
        }
    }

    #[test]
    fn writes_a_readme_only_if_missing() {
        let folder = tempfile::tempdir().unwrap();
        write_readme(folder.path()).unwrap();
        let readme = folder.path().join("README.txt");
        assert!(fs::read_to_string(&readme)
            .unwrap()
            .contains("copy and share"));

        fs::write(&readme, "my own notes").unwrap();
        write_readme(folder.path()).unwrap();
        assert_eq!(fs::read_to_string(&readme).unwrap(), "my own notes");
    }

    #[test]
    fn scan_finds_modules_and_reports_other_files() {
        let folder = tempfile::tempdir().unwrap();
        install_bundled(&bundled_gz(), folder.path()).unwrap();
        fs::write(folder.path().join("broken.db"), "not a module").unwrap();
        fs::write(folder.path().join("notes.txt"), "ignored").unwrap();
        fs::write(folder.path().join(".hidden.db"), "ignored").unwrap();

        let (modules, skipped) = scan(folder.path()).unwrap();
        assert_eq!(modules.len(), 1);
        assert_eq!(modules[0].id, "eng_kjv");
        assert_eq!(modules[0].name, "King James Version (1769)");
        assert_eq!(skipped.len(), 1);
        assert!(skipped[0].path.ends_with("broken.db"));
    }

    #[test]
    fn compares_versions_by_number() {
        assert_eq!(compare_versions("1.10.0", "1.9.2"), Ordering::Greater);
        assert_eq!(compare_versions("1.0.0", "1.0.0"), Ordering::Equal);
        assert_eq!(compare_versions("1.0", "1.0.1"), Ordering::Less);
        assert_eq!(compare_versions("2.0.0", "10.0.0"), Ordering::Less);
    }

    #[test]
    fn moves_modules_and_leaves_other_files() {
        let old = tempfile::tempdir().unwrap();
        let new = tempfile::tempdir().unwrap();
        install_bundled(&bundled_gz(), old.path()).unwrap();
        write_readme(old.path()).unwrap();
        let mut other = File::create(old.path().join("notes.db")).unwrap();
        other.write_all(b"someone's other file").unwrap();

        let target = new.path().join("modules");
        let report = move_modules(old.path(), &target).unwrap();
        assert_eq!(report.moved, [kjv_file_name()]);
        assert!(report.left_behind.is_empty());
        assert!(ModuleDb::open(&target.join(kjv_file_name())).is_ok());
        assert!(!old.path().join(kjv_file_name()).exists());
        assert!(old.path().join("notes.db").exists());
        assert!(old.path().join("README.txt").exists());
    }

    #[test]
    fn leaves_a_module_behind_if_the_new_folder_has_that_file() {
        let old = tempfile::tempdir().unwrap();
        let new = tempfile::tempdir().unwrap();
        install_bundled(&bundled_gz(), old.path()).unwrap();
        install_bundled(&bundled_gz(), new.path()).unwrap();

        let report = move_modules(old.path(), new.path()).unwrap();
        assert!(report.moved.is_empty());
        assert_eq!(report.left_behind, [kjv_file_name()]);
        assert!(old.path().join(kjv_file_name()).exists());
    }

    #[test]
    fn moving_to_the_same_folder_does_nothing() {
        let folder = tempfile::tempdir().unwrap();
        install_bundled(&bundled_gz(), folder.path()).unwrap();
        assert_eq!(
            move_modules(folder.path(), folder.path()).unwrap(),
            MoveReport::default()
        );
        assert!(folder.path().join(kjv_file_name()).exists());
    }
}
