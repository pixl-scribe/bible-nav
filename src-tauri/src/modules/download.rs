//! Fetching the module catalog and downloading modules, only when the
//! reader asks.
//!
//! A download is written to hidden temporary files in the modules folder,
//! checked against the catalog's size and SHA-256, unpacked, and then
//! checked exactly as any module is (signature, file hash, roots, pins)
//! before it's put in place under its real name. Nothing half-downloaded
//! or unchecked ever appears as a module, and an existing module is never
//! replaced: a new version is a new file beside the old one.

use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use flate2::read::GzDecoder;
use sha2::{Digest, Sha256};

use super::catalog::{Offer, MAX_CATALOG_BYTES};
use super::db::ModuleDb;
use super::error::{ModuleError, Result};
use super::integrity::sidecars;
use super::store::safe_file_name;
use super::trust::Trust;

/// Where downloads come from: the web, or (in tests) anything else.
pub trait Source {
    /// Opens `url` for reading.
    fn open(&self, url: &str) -> Result<Box<dyn Read>>;
}

/// The web, over HTTPS.
pub struct Web {
    agent: ureq::Agent,
}

impl Web {
    pub fn new() -> Self {
        let config = ureq::Agent::config_builder()
            .timeout_connect(Some(Duration::from_secs(20)))
            // A slow connection may take long over a whole module, but
            // never this long between two pieces of it.
            .timeout_recv_body(Some(Duration::from_secs(60)))
            .https_only(true)
            .user_agent("Bible Nav")
            .build();
        Self {
            agent: config.into(),
        }
    }
}

impl Source for Web {
    fn open(&self, url: &str) -> Result<Box<dyn Read>> {
        let response = self
            .agent
            .get(url)
            .call()
            .map_err(|error| ModuleError::Network(error.to_string()))?;
        Ok(Box::new(response.into_body().into_reader()))
    }
}

/// Reads the catalog from the first of `urls` that answers. Returns the
/// address used and the catalog's bytes.
pub fn fetch_catalog(source: &dyn Source, urls: &[&str]) -> Result<(String, Vec<u8>)> {
    let mut last = ModuleError::Network("no addresses to try".into());
    for url in urls {
        match source
            .open(url)
            .and_then(|reader| read_limited(reader, MAX_CATALOG_BYTES))
        {
            Ok(bytes) => return Ok((url.to_string(), bytes)),
            Err(error) => last = error,
        }
    }
    Err(last)
}

fn read_limited(reader: Box<dyn Read>, limit: u64) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    reader.take(limit + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        return Err(ModuleError::Network(
            "the module list is far too large".into(),
        ));
    }
    Ok(bytes)
}

/// What installing an offer did.
#[derive(Debug, PartialEq, Eq)]
pub enum Installed {
    /// Downloaded, checked, and put in place at this path.
    New(PathBuf),
    /// A valid copy was already there; nothing was downloaded.
    AlreadyPresent(PathBuf),
}

/// Downloads an offered module into `folder`, checks it completely with
/// `trust`, and puts it in place. `progress` hears (bytes so far, total)
/// as the download goes; setting `cancel` stops it.
pub fn install(
    offer: &Offer,
    folder: &Path,
    source: &dyn Source,
    trust: &Trust,
    progress: &mut dyn FnMut(u64, u64),
    cancel: &AtomicBool,
) -> Result<Installed> {
    let module = &offer.manifest.module;
    let file_name = safe_file_name(&module.id, &module.version)?;
    if offer.manifest.file.name != file_name {
        return Err(ModuleError::Untrusted(format!(
            "its manifest is for {}, not {file_name}",
            offer.manifest.file.name
        )));
    }
    let target = folder.join(&file_name);
    if target.exists() && ModuleDb::open(&target).is_ok() {
        return Ok(Installed::AlreadyPresent(target));
    }

    fs::create_dir_all(folder)?;
    // Hidden names, so the folder scan never sees a part-done download.
    let temp = Temp {
        gz: folder.join(format!(".download-{file_name}.gz")),
        db: folder.join(format!(".download-{file_name}")),
    };
    let result = temp.fill(offer, source, trust, &file_name, progress, cancel);
    let placed = result.and_then(|()| place(offer, &temp.db, &target));
    temp.remove();
    placed.map(|()| Installed::New(target))
}

