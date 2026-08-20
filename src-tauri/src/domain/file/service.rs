use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::constants::{LARGE_FILE_BYTES, LARGE_FILE_LINES, READ_ONLY_FILE_BYTES, REFUSED_FILE_BYTES};
use crate::error::{AppError, AppResult};
use crate::ids::{ProjectId, TabId};
use crate::infra::clock::{now_epoch_ms, MS_PER_SECOND};
use crate::infra::language::{self, LanguageOverlay};
use crate::infra::persist;
use crate::infra::root_guard;
use crate::paths::AppPaths;
use crate::state::AppState;

use super::types::{FileSizeTier, OpenedFile};

const BINARY_SNIFF_BYTES: usize = 8_000;
const MIRROR_FILE_SUFFIX: &str = ".json";
const FNV_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;
const UNTITLED_MIRROR_DIR_NAME: &str = "untitled";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MirrorFile {
    path: String,
    content: String,
    saved_at_ms: f64,
    #[serde(default)]
    disk_modified_ms: Option<f64>,
}

/// A restorable hot-exit mirror, resolved against the file's *current* disk
/// state at list time. `conflict` is `true` when the disk was modified after
/// the mirror's `disk_modified_ms` baseline was captured, meaning applying
/// the mirror as-is would silently discard an external change.
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct MirrorEntry {
    pub path: String,
    pub content: String,
    pub saved_at_ms: f64,
    pub disk_modified_ms: Option<f64>,
    pub conflict: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct UntitledMirrorFile {
    tab_id: String,
    content: String,
    saved_at_ms: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct UntitledMirrorEntry {
    pub tab_id: TabId,
    pub content: String,
    pub saved_at_ms: f64,
}

pub fn open_file(path: &Path, language_overlays: &[LanguageOverlay]) -> AppResult<OpenedFile> {
    let metadata = std::fs::metadata(path)?;
    if !metadata.is_file() {
        return Err(AppError::InvalidArgument(format!("파일이 아닙니다: {}", path.display())));
    }

    let byte_size = metadata.len();
    let modified_ms = modified_epoch_ms(&metadata)?;
    let language_id = language::language_id_for_path(path, language_overlays);
    let path_string = path.to_string_lossy().to_string();

    if byte_size >= REFUSED_FILE_BYTES {
        return Ok(refused_file(path_string, language_id, byte_size, modified_ms));
    }

    if is_binary(&read_sniff(path)?) {
        return Ok(refused_file(path_string, language_id, byte_size, modified_ms));
    }

    let content = String::from_utf8_lossy(&std::fs::read(path)?).into_owned();
    let line_count = content.lines().count();

    let tier = if byte_size >= READ_ONLY_FILE_BYTES {
        FileSizeTier::ReadOnly
    } else if byte_size >= LARGE_FILE_BYTES || line_count >= LARGE_FILE_LINES {
        FileSizeTier::Large
    } else {
        FileSizeTier::Normal
    };

    Ok(OpenedFile {
        path: path_string,
        content,
        language_id,
        byte_size: saturate_u32(byte_size),
        line_count: saturate_u32(line_count as u64),
        tier,
        read_only: tier == FileSizeTier::ReadOnly,
        modified_ms,
    })
}

/// Private on purpose (R6#2): the raw, guard-free write must not be callable from outside this
/// module — [`save_file_within_open_projects`] is the only path that reaches it, so the type
/// system (not just the boundary whitelist, whose entries are module-granular) blocks any future
/// caller from re-composing a save that skips the root guard or the self-write mark.
fn save_file(path: &Path, content: &str) -> AppResult<()> {
    persist::write_atomic_preserving_mode(path, content.as_bytes())
}

/// The complete guarded save sequence the `file_save` command exposes over IPC — root-guard
/// resolution against the open projects, atomic mode-preserving write, self-write marking (so the
/// watcher does not report the write back as an external change), and hot-exit mirror cleanup.
/// Every save initiated outside the `file` domain (currently `ide_resolve_diff`'s accepted-diff
/// persist) must go through this instead of composing `save_file`/[`clear_mirror`] by hand, so
/// no caller can skip the root guard or the self-write mark (R6#2). A rejection surfaces as
/// [`AppError::Forbidden`](crate::error::AppError) — `root_guard::resolve_owning_project`'s only
/// error shape — which callers may treat as "the owning project is no longer open".
pub fn save_file_within_open_projects(state: &AppState, path: &Path, content: &str) -> AppResult<()> {
    let projects = state.projects.read().clone();
    let (project_id, resolved) = root_guard::resolve_owning_project(&projects, path)?;

    save_file(&resolved, content)?;
    state.self_writes.mark(&resolved);
    clear_mirror(&state.paths, &project_id, &resolved)
}

pub fn create_entry(path: &Path, is_dir: bool) -> AppResult<()> {
    if path.exists() {
        return Err(AppError::InvalidArgument(format!("already exists: {}", path.display())));
    }

    if is_dir {
        return Ok(std::fs::create_dir_all(path)?);
    }

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::OpenOptions::new().write(true).create_new(true).open(path)?;
    Ok(())
}

pub fn rename_entry(from: &Path, to: &Path) -> AppResult<()> {
    if let Some(parent) = to.parent() {
        std::fs::create_dir_all(parent)?;
    }
    Ok(std::fs::rename(from, to)?)
}

#[cfg(target_os = "macos")]
pub fn delete_entry(path: &Path) -> AppResult<()> {
    use trash::macos::{DeleteMethod, TrashContextExtMacos};
    use trash::TrashContext;

    let mut context = TrashContext::default();
    context.set_delete_method(DeleteMethod::NsFileManager);
    context
        .delete_all([path])
        .map_err(|error| AppError::Internal(format!("휴지통으로 이동하지 못했습니다: {error}")))
}

#[cfg(not(target_os = "macos"))]
pub fn delete_entry(path: &Path) -> AppResult<()> {
    trash::delete(path).map_err(|error| AppError::Internal(format!("휴지통으로 이동하지 못했습니다: {error}")))
}

pub fn copy_entry(from: &Path, to: &Path) -> AppResult<()> {
    let metadata = std::fs::metadata(from)?;
    if to == from || to.starts_with(from) {
        return Err(AppError::InvalidArgument(format!(
            "자기 자신 또는 하위 경로로는 복사할 수 없습니다: {}",
            to.display()
        )));
    }
    if metadata.is_dir() {
        copy_dir_recursive(from, to)
    } else {
        if let Some(parent) = to.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::copy(from, to)?;
        Ok(())
    }
}

/// `storage_path` (canonicalized by the caller's `ensure_within_root`) only
/// decides *where* the mirror JSON lives on disk — it must be `display_path`,
/// not `storage_path`, that gets written into the stored `MirrorFile.path`.
/// Restoration (`editor-pane.tsx`'s `entry.path === path`) and pruning
/// (`prune_mirrors`'s `keep_paths` from open tabs) both compare against the
/// tab's original, non-canonicalized path string; storing the resolved path
/// instead would silently desync a symlinked file's mirror from its tab,
/// making it invisible to both restore-matching and prune's keep-list.
///
/// The `disk_modified_ms` baseline is read live, right here, from
/// `storage_path`'s own metadata — never accepted as a caller-supplied value.
/// A caller-supplied baseline (the previous design) could only ever be as
/// fresh as whatever the *frontend* last observed the disk to be, which is
/// stale by construction across any render/effect boundary between "the
/// frontend last learned the file's `modifiedMs`" and "this call actually
/// reaches the backend" — including, concretely, a save's own `onSuccess`
/// landing (and refetching a *newer* `modifiedMs`) while an older-scheduled
/// mirror write from before that save is still in flight. Deriving it here
/// instead ties the baseline to the one instant that actually matters: what
/// the disk looked like at the moment this exact write happened, which is
/// the only baseline `list_mirrors`'s `conflict` check can ever legitimately
/// compare an *external* change against. Returned to the caller so
/// `editor-pane.tsx`'s optimistic `FILE.MIRRORS` cache update can reuse the
/// same authoritative value instead of guessing.
pub fn mirror_dirty(
    paths: &AppPaths,
    project_id: &ProjectId,
    storage_path: &Path,
    display_path: &str,
    content: &str,
) -> AppResult<Option<f64>> {
    let disk_modified_ms = current_modified_ms(storage_path);
    let mirror = MirrorFile {
        path: display_path.to_string(),
        content: content.to_string(),
        saved_at_ms: now_epoch_ms(),
        disk_modified_ms,
    };
    persist::write_json(&mirror_file_path(paths, project_id, storage_path), &mirror)?;
    Ok(disk_modified_ms)
}

pub fn clear_mirror(paths: &AppPaths, project_id: &ProjectId, path: &Path) -> AppResult<()> {
    match std::fs::remove_file(mirror_file_path(paths, project_id, path)) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

/// Lists restorable mirrors. A mirror whose source file no longer exists on
/// disk is skipped rather than surfaced as a ghost restore target — the
/// stale mirror JSON itself is left in place for `prune_mirrors` to reap
/// once the caller knows which paths are still open.
pub fn list_mirrors(paths: &AppPaths, project_id: &ProjectId) -> AppResult<Vec<MirrorEntry>> {
    let dir = paths.buffers_dir(project_id);
    let entries = match std::fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error.into()),
    };

    let mut mirrors = Vec::new();
    for entry in entries {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        let Some(mirror) = persist::read_json::<MirrorFile>(&entry.path())? else {
            continue;
        };
        let Some(current_disk_modified_ms) = current_modified_ms(Path::new(&mirror.path)) else {
            continue;
        };
        let conflict = mirror.disk_modified_ms.is_some_and(|baseline| current_disk_modified_ms > baseline);
        mirrors.push(MirrorEntry {
            path: mirror.path,
            content: mirror.content,
            saved_at_ms: mirror.saved_at_ms,
            disk_modified_ms: mirror.disk_modified_ms,
            conflict,
        });
    }
    Ok(mirrors)
}

/// Deletes every path-mirror not present in `keep_paths` (currently open
/// tabs). Corrupt/unreadable mirror files are also swept, since they can
/// never be restored anyway.
pub fn prune_mirrors(paths: &AppPaths, project_id: &ProjectId, keep_paths: &[String]) -> AppResult<()> {
    let dir = paths.buffers_dir(project_id);
    let entries = match std::fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.into()),
    };

    let keep: std::collections::HashSet<&str> = keep_paths.iter().map(String::as_str).collect();
    for entry in entries {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        let should_remove = match persist::read_json::<MirrorFile>(&entry.path())? {
            Some(mirror) => !keep.contains(mirror.path.as_str()),
            None => true,
        };
        if should_remove {
            std::fs::remove_file(entry.path())?;
        }
    }
    Ok(())
}

