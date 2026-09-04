use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

use crate::error::{AppError, AppErrorKind, AppResult};

/// Entry-count and cumulative-decompressed-size caps for [`extract_hardened_zip`] — deliberately a
/// separate, more generous budget than the vsix theme-extraction path's
/// `VSIX_TOTAL_MAX_EXTRACTED_BYTES` (64MB, sized for a handful of small JSON theme files): a plugin
/// bundle can legitimately contain several grammar files, icons, and a `taide-plugin.json`, but
/// must still be bounded against a zip bomb (a small archive claiming to expand to gigabytes).
/// `domain::vsix::service::stage_vsix_import` reuses [`ARCHIVE_MAX_TOTAL_BYTES`] as its own
/// entry-by-entry read budget so both plugin-import flows share one cap.
pub const ARCHIVE_MAX_ENTRIES: usize = 5_000;
pub const ARCHIVE_MAX_TOTAL_BYTES: u64 = 128 * 1024 * 1024;
const HARDENED_FILE_MODE: u32 = 0o644;
const HARDENED_DIR_MODE: u32 = 0o755;
const COPY_CHUNK_BYTES: usize = 64 * 1024;

#[cfg(unix)]
fn set_hardened_mode(path: &Path, mode: u32) {
    use std::os::unix::fs::PermissionsExt;
    let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode));
}
#[cfg(not(unix))]
fn set_hardened_mode(_path: &Path, _mode: u32) {}

/// Streams one zip entry to `out_path`, decrementing `remaining_budget` by every byte actually
/// decompressed (not the entry's *declared* size, which a crafted archive could lie about) and
/// erroring out mid-copy the moment the cumulative budget across the whole archive is exhausted —
/// the real zip-bomb defense, since a maliciously small compressed entry can still expand to
/// gigabytes via DEFLATE.
fn copy_entry_with_budget(mut entry: impl Read, out_path: &Path, remaining_budget: &mut u64) -> AppResult<()> {
    let mut out_file = std::fs::File::create(out_path)?;
    let mut buffer = [0u8; COPY_CHUNK_BYTES];
    loop {
        let read = entry.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        *remaining_budget = remaining_budget.checked_sub(read as u64).ok_or_else(|| {
            AppError::localized(
                AppErrorKind::InvalidArgument,
                "error.archive.extractSizeLimitExceeded",
                "the archive extraction size limit was exceeded",
            )
        })?;
        out_file.write_all(&buffer[..read])?;
    }
    Ok(())
}

