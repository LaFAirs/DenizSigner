//! Local IPA validation: extension + ZIP magic + Payload/<name>.app entry.
//!
//! Runs before any signing/install step so invalid files are rejected
//! locally without network traffic.

use std::io::Read;
use std::path::Path;

use crate::error::AppError;

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
    let mut found_app = false;
    for i in 0..zip.len() {
        if let Ok(f) = zip.by_index(i) {
            let name = f.name().to_string();
            if name.starts_with("Payload/") && name.ends_with(".app/Info.plist") {
                found_app = true;
                break;
            }
        }
    }
    if !found_app {
        return Err(AppError::InvalidIpa(
            "Archive has no Payload/*.app bundle.".into(),
        ));
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
}