pub fn mirror_untitled(paths: &AppPaths, project_id: &ProjectId, tab_id: &TabId, content: &str) -> AppResult<()> {
    let mirror = UntitledMirrorFile {
        tab_id: tab_id.as_str().to_string(),
        content: content.to_string(),
        saved_at_ms: now_epoch_ms(),
    };
    persist::write_json(&untitled_mirror_file_path(paths, project_id, tab_id), &mirror)
}

pub fn clear_untitled_mirror(paths: &AppPaths, project_id: &ProjectId, tab_id: &TabId) -> AppResult<()> {
    match std::fs::remove_file(untitled_mirror_file_path(paths, project_id, tab_id)) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

/// Deletes every untitled-tab mirror not present in `keep_tab_ids`. Untitled
/// tabs are no longer volatile (`layout::service::is_volatile` no longer
/// strips them), so a restored layout's tab ids — active tree plus the
/// closed-tab reopen stack — are the authoritative keep set. This complements
/// `prune_mirrors`, which only walks path-mirrors and explicitly skips the
/// `untitled/` subdirectory; without an equivalent sweep here, an untitled
/// tab closed after a restart is never reaped, because the frontend's
/// same-session `pruneUntitledContents` only diffs against its in-memory
/// registry (empty on a fresh boot) rather than the mirrors Rust actually
/// wrote to disk.
pub fn prune_untitled_mirrors(paths: &AppPaths, project_id: &ProjectId, keep_tab_ids: &[TabId]) -> AppResult<()> {
    let dir = untitled_mirror_dir(paths, project_id);
    let entries = match std::fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.into()),
    };

    let keep: std::collections::HashSet<&str> = keep_tab_ids.iter().map(TabId::as_str).collect();
    for entry in entries {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        let should_remove = match persist::read_json::<UntitledMirrorFile>(&entry.path())? {
            Some(mirror) => !keep.contains(mirror.tab_id.as_str()),
            None => true,
        };
        if should_remove {
            std::fs::remove_file(entry.path())?;
        }
    }
    Ok(())
}