/// The temporary files of one download, removed whatever happens.
struct Temp {
    gz: PathBuf,
    db: PathBuf,
}

impl Temp {
    fn fill(
        &self,
        offer: &Offer,
        source: &dyn Source,
        trust: &Trust,
        file_name: &str,
        progress: &mut dyn FnMut(u64, u64),
        cancel: &AtomicBool,
    ) -> Result<()> {
        download_checked(offer, source, &self.gz, progress, cancel)?;
        unpack(&self.gz, &self.db, offer.manifest.file.size)?;
        let files = sidecars(&self.db);
        fs::write(&files.manifest, &offer.entry.manifest)?;
        fs::write(&files.signatures, &offer.entry.signatures)?;
        // The same checks as any module in the folder. The block closes the
        // module before it's renamed.
        {
            ModuleDb::open_checked(&self.db, &files, file_name, trust)?;
        }
        Ok(())
    }

    fn remove(&self) {
        let files = sidecars(&self.db);
        for path in [&self.gz, &self.db, &files.manifest, &files.signatures] {
            let _ = fs::remove_file(path);
        }
    }
}

/// Downloads the compressed module from the first address that gives the
/// catalog's exact size and SHA-256.
fn download_checked(
    offer: &Offer,
    source: &dyn Source,
    gz: &Path,
    progress: &mut dyn FnMut(u64, u64),
    cancel: &AtomicBool,
) -> Result<()> {
    let download = &offer.entry.download;
    let mut last = ModuleError::Network("no addresses to try".into());
    for url in &download.urls {
        let attempt = source.open(url).and_then(|reader| {
            let mut file = File::create(gz)?;
            let (size, sha256) = copy_counted(reader, &mut file, download.size, progress, cancel)?;
            file.sync_all()?;
            if size != download.size || sha256 != download.sha256 {
                return Err(ModuleError::Network(format!(
                    "{url} gave a different file than the module list describes"
                )));
            }
            Ok(())
        });
        match attempt {
            Ok(()) => return Ok(()),
            Err(ModuleError::Cancelled) => return Err(ModuleError::Cancelled),
            Err(error) => last = error,
        }
    }
    Err(last)
}

/// Copies at most `expected` bytes (a longer download fails), reporting
/// progress and stopping when cancelled. Returns the size and SHA-256.
fn copy_counted(
    mut reader: Box<dyn Read>,
    out: &mut dyn Write,
    expected: u64,
    progress: &mut dyn FnMut(u64, u64),
    cancel: &AtomicBool,
) -> Result<(u64, String)> {
    let mut hasher = Sha256::new();
    let mut buffer = vec![0; 1 << 16];
    let mut total = 0u64;
    loop {
        if cancel.load(Ordering::Relaxed) {
            return Err(ModuleError::Cancelled);
        }
        let read = match reader.read(&mut buffer) {
            Ok(0) => break,
            Ok(read) => read,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(ModuleError::Network(error.to_string())),
        };
        total += read as u64;
        if total > expected {
            return Err(ModuleError::Network(
                "the download is larger than the module list says".into(),
            ));
        }
        hasher.update(&buffer[..read]);
        out.write_all(&buffer[..read])?;
        progress(total, expected);
    }
    let sha256 = hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    Ok((total, sha256))
}

