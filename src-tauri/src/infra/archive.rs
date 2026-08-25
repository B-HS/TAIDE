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