pub fn list_untitled_mirrors(paths: &AppPaths, project_id: &ProjectId) -> AppResult<Vec<UntitledMirrorEntry>> {
    let dir = untitled_mirror_dir(paths, project_id);
    let entries = match std::fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error.into()),
    };

    let mut mirrors = Vec::new();
    for entry in entries {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        if let Some(mirror) = persist::read_json::<UntitledMirrorFile>(&entry.path())? {
            mirrors.push(UntitledMirrorEntry {
                tab_id: TabId::from(mirror.tab_id),
                content: mirror.content,
                saved_at_ms: mirror.saved_at_ms,
            });
        }
    }
    Ok(mirrors)
}

fn copy_dir_recursive(from: &Path, to: &Path) -> AppResult<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_recursive(&entry.path(), &target)?;
        } else {
            std::fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

fn mirror_file_path(paths: &AppPaths, project_id: &ProjectId, path: &Path) -> PathBuf {
    paths
        .buffers_dir(project_id)
        .join(format!("{}{MIRROR_FILE_SUFFIX}", hash_path(&path.to_string_lossy())))
}

/// Untitled tabs have no filesystem path, so they mirror under a dedicated
/// subdirectory keyed by `TabId` instead of a path hash.
fn untitled_mirror_dir(paths: &AppPaths, project_id: &ProjectId) -> PathBuf {
    paths.buffers_dir(project_id).join(UNTITLED_MIRROR_DIR_NAME)
}

fn untitled_mirror_file_path(paths: &AppPaths, project_id: &ProjectId, tab_id: &TabId) -> PathBuf {
    untitled_mirror_dir(paths, project_id).join(format!("{}{MIRROR_FILE_SUFFIX}", tab_id.as_str()))
}

fn current_modified_ms(path: &Path) -> Option<f64> {
    let metadata = std::fs::metadata(path).ok()?;
    modified_epoch_ms(&metadata).ok()
}

fn hash_path(path: &str) -> String {
    let mut hash = FNV_OFFSET_BASIS;
    for byte in path.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    format!("{hash:016x}")
}

fn read_sniff(path: &Path) -> AppResult<Vec<u8>> {
    let mut file = std::fs::File::open(path)?;
    let mut buffer = vec![0u8; BINARY_SNIFF_BYTES];
    let read = file.read(&mut buffer)?;
    buffer.truncate(read);
    Ok(buffer)
}

fn is_binary(sniff: &[u8]) -> bool {
    sniff.contains(&0)
}

fn refused_file(path: String, language_id: String, byte_size: u64, modified_ms: f64) -> OpenedFile {
    OpenedFile {
        path,
        content: String::new(),
        language_id,
        byte_size: saturate_u32(byte_size),
        line_count: 0,
        tier: FileSizeTier::Refused,
        read_only: true,
        modified_ms,
    }
}

fn saturate_u32(value: u64) -> u32 {
    value.min(u64::from(u32::MAX)) as u32
}

fn modified_epoch_ms(metadata: &std::fs::Metadata) -> AppResult<f64> {
    let modified = metadata
        .modified()
        .map_err(|error| AppError::Internal(format!("mtime을 읽을 수 없습니다: {error}")))?;
    let duration = modified
        .duration_since(UNIX_EPOCH)
        .map_err(|error| AppError::Internal(format!("mtime 변환에 실패했습니다: {error}")))?;
    Ok(duration.as_secs_f64() * MS_PER_SECOND)
}

pub fn read_raw(path: &Path) -> AppResult<Vec<u8>> {
    let metadata = std::fs::metadata(path)?;
    if metadata.len() > crate::constants::READ_ONLY_FILE_BYTES {
        return Err(AppError::InvalidArgument(format!(
            "파일이 너무 커서 미리보기를 만들 수 없습니다: {}",
            path.display()
        )));
    }
    Ok(std::fs::read(path)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("taide-file-test-{name}-{}", uuid::Uuid::new_v4()))
    }

    fn cleanup(dir: &Path) {
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn 폴더를_자기_하위로_복사하면_거부한다() {
        let dir = temp_dir("copy-self");
        let source = dir.join("src");
        std::fs::create_dir_all(&source).unwrap();
        std::fs::write(source.join("main.rs"), "fn main() {}\n").unwrap();

        assert!(copy_entry(&source, &source.join("src")).is_err());
        assert!(copy_entry(&source, &source).is_err());
        assert!(!source.join("src").exists());

        cleanup(&dir);
    }

    #[test]
    fn 형제_경로로는_폴더를_복사한다() {
        let dir = temp_dir("copy-sibling");
        let source = dir.join("src");
        std::fs::create_dir_all(&source).unwrap();
        std::fs::write(source.join("main.rs"), "fn main() {}\n").unwrap();

        copy_entry(&source, &dir.join("src copy")).expect("copy");

        assert!(dir.join("src copy").join("main.rs").exists());

        cleanup(&dir);
    }

    #[test]
    fn 작은_파일은_normal_티어다() {
        let dir = temp_dir("normal");
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("main.rs");
        std::fs::write(&file, "fn main() {}\n").unwrap();

        let opened = open_file(&file, &[]).expect("open");

        assert_eq!(opened.tier, FileSizeTier::Normal);
        assert!(!opened.read_only);
        assert_eq!(opened.language_id, "rust");
        assert_eq!(opened.content, "fn main() {}\n");

        cleanup(&dir);
    }

    #[test]
    fn large_file_bytes_이상이면_large_티어다() {
        let dir = temp_dir("large-bytes");
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("big.txt");
        let content = "a".repeat((LARGE_FILE_BYTES) as usize);
        std::fs::write(&file, &content).unwrap();

        let opened = open_file(&file, &[]).expect("open");

        assert_eq!(opened.tier, FileSizeTier::Large);
        assert!(!opened.read_only);

        cleanup(&dir);
    }

    #[test]
    fn large_file_lines_이상이면_large_티어다() {
        let dir = temp_dir("large-lines");
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("many-lines.txt");
        let content = "x\n".repeat(LARGE_FILE_LINES + 1);
        std::fs::write(&file, &content).unwrap();

        let opened = open_file(&file, &[]).expect("open");

        assert_eq!(opened.tier, FileSizeTier::Large);

        cleanup(&dir);
    }

    #[test]
    fn read_only_file_bytes_이상이면_readonly_티어다() {
        let dir = temp_dir("readonly");
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("huge.txt");
        let content = "a".repeat((READ_ONLY_FILE_BYTES) as usize);
        std::fs::write(&file, &content).unwrap();

        let opened = open_file(&file, &[]).expect("open");

        assert_eq!(opened.tier, FileSizeTier::ReadOnly);
        assert!(opened.read_only);

        cleanup(&dir);
    }

    #[test]
    fn refused_file_bytes_이상이면_내용을_읽지_않는다() {
        let dir = temp_dir("refused");
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("massive.bin");
        let handle = std::fs::File::create(&file).unwrap();
        handle.set_len(REFUSED_FILE_BYTES).unwrap();

        let opened = open_file(&file, &[]).expect("open");

        assert_eq!(opened.tier, FileSizeTier::Refused);
        assert!(opened.read_only);
        assert!(opened.content.is_empty());

        cleanup(&dir);
    }

    #[test]
    fn nul_바이트가_있으면_바이너리로_refused_처리된다() {
        let dir = temp_dir("binary");
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("binary.dat");
        std::fs::write(&file, [0x00u8, 0x01, 0x02, 0x03]).unwrap();

        let opened = open_file(&file, &[]).expect("open");

        assert_eq!(opened.tier, FileSizeTier::Refused);
        assert!(opened.content.is_empty());

        cleanup(&dir);
    }

    /// The extension→language mapping itself is covered exhaustively by
    /// `infra::language::language_id_for_path`'s own tests — this only checks that `open_file`
    /// actually wires its result into `OpenedFile::language_id`.
    #[test]
    fn open_file은_language_id를_infra_language에서_가져온다() {
        let dir = temp_dir("language-id");
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("a.ts");
        std::fs::write(&file, "export const x = 1").unwrap();

        let opened = open_file(&file, &[]).expect("open");

        assert_eq!(opened.language_id, "typescript");

        cleanup(&dir);
    }

    #[test]
    fn 이미_존재하는_경로에_생성하면_오류를_반환한다() {
        let dir = temp_dir("create-duplicate");
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("existing.txt");
        std::fs::write(&file, "hello").unwrap();

        let result = create_entry(&file, false);
        assert!(result.is_err());

        let existing_dir = dir.join("existing-dir");
        std::fs::create_dir_all(&existing_dir).unwrap();
        let dir_result = create_entry(&existing_dir, true);
        assert!(dir_result.is_err());

        cleanup(&dir);
    }

    #[test]
    fn 존재하지_않는_경로를_삭제하면_오류를_반환한다() {
        let dir = temp_dir("delete-missing");
        std::fs::create_dir_all(&dir).unwrap();
        let missing = dir.join("does-not-exist.txt");

        let result = delete_entry(&missing);

        assert!(result.is_err());

        cleanup(&dir);
    }

    const CONFLICT_TEST_FUTURE_OFFSET_MS: f64 = 86_400_000.0;

    #[test]
    fn dirty_미러는_저장하고_삭제할_수_있다() {
        let dir = temp_dir("mirror");
        std::fs::create_dir_all(&dir).unwrap();
        let paths = AppPaths::new(dir.clone());
        let project_id = ProjectId::new();
        let target = dir.join("main.rs");
        std::fs::write(&target, "on disk").unwrap();

        mirror_dirty(&paths, &project_id, &target, &target.to_string_lossy(), "dirty content").expect("mirror");
        let mirrors = list_mirrors(&paths, &project_id).expect("list");

        assert_eq!(mirrors.len(), 1);
        assert_eq!(mirrors[0].path, target.to_string_lossy().to_string());
        assert_eq!(mirrors[0].content, "dirty content");
        assert!(!mirrors[0].conflict);

        clear_mirror(&paths, &project_id, &target).expect("clear");
        let mirrors_after_clear = list_mirrors(&paths, &project_id).expect("list after clear");

        assert!(mirrors_after_clear.is_empty());

        cleanup(&dir);
    }

    #[test]
    fn mirror_dirty는_호출_시점의_실제_디스크_mtime을_baseline으로_반환한다() {
        let dir = temp_dir("mirror-live-baseline");
        std::fs::create_dir_all(&dir).unwrap();
        let paths = AppPaths::new(dir.clone());
        let project_id = ProjectId::new();
        let target = dir.join("live.rs");
        std::fs::write(&target, "on disk").unwrap();
        let expected = current_modified_ms(&target);

        let returned = mirror_dirty(&paths, &project_id, &target, &target.to_string_lossy(), "dirty content").expect("mirror");

        assert_eq!(
            returned, expected,
            "mirror_dirty가 반환하는 baseline은 호출자가 넘긴 값이 아니라 호출 시점의 실제 디스크 mtime이어야 한다"
        );
        let mirrors = list_mirrors(&paths, &project_id).expect("list");
        assert!(
            !mirrors[0].conflict,
            "방금 읽은 baseline과 현재 디스크 상태가 같으므로 충돌이 아니어야 한다"
        );

        cleanup(&dir);
    }

    /// `list_mirrors`'s conflict comparison (`current_disk_modified_ms > baseline`) is exercised
    /// directly against a hand-written `MirrorFile` rather than through `mirror_dirty` — since
    /// `mirror_dirty` now derives its baseline live from disk (see its own doc comment), it can no
    /// longer be handed an artificial one, but the comparison it feeds still needs coverage for both
    /// directions independent of what wrote the mirror.
    #[test]
    fn 디스크가_baseline보다_새로우면_충돌로_판정된다() {
        let dir = temp_dir("mirror-conflict");
        std::fs::create_dir_all(&dir).unwrap();
        let paths = AppPaths::new(dir.clone());
        let project_id = ProjectId::new();
        let target = dir.join("conflict.rs");
        std::fs::write(&target, "changed on disk").unwrap();

        let mirror = MirrorFile {
            path: target.to_string_lossy().to_string(),
            content: "mirrored content".to_string(),
            saved_at_ms: now_epoch_ms(),
            disk_modified_ms: Some(0.0),
        };
        persist::write_json(&mirror_file_path(&paths, &project_id, &target), &mirror).expect("write mirror directly");
        let mirrors = list_mirrors(&paths, &project_id).expect("list");

        assert_eq!(mirrors.len(), 1);
        assert!(mirrors[0].conflict);

        cleanup(&dir);
    }

    #[test]
    fn 디스크가_baseline보다_새롭지_않으면_충돌이_아니다() {
        let dir = temp_dir("mirror-no-conflict");
        std::fs::create_dir_all(&dir).unwrap();
        let paths = AppPaths::new(dir.clone());
        let project_id = ProjectId::new();
        let target = dir.join("no-conflict.rs");
        std::fs::write(&target, "unchanged on disk").unwrap();

        let baseline = now_epoch_ms() + CONFLICT_TEST_FUTURE_OFFSET_MS;
        let mirror = MirrorFile {
            path: target.to_string_lossy().to_string(),
            content: "mirrored content".to_string(),
            saved_at_ms: now_epoch_ms(),
            disk_modified_ms: Some(baseline),
        };
        persist::write_json(&mirror_file_path(&paths, &project_id, &target), &mirror).expect("write mirror directly");
        let mirrors = list_mirrors(&paths, &project_id).expect("list");

        assert_eq!(mirrors.len(), 1);
        assert!(!mirrors[0].conflict);

        cleanup(&dir);
    }

    #[test]
    fn 디스크에서_사라진_미러_항목은_목록에서_제외된다() {
        let dir = temp_dir("mirror-missing-disk");
        std::fs::create_dir_all(&dir).unwrap();
        let paths = AppPaths::new(dir.clone());
        let project_id = ProjectId::new();
        let target = dir.join("gone.rs");
        std::fs::write(&target, "will be deleted").unwrap();

        mirror_dirty(&paths, &project_id, &target, &target.to_string_lossy(), "mirrored content").expect("mirror");
        std::fs::remove_file(&target).unwrap();

        let mirrors = list_mirrors(&paths, &project_id).expect("list");

        assert!(mirrors.is_empty());

        cleanup(&dir);
    }

    #[test]
    fn prune_mirrors는_keep_paths에_없는_미러를_삭제한다() {
        let dir = temp_dir("mirror-prune");
        std::fs::create_dir_all(&dir).unwrap();
        let paths = AppPaths::new(dir.clone());
        let project_id = ProjectId::new();
        let keep = dir.join("keep.rs");
        let drop = dir.join("drop.rs");
        std::fs::write(&keep, "keep").unwrap();
        std::fs::write(&drop, "drop").unwrap();

        mirror_dirty(&paths, &project_id, &keep, &keep.to_string_lossy(), "keep content").expect("mirror keep");
        mirror_dirty(&paths, &project_id, &drop, &drop.to_string_lossy(), "drop content").expect("mirror drop");

        prune_mirrors(&paths, &project_id, &[keep.to_string_lossy().to_string()]).expect("prune");
        let mirrors = list_mirrors(&paths, &project_id).expect("list");

        assert_eq!(mirrors.len(), 1);
        assert_eq!(mirrors[0].path, keep.to_string_lossy().to_string());

        cleanup(&dir);
    }

    #[cfg(unix)]
    #[test]
    fn 심볼릭_링크_경로의_미러는_원본_탭_경로로_저장되어_prune에서_살아남는다() {
        let dir = temp_dir("mirror-symlink");
        std::fs::create_dir_all(&dir).unwrap();
        let paths = AppPaths::new(dir.clone());
        let project_id = ProjectId::new();
        let real = dir.join("real.rs");
        let link = dir.join("link.rs");
        std::fs::write(&real, "on disk").unwrap();
        std::os::unix::fs::symlink(&real, &link).unwrap();

        let storage_path = std::fs::canonicalize(&link).expect("canonicalize symlink");
        let display_path = link.to_string_lossy().to_string();

        mirror_dirty(&paths, &project_id, &storage_path, &display_path, "dirty content").expect("mirror");
        let mirrors = list_mirrors(&paths, &project_id).expect("list");

        assert_eq!(mirrors.len(), 1);
        assert_eq!(
            mirrors[0].path, display_path,
            "저장된 path는 canonicalize된 실경로가 아니라 탭의 원래 경로여야 한다"
        );

        prune_mirrors(&paths, &project_id, std::slice::from_ref(&display_path)).expect("prune with tab's symlink path as keep");
        let mirrors_after_prune = list_mirrors(&paths, &project_id).expect("list after prune");

        assert_eq!(
            mirrors_after_prune.len(),
            1,
            "탭 경로(심볼릭 링크)가 keep_paths에 있으므로 살아남아야 한다"
        );

        cleanup(&dir);
    }

    #[test]
    fn untitled_미러는_탭id로_저장하고_삭제할_수_있다() {
        let dir = temp_dir("untitled-mirror");
        let paths = AppPaths::new(dir.clone());
        let project_id = ProjectId::new();
        let tab_id = TabId::new();

        mirror_untitled(&paths, &project_id, &tab_id, "draft content").expect("mirror untitled");
        let mirrors = list_untitled_mirrors(&paths, &project_id).expect("list");

        assert_eq!(mirrors.len(), 1);
        assert_eq!(mirrors[0].tab_id, tab_id);
        assert_eq!(mirrors[0].content, "draft content");

        clear_untitled_mirror(&paths, &project_id, &tab_id).expect("clear");
        let mirrors_after_clear = list_untitled_mirrors(&paths, &project_id).expect("list after clear");

        assert!(mirrors_after_clear.is_empty());

        cleanup(&dir);
    }

    #[test]
    fn prune_untitled_mirrors는_keep_tab_ids에_없는_미러를_삭제한다() {
        let dir = temp_dir("untitled-mirror-prune");
        let paths = AppPaths::new(dir.clone());
        let project_id = ProjectId::new();
        let keep_tab_id = TabId::new();
        let drop_tab_id = TabId::new();

        mirror_untitled(&paths, &project_id, &keep_tab_id, "keep content").expect("mirror keep");
        mirror_untitled(&paths, &project_id, &drop_tab_id, "drop content").expect("mirror drop");

        prune_untitled_mirrors(&paths, &project_id, std::slice::from_ref(&keep_tab_id)).expect("prune");
        let mirrors = list_untitled_mirrors(&paths, &project_id).expect("list");

        assert_eq!(mirrors.len(), 1);
        assert_eq!(mirrors[0].tab_id, keep_tab_id);

        cleanup(&dir);
    }

    #[test]
    fn 존재하지_않는_미러_삭제는_오류가_아니다() {
        let dir = temp_dir("mirror-missing");
        let paths = AppPaths::new(dir.clone());
        let project_id = ProjectId::new();

        let result = clear_mirror(&paths, &project_id, &PathBuf::from("/workspace/none.rs"));

        assert!(result.is_ok());

        cleanup(&dir);
    }

    /// R6#2 회귀: 도메인 밖 호출자(`ide_resolve_diff`)가 공유하는 가드 저장 경로가 `file_save`
    /// 커맨드와 동일하게 (1) 열린 프로젝트 루트 밖 경로를 `Forbidden` 으로 거부하고, (2) 루트 안
    /// 경로는 저장 후 hot-exit 미러까지 정리하는지를 고정한다 — 이 함수를 우회해 `save_file` 을
    /// 직접 조합하면 루트 가드·self-write 마크·미러 정리가 조용히 빠진다.
    #[test]
    fn save_file_within_open_projects는_루트_가드와_미러_정리를_함께_수행한다() {
        let dir = temp_dir("guarded-save");
        let root = dir.join("project");
        std::fs::create_dir_all(&root).unwrap();
        let target = root.join("a.txt");
        std::fs::write(&target, "old").unwrap();

        let state = crate::state::AppState::new(AppPaths::new(dir.join("data")));
        let project_id = ProjectId::new();
        state.projects.write().insert(
            project_id.clone(),
            crate::domain::project::types::Project {
                id: project_id.clone(),
                root: root.to_string_lossy().into_owned(),
                name: "project".to_string(),
                capabilities: Vec::new(),
                root_missing: false,
                last_opened_at: 0.0,
            },
        );

        let outside = dir.join("outside.txt");
        std::fs::write(&outside, "x").unwrap();
        assert!(
            matches!(save_file_within_open_projects(&state, &outside, "new"), Err(AppError::Forbidden(_))),
            "열린 프로젝트 루트 밖 경로는 Forbidden 으로 거부되어야 한다"
        );
        assert_eq!(std::fs::read_to_string(&outside).unwrap(), "x", "거부된 경로에는 쓰지 않아야 한다");

        let resolved = std::fs::canonicalize(&target).unwrap();
        mirror_dirty(&state.paths, &project_id, &resolved, &target.to_string_lossy(), "dirty").unwrap();
        assert_eq!(list_mirrors(&state.paths, &project_id).unwrap().len(), 1);

        save_file_within_open_projects(&state, &target, "new content").expect("루트 안 경로는 저장되어야 한다");

        assert_eq!(std::fs::read_to_string(&target).unwrap(), "new content");
        assert!(
            list_mirrors(&state.paths, &project_id).unwrap().is_empty(),
            "저장 후 hot-exit 미러가 정리되어야 한다"
        );

        cleanup(&dir);
    }
}
