//! Local IPA validation: extension + ZIP structure + safety gates.
//!
//! Runs before any signing/install step so invalid files are rejected
//! locally without network traffic. DenizSigner never extracts IPA contents
//! itself (that happens inside `isideload`); these checks reject hostile or
//! corrupt archives early with a clean, user-readable error.
//!
//! Gates: `.ipa` extension, valid ZIP, `Payload/*.app/Info.plist` present,
//! no `..` / absolute / drive-letter entries (path traversal), no symlinks,
//! bounded entry count and total uncompressed size.

use std::io::Read;
use std::path::Path;

use crate::error::AppError;

/// Upper bound for entries scanned (zip bomb / fuzz defense).
const MAX_ENTRIES: usize = 100_000;
/// Upper bound for summed uncompressed sizes (8 GiB — far above real IPAs).
const MAX_TOTAL_UNCOMPRESSED: u64 = 8 * 1024 * 1024 * 1024;

pub fn validate_ipa_path(path: &str) -> Result<(), AppError> {
    let p = Path::new(path);
    if p.extension().and_then(|e| e.to_str()).map(|e| e.eq_ignore_ascii_case("ipa")) != Some(true) {
        return Err(AppError::InvalidIpa(format!(
            "Not an .ipa file: {}",
            p.file_name().and_then(|n| n.to_str()).unwrap_or("?")
        )));
    }
    let file = std::fs::File::open(p).map_err(|e| {
        AppError::Filesystem("Failed to read IPA file".into(), e.to_string())
    })?;
    validate_ipa_reader(file)
}

fn validate_ipa_reader<R: Read + std::io::Seek>(reader: R) -> Result<(), AppError> {
    let mut zip = zip::ZipArchive::new(reader)
        .map_err(|_| AppError::InvalidIpa("File is not a valid ZIP/IPA archive.".into()))?;
    if zip.len() > MAX_ENTRIES {
        return Err(AppError::InvalidIpa(format!(
            "Archive has too many entries ({}).",
            zip.len()
        )));
    }
    let mut found_app = false;
    let mut total: u64 = 0;
    for i in 0..zip.len() {
        let f = zip.by_index(i).map_err(|_| {
            AppError::InvalidIpa("Archive entry is unreadable.".into())
        })?;
        check_entry_name(f.name())?;
        // Symlink entries (unix file-type bits 0o120_000) are rejected;
        // directories and regular files pass.
        if f.unix_mode().is_some_and(|m| m & 0o170_000 == 0o120_000) {
            return Err(AppError::InvalidIpa(format!(
                "Archive contains a symlink ({}), rejected.",
                f.name()
            )));
        }
        total = total
            .checked_add(f.size())
            .filter(|t| *t <= MAX_TOTAL_UNCOMPRESSED)
            .ok_or_else(|| {
                AppError::InvalidIpa("Archive is suspiciously large, rejected.".into())
            })?;
        if f.name().starts_with("Payload/") && f.name().ends_with(".app/Info.plist") {
            found_app = true;
        }
    }
    if !found_app {
        return Err(AppError::InvalidIpa(
            "Archive has no Payload/*.app bundle.".into(),
        ));
    }
    Ok(())
}

/// Rejects traversal (`..`), absolute paths (`/...`), Windows drive paths
/// (`C:...`), and backslash escapes. Mirrors what safe extractors enforce.
fn check_entry_name(name: &str) -> Result<(), AppError> {
    let bad = name.is_empty()
        || name.starts_with('/')
        || name.starts_with('\\')
        || name.contains("..")
        || name.contains('\\')
        || (name.len() >= 2 && name.as_bytes()[1] == b':' && name.as_bytes()[0].is_ascii_alphabetic());
    if bad {
        return Err(AppError::InvalidIpa(format!(
            "Archive contains an unsafe entry name, rejected: {name}"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, Write};

    fn make_ipa(with_app: bool) -> Vec<u8> {
        let mut buf = Cursor::new(Vec::new());
        let mut zip = zip::ZipWriter::new(&mut buf);
        zip.start_file("Payload/Test.app/Info.plist", zip::write::SimpleFileOptions::default()).unwrap();
        if with_app {
            zip.write_all(b"plist").unwrap();
        } else {
            // write unrelated entry instead
        }
        let _ = zip.finish().unwrap();
        buf.into_inner()
    }

    fn make_ipa_with_entry(name: &str) -> Vec<u8> {
        let mut buf = Cursor::new(Vec::new());
        {
            let mut zip = zip::ZipWriter::new(&mut buf);
            zip.start_file("Payload/Test.app/Info.plist", zip::write::SimpleFileOptions::default()).unwrap();
            zip.write_all(b"plist").unwrap();
            zip.start_file(name, zip::write::SimpleFileOptions::default()).unwrap();
            zip.write_all(b"x").unwrap();
            zip.finish().unwrap();
        }
        buf.into_inner()
    }

    #[test]
    fn rejects_non_ipa_extension() {
        assert!(validate_ipa_path("C:\\tmp\\app.zip").is_err());
    }

    #[test]
    fn rejects_archive_without_app() {
        let mut buf = Cursor::new(Vec::new());
        {
            let mut zip = zip::ZipWriter::new(&mut buf);
            zip.start_file("readme.txt", zip::write::SimpleFileOptions::default()).unwrap();
            zip.write_all(b"hi").unwrap();
            zip.finish().unwrap();
        }
        assert!(validate_ipa_reader(Cursor::new(buf.into_inner())).is_err());
    }

    #[test]
    fn accepts_valid_ipa_structure() {
        assert!(validate_ipa_reader(Cursor::new(make_ipa(true))).is_ok());
    }

    #[test]
    fn blocks_traversal_and_absolute_entries() {
        for evil in [
            "../evil.plist",
            "Payload/../../evil",
            "/abs/path.plist",
            "C:\\Windows\\evil.dll",
            "Payload\\..\\evil",
        ] {
            assert!(
                validate_ipa_reader(Cursor::new(make_ipa_with_entry(evil))).is_err(),
                "{evil}"
            );
        }
    }

    #[test]
    fn entry_name_gate_unit() {
        assert!(check_entry_name("Payload/App.app/Info.plist").is_ok());
        for bad in ["", "/x", "../x", "a/../b", "C:/x", "a\\b"] {
            assert!(check_entry_name(bad).is_err(), "{bad}");
        }
    }
}