/// Unpacks a gzipped module, refusing to write more than its manifest's
/// size, so a hostile file can't fill the disk.
fn unpack(gz: &Path, db: &Path, size: u64) -> Result<()> {
    let decoder = GzDecoder::new(File::open(gz)?);
    let mut out = File::create(db)?;
    let written = io::copy(&mut decoder.take(size + 1), &mut out)?;
    out.sync_all()?;
    if written != size {
        return Err(ModuleError::Untrusted(
            "the downloaded file doesn't match its signed manifest".into(),
        ));
    }
    Ok(())
}

/// Writes the manifest and signatures under their real names, then renames
/// the checked module into place: the module appears last, so it never
/// appears without them. An invalid file already at the target name is set
/// aside as `.rejected`, as installs of built-in modules do.
fn place(offer: &Offer, checked: &Path, target: &Path) -> Result<()> {
    if target.exists() {
        fs::rename(target, target.with_extension("db.rejected"))?;
    }
    let files = sidecars(target);
    fs::write(&files.manifest, &offer.entry.manifest)?;
    fs::write(&files.signatures, &offer.entry.signatures)?;
    fs::rename(checked, target)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::catalog::{check_entries, parse};
    use crate::modules::test_support::{fixture_path, kjv_file_name};
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use std::collections::HashMap;
    use std::io::Cursor;

    /// Serves files from memory, by address.
    struct Memory(HashMap<String, Vec<u8>>);

    impl Source for Memory {
        fn open(&self, url: &str) -> Result<Box<dyn Read>> {
            self.0
                .get(url)
                .map(|bytes| Box::new(Cursor::new(bytes.clone())) as Box<dyn Read>)
                .ok_or_else(|| ModuleError::Network(format!("{url}: not found")))
        }
    }

    const URL: &str = "https://example.org/eng_kjv.db.gz";
    const MIRROR: &str = "https://mirror.example.org/eng_kjv.db.gz";

    fn gz(bytes: &[u8]) -> Vec<u8> {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::fast());
        encoder.write_all(bytes).unwrap();
        encoder.finish().unwrap()
    }

    fn hex(bytes: &[u8]) -> String {
        Sha256::digest(bytes)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }

    /// The bundled KJV offered by a catalog, with its compressed bytes.
    fn kjv_offer(urls: &[&str]) -> (Offer, Vec<u8>) {
        let compressed = gz(&fs::read(fixture_path()).unwrap());
        let files = sidecars(&fixture_path());
        let catalog = serde_json::json!({
            "type": "bible-nav module catalog",
            "catalogVersion": 1,
            "modules": [{
                "manifest": fs::read_to_string(files.manifest).unwrap(),
                "signatures": fs::read_to_string(files.signatures).unwrap(),
                "download": {
                    "name": format!("{}.gz", kjv_file_name()),
                    "size": compressed.len(),
                    "sha256": hex(&compressed),
                    "urls": urls,
                }
            }]
        });
        let parsed = parse(&serde_json::to_vec(&catalog).unwrap(), "test").unwrap();
        let (mut offers, skipped) =
            check_entries(parsed.entries, &crate::modules::trust::current());
        assert!(skipped.is_empty(), "{skipped:?}");
        (offers.remove(0), compressed)
    }

    fn run(offer: &Offer, folder: &Path, source: &Memory) -> Result<Installed> {
        install(
            offer,
            folder,
            source,
            &crate::modules::trust::current(),
            &mut |_, _| {},
            &AtomicBool::new(false),
        )
    }

    fn hidden_files(folder: &Path) -> Vec<String> {
        fs::read_dir(folder)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|name| name.starts_with('.'))
            .collect()
    }

    #[test]
    fn a_download_is_checked_and_put_in_place_with_its_manifest() {
        let (offer, compressed) = kjv_offer(&[URL]);
        let folder = tempfile::tempdir().unwrap();
        let source = Memory(HashMap::from([(URL.to_string(), compressed)]));
        let mut seen = Vec::new();
        let installed = install(
            &offer,
            folder.path(),
            &source,
            &crate::modules::trust::current(),
            &mut |done, total| seen.push((done, total)),
            &AtomicBool::new(false),
        )
        .unwrap();
        let target = folder.path().join(kjv_file_name());
        assert_eq!(installed, Installed::New(target.clone()));
        assert!(ModuleDb::open(&target).is_ok());
        assert!(sidecars(&target).signatures.exists());
        assert!(hidden_files(folder.path()).is_empty());
        let (done, total) = *seen.last().unwrap();
        assert_eq!(done, total);

        // A second time, nothing is downloaded.
        let empty = Memory(HashMap::new());
        assert_eq!(
            run(&offer, folder.path(), &empty).unwrap(),
            Installed::AlreadyPresent(target)
        );
    }

    #[test]
    fn a_bad_address_falls_back_to_the_next() {
        let (offer, compressed) = kjv_offer(&[URL, MIRROR]);
        let folder = tempfile::tempdir().unwrap();
        let mut broken = compressed.clone();
        broken[100] ^= 1;
        let source = Memory(HashMap::from([
            (URL.to_string(), broken),
            (MIRROR.to_string(), compressed),
        ]));
        assert!(matches!(
            run(&offer, folder.path(), &source).unwrap(),
            Installed::New(_)
        ));
    }

    #[test]
    fn a_wrong_file_never_appears_as_a_module() {
        let (offer, compressed) = kjv_offer(&[URL]);
        let folder = tempfile::tempdir().unwrap();
        let mut broken = compressed;
        broken[100] ^= 1;
        let source = Memory(HashMap::from([(URL.to_string(), broken)]));
        let error = run(&offer, folder.path(), &source).unwrap_err();
        assert!(error.to_string().contains("different file"), "{error}");
        assert_eq!(fs::read_dir(folder.path()).unwrap().count(), 0);
    }

    #[test]
    fn a_file_matching_the_catalog_but_not_the_signed_manifest_is_refused() {
        // A catalog's size and hash aren't signed: a hostile catalog can
        // describe a tampered file exactly. The signed manifest still stops it.
        let (mut offer, _) = kjv_offer(&[URL]);
        let mut altered = fs::read(fixture_path()).unwrap();
        let at = altered.len() / 2;
        altered[at] ^= 1;
        let compressed = gz(&altered);
        offer.entry.download.size = compressed.len() as u64;
        offer.entry.download.sha256 = hex(&compressed);
        let folder = tempfile::tempdir().unwrap();
        let source = Memory(HashMap::from([(URL.to_string(), compressed)]));
        let error = run(&offer, folder.path(), &source).unwrap_err();
        assert!(error.to_string().contains("damaged or altered"), "{error}");
        assert_eq!(fs::read_dir(folder.path()).unwrap().count(), 0);
    }

    #[test]
    fn cancelling_stops_the_download_and_leaves_nothing() {
        let (offer, compressed) = kjv_offer(&[URL]);
        let folder = tempfile::tempdir().unwrap();
        let source = Memory(HashMap::from([(URL.to_string(), compressed)]));
        let cancel = AtomicBool::new(false);
        let error = install(
            &offer,
            folder.path(),
            &source,
            &crate::modules::trust::current(),
            &mut |done, _| {
                if done > 100_000 {
                    cancel.store(true, Ordering::Relaxed);
                }
            },
            &cancel,
        )
        .unwrap_err();
        assert!(matches!(error, ModuleError::Cancelled));
        assert_eq!(fs::read_dir(folder.path()).unwrap().count(), 0);
    }

    #[test]
    fn the_catalog_comes_from_the_first_address_that_answers() {
        let source = Memory(HashMap::from([(MIRROR.to_string(), b"{}".to_vec())]));
        let (used, bytes) = fetch_catalog(&source, &[URL, MIRROR]).unwrap();
        assert_eq!(used, MIRROR);
        assert_eq!(bytes, b"{}");
        assert!(fetch_catalog(&source, &[URL]).is_err());
    }
}