/// Extracts an untrusted zip archive (a plugin bundle, whether a TAIDE-native `.vsix`/zip or —
/// indirectly, entry-by-entry rather than through this function — a real VS Code `.vsix`) to
/// `dest`, enforcing an entry-count cap and a cumulative decompressed-size budget on top of the
/// zip-slip protection `ZipArchive::enclosed_name` already provides, and masking every extracted
/// file/directory to a fixed 0o644/0o755 regardless of what `unix_mode` the archive itself claims.
///
/// Lives in `infra` because both `domain::plugin` (archive installs) and `domain::vsix` (its
/// extraction budget) need it — sharing it domain-to-domain used to be one half of the
/// plugin ↔ vsix import cycle the domain-boundary rule forbids (audit R7#4, T1-I §1.3).
///
/// Deliberately not a reuse of `infra::lsp_install::extract_zip` (contract §3.4 says so explicitly):
/// that function trusts the archive's `unix_mode` verbatim and has no size/entry cap at all — fine
/// for **first-party, checksum-verified** LSP server downloads (`lsp::commands::run_download_install`
/// already validates a SHA-256 before extraction), unsafe for an arbitrary user-supplied plugin
/// archive with no such provenance check.
pub fn extract_hardened_zip(source_path: &Path, dest: &Path) -> AppResult<()> {
    let file = File::open(source_path).map_err(|error| {
        AppError::localized(
            AppErrorKind::InvalidArgument,
            "error.archive.openFailed",
            format!("could not open archive: {error}"),
        )
        .with_arg("detail", &error)
    })?;
    let mut archive = zip::ZipArchive::new(file).map_err(|error| {
        AppError::localized(
            AppErrorKind::InvalidArgument,
            "error.archive.unzipFailed",
            format!("could not unzip archive: {error}"),
        )
        .with_arg("detail", &error)
    })?;

    if archive.len() > ARCHIVE_MAX_ENTRIES {
        return Err(AppError::localized(
            AppErrorKind::InvalidArgument,
            "error.archive.tooManyEntries",
            format!("archive has too many entries ({}, max {ARCHIVE_MAX_ENTRIES})", archive.len()),
        )
        .with_arg("count", archive.len())
        .with_arg("max", ARCHIVE_MAX_ENTRIES));
    }

    let mut remaining_budget = ARCHIVE_MAX_TOTAL_BYTES;

    for index in 0..archive.len() {
        let entry = archive.by_index(index).map_err(|error| {
            AppError::localized(
                AppErrorKind::Internal,
                "error.archive.entryReadFailed",
                format!("failed to read zip entry: {error}"),
            )
            .with_arg("detail", &error)
        })?;
        let Some(relative_path) = entry.enclosed_name() else { continue };
        let out_path = dest.join(relative_path);

        if entry.is_dir() {
            std::fs::create_dir_all(&out_path)?;
            set_hardened_mode(&out_path, HARDENED_DIR_MODE);
            continue;
        }

        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        copy_entry_with_budget(entry, &out_path, &mut remaining_budget)?;
        set_hardened_mode(&out_path, HARDENED_FILE_MODE);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use zip::write::SimpleFileOptions;

    use super::*;
    use crate::error::AppErrorKind;

    fn temp_dir(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("taide-archive-test-{name}-{}", uuid::Uuid::new_v4()))
    }

    fn cleanup(dir: &Path) {
        std::fs::remove_dir_all(dir).ok();
    }

    fn write_archive(dir: &Path, entries: &[(&str, &[u8])], options: SimpleFileOptions) -> std::path::PathBuf {
        let mut bytes: Vec<u8> = Vec::new();
        {
            let mut writer = zip::ZipWriter::new(Cursor::new(&mut bytes));
            for (name, content) in entries {
                if name.ends_with('/') {
                    writer.add_directory(*name, options).unwrap();
                    continue;
                }
                writer.start_file(*name, options).unwrap();
                writer.write_all(content).unwrap();
            }
            writer.finish().unwrap();
        }

        std::fs::create_dir_all(dir).unwrap();
        let path = dir.join("bundle.zip");
        std::fs::write(&path, &bytes).unwrap();
        path
    }

    fn localized_key(error: &AppError) -> String {
        match error {
            AppError::Localized(localized) => localized.key.clone(),
            other => panic!("localized 에러여야 합니다: {other:?}"),
        }
    }

    #[cfg(unix)]
    fn mode_of(path: &Path) -> u32 {
        use std::os::unix::fs::PermissionsExt;
        std::fs::metadata(path).unwrap().permissions().mode() & 0o777
    }

    #[test]
    fn 중첩_경로_엔트리를_그대로_풀어낸다() {
        let dir = temp_dir("nested");
        let source = write_archive(
            &dir,
            &[("taide-plugin.json", b"{}"), ("syntaxes/rust/rust.tmLanguage.json", b"grammar")],
            SimpleFileOptions::default(),
        );
        let dest = dir.join("out");

        extract_hardened_zip(&source, &dest).expect("정상 아카이브는 풀려야 합니다");

        assert_eq!(std::fs::read_to_string(dest.join("taide-plugin.json")).unwrap(), "{}");
        assert_eq!(
            std::fs::read_to_string(dest.join("syntaxes/rust/rust.tmLanguage.json")).unwrap(),
            "grammar",
            "디렉터리 엔트리가 없어도 부모 경로를 만들어야 합니다"
        );

        cleanup(&dir);
    }

    #[test]
    fn 상위_경로로_탈출하는_엔트리는_무시된다() {
        let dir = temp_dir("zip-slip");
        let source = write_archive(
            &dir,
            &[("../escaped.txt", b"owned"), ("kept.txt", b"kept")],
            SimpleFileOptions::default(),
        );
        let dest = dir.join("out");

        extract_hardened_zip(&source, &dest).expect("탈출 엔트리는 건너뛰고 나머지는 풀려야 합니다");

        assert!(!dir.join("escaped.txt").exists(), "dest 밖에 파일이 생기면 zip-slip 입니다");
        assert!(!dest.join("escaped.txt").exists());
        assert_eq!(std::fs::read_to_string(dest.join("kept.txt")).unwrap(), "kept");

        cleanup(&dir);
    }

    #[test]
    fn 절대_경로_엔트리도_무시된다() {
        let dir = temp_dir("absolute");
        let escape_target = dir.join("absolute-escape.txt");
        let source = write_archive(
            &dir,
            &[(escape_target.to_str().unwrap(), b"owned"), ("kept.txt", b"kept")],
            SimpleFileOptions::default(),
        );
        let dest = dir.join("out");

        extract_hardened_zip(&source, &dest).expect("절대 경로 엔트리는 건너뛰고 나머지는 풀려야 합니다");

        assert!(!escape_target.exists(), "절대 경로 엔트리가 dest 밖에 쓰이면 안 됩니다");
        assert!(dest.join("kept.txt").exists());

        cleanup(&dir);
    }

    #[test]
    fn 엔트리_수_상한_이하는_통과한다() {
        let dir = temp_dir("entry-cap-ok");
        let names: Vec<String> = (0..ARCHIVE_MAX_ENTRIES).map(|index| format!("f{index}.txt")).collect();
        let entries: Vec<(&str, &[u8])> = names.iter().map(|name| (name.as_str(), b"x".as_slice())).collect();
        let source = write_archive(&dir, &entries, SimpleFileOptions::default());
        let dest = dir.join("out");

        extract_hardened_zip(&source, &dest).expect("정확히 상한만큼의 엔트리는 허용됩니다");

        assert!(dest.join(format!("f{}.txt", ARCHIVE_MAX_ENTRIES - 1)).exists());

        cleanup(&dir);
    }

    #[test]
    fn 엔트리_수_상한을_넘으면_한_건도_풀지_않는다() {
        let dir = temp_dir("entry-cap-over");
        let names: Vec<String> = (0..=ARCHIVE_MAX_ENTRIES).map(|index| format!("f{index}.txt")).collect();
        let entries: Vec<(&str, &[u8])> = names.iter().map(|name| (name.as_str(), b"x".as_slice())).collect();
        let source = write_archive(&dir, &entries, SimpleFileOptions::default());
        let dest = dir.join("out");

        let error = extract_hardened_zip(&source, &dest).expect_err("상한 초과 아카이브는 거부되어야 합니다");

        assert_eq!(error.kind(), AppErrorKind::InvalidArgument);
        assert_eq!(localized_key(&error), "error.archive.tooManyEntries");
        assert!(!dest.exists(), "엔트리 수 검사는 어떤 파일도 쓰기 전에 끝나야 합니다");

        cleanup(&dir);
    }

    /// The zip-bomb case the cumulative budget exists for: a single entry of highly compressible
    /// repeated data whose *decompressed* size crosses [`ARCHIVE_MAX_TOTAL_BYTES`] while the
    /// archive on disk stays tiny — so nothing but counting the bytes as they come out of the
    /// decompressor can catch it.
    #[test]
    fn 압축률이_높은_반복_데이터가_해제_예산을_넘으면_중단된다() {
        let dir = temp_dir("budget");
        let bomb = vec![0u8; usize::try_from(ARCHIVE_MAX_TOTAL_BYTES).unwrap() + COPY_CHUNK_BYTES];
        let source = write_archive(&dir, &[("bomb.bin", &bomb)], SimpleFileOptions::default());
        drop(bomb);
        let compressed_size = std::fs::metadata(&source).unwrap().len();
        let dest = dir.join("out");

        let error = extract_hardened_zip(&source, &dest).expect_err("해제 예산 초과는 거부되어야 합니다");

        assert!(
            compressed_size < ARCHIVE_MAX_TOTAL_BYTES,
            "압축된 아카이브 자체는 예산보다 작아야 예산이 의미를 갖습니다 ({compressed_size} bytes)"
        );
        assert_eq!(error.kind(), AppErrorKind::InvalidArgument);
        assert_eq!(localized_key(&error), "error.archive.extractSizeLimitExceeded");

        cleanup(&dir);
    }

    #[test]
    fn 해제_예산은_선언된_크기가_아니라_실제_해제_바이트로_깎인다() {
        let dir = temp_dir("budget-actual");
        std::fs::create_dir_all(&dir).unwrap();
        let out_path = dir.join("entry.bin");
        let mut remaining = 4u64;

        copy_entry_with_budget(Cursor::new(b"1234".to_vec()), &out_path, &mut remaining).expect("예산과 정확히 같은 크기는 통과합니다");

        assert_eq!(remaining, 0);
        assert_eq!(std::fs::read(&out_path).unwrap(), b"1234");

        let mut remaining = 3u64;
        let error = copy_entry_with_budget(Cursor::new(b"1234".to_vec()), &out_path, &mut remaining)
            .expect_err("예산보다 1바이트라도 크면 실패해야 합니다");
        assert_eq!(localized_key(&error), "error.archive.extractSizeLimitExceeded");

        cleanup(&dir);
    }

    #[cfg(unix)]
    #[test]
    fn 아카이브가_주장하는_실행_권한을_무시하고_고정_모드로_쓴다() {
        let dir = temp_dir("mode");
        let source = write_archive(
            &dir,
            &[("bin/", b""), ("bin/run.sh", b"#!/bin/sh\n")],
            SimpleFileOptions::default().unix_permissions(0o777),
        );
        let dest = dir.join("out");

        extract_hardened_zip(&source, &dest).expect("권한을 마스킹하고 풀어야 합니다");

        assert_eq!(mode_of(&dest.join("bin/run.sh")), HARDENED_FILE_MODE);
        assert_eq!(mode_of(&dest.join("bin")), HARDENED_DIR_MODE);

        cleanup(&dir);
    }

    #[test]
    fn 존재하지_않는_아카이브는_open_실패로_보고된다() {
        let dir = temp_dir("missing");
        let error = extract_hardened_zip(&dir.join("nope.zip"), &dir.join("out")).expect_err("없는 파일은 실패해야 합니다");

        assert_eq!(error.kind(), AppErrorKind::InvalidArgument);
        assert_eq!(localized_key(&error), "error.archive.openFailed");
    }

    #[test]
    fn zip_이_아닌_파일은_unzip_실패로_보고된다() {
        let dir = temp_dir("not-zip");
        std::fs::create_dir_all(&dir).unwrap();
        let source = dir.join("bundle.zip");
        std::fs::write(&source, b"this is not a zip archive").unwrap();

        let error = extract_hardened_zip(&source, &dir.join("out")).expect_err("zip 이 아니면 실패해야 합니다");

        assert_eq!(error.kind(), AppErrorKind::InvalidArgument);
        assert_eq!(localized_key(&error), "error.archive.unzipFailed");

        cleanup(&dir);
    }
}
