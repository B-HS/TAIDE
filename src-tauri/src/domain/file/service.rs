use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::constants::{LARGE_FILE_BYTES, LARGE_FILE_LINES, READ_ONLY_FILE_BYTES, REFUSED_FILE_BYTES};
use crate::domain::project::types::Project;
use crate::error::{AppError, AppResult};
use crate::ids::ProjectId;
use crate::infra::persist;
use crate::paths::AppPaths;

use super::types::{FileSizeTier, OpenedFile};

const BINARY_SNIFF_BYTES: usize = 8_000;
const MIRROR_FILE_SUFFIX: &str = ".json";
const FNV_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;
const MS_PER_SECOND: f64 = 1_000.0;

const LANGUAGE_ID_BY_EXTENSION: &[(&str, &str)] = &[
    ("rs", "rust"),
    ("ts", "typescript"),
    ("tsx", "typescriptreact"),
    ("js", "javascript"),
    ("jsx", "javascriptreact"),
    ("mjs", "javascript"),
    ("cjs", "javascript"),
    ("json", "json"),
    ("jsonc", "jsonc"),
    ("md", "markdown"),
    ("mdx", "markdown"),
    ("toml", "toml"),
    ("yaml", "yaml"),
    ("yml", "yaml"),
    ("html", "html"),
    ("css", "css"),
    ("scss", "scss"),
    ("py", "python"),
    ("go", "go"),
    ("sh", "shellscript"),
    ("bash", "shellscript"),
];
const DEFAULT_LANGUAGE_ID: &str = "plaintext";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MirrorFile {
    path: String,
    content: String,
    saved_at_ms: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MirrorEntry {
    pub path: String,
    pub content: String,
    pub saved_at_ms: f64,
}

pub fn open_file(path: &Path) -> AppResult<OpenedFile> {
    let metadata = std::fs::metadata(path)?;
    if !metadata.is_file() {
        return Err(AppError::InvalidArgument(format!("파일이 아닙니다: {}", path.display())));
    }

    let byte_size = metadata.len();
    let modified_ms = modified_epoch_ms(&metadata)?;
    let language_id = language_id_for_path(path).to_string();
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

pub fn save_file(path: &Path, content: &str) -> AppResult<()> {
    persist::write_atomic(path, content.as_bytes())
}

pub fn create_entry(path: &Path, is_dir: bool) -> AppResult<()> {
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

pub fn delete_entry(path: &Path) -> AppResult<()> {
    let metadata = std::fs::metadata(path)?;
    if metadata.is_dir() {
        Ok(std::fs::remove_dir_all(path)?)
    } else {
        Ok(std::fs::remove_file(path)?)
    }
}

pub fn copy_entry(from: &Path, to: &Path) -> AppResult<()> {
    let metadata = std::fs::metadata(from)?;
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

pub fn mirror_dirty(paths: &AppPaths, project_id: &ProjectId, path: &Path, content: &str) -> AppResult<()> {
    let mirror = MirrorFile {
        path: path.to_string_lossy().to_string(),
        content: content.to_string(),
        saved_at_ms: now_epoch_ms(),
    };
    persist::write_json(&mirror_file_path(paths, project_id, path), &mirror)
}

pub fn clear_mirror(paths: &AppPaths, project_id: &ProjectId, path: &Path) -> AppResult<()> {
    match std::fs::remove_file(mirror_file_path(paths, project_id, path)) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

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
        if let Some(mirror) = persist::read_json::<MirrorFile>(&entry.path())? {
            mirrors.push(MirrorEntry {
                path: mirror.path,
                content: mirror.content,
                saved_at_ms: mirror.saved_at_ms,
            });
        }
    }
    Ok(mirrors)
}

pub fn project_root(projects: &HashMap<ProjectId, Project>, project_id: &ProjectId) -> AppResult<PathBuf> {
    projects
        .get(project_id)
        .map(|project| PathBuf::from(&project.root))
        .ok_or_else(|| AppError::NotFound(format!("project not open: {project_id}")))
}

pub fn resolve_owning_project(projects: &HashMap<ProjectId, Project>, path: &Path) -> AppResult<(ProjectId, PathBuf)> {
    for (project_id, project) in projects {
        if let Ok(resolved) = ensure_within_root(Path::new(&project.root), path) {
            return Ok((project_id.clone(), resolved));
        }
    }
    Err(AppError::InvalidArgument(format!(
        "열린 프로젝트 루트 밖의 경로입니다: {}",
        path.display()
    )))
}

pub fn ensure_within_root(root: &Path, path: &Path) -> AppResult<PathBuf> {
    let canonical_root = std::fs::canonicalize(root)?;
    let resolved = canonicalize_lenient(path)?;

    if resolved.starts_with(&canonical_root) {
        Ok(resolved)
    } else {
        Err(AppError::InvalidArgument(format!(
            "경로가 프로젝트 루트 밖에 있습니다: {}",
            path.display()
        )))
    }
}

fn canonicalize_lenient(path: &Path) -> AppResult<PathBuf> {
    if let Ok(canonical) = std::fs::canonicalize(path) {
        return Ok(canonical);
    }

    let file_name = path
        .file_name()
        .ok_or_else(|| AppError::InvalidArgument(format!("유효하지 않은 경로입니다: {}", path.display())))?;
    let parent = path
        .parent()
        .ok_or_else(|| AppError::InvalidArgument(format!("유효하지 않은 경로입니다: {}", path.display())))?;

    Ok(canonicalize_lenient(parent)?.join(file_name))
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

fn language_id_for_path(path: &Path) -> &'static str {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.to_lowercase())
        .and_then(|extension| {
            LANGUAGE_ID_BY_EXTENSION
                .iter()
                .find(|(key, _)| *key == extension)
                .map(|(_, language_id)| *language_id)
        })
        .unwrap_or(DEFAULT_LANGUAGE_ID)
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

fn now_epoch_ms() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs_f64() * MS_PER_SECOND)
        .unwrap_or(0.0)
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
    fn 작은_파일은_normal_티어다() {
        let dir = temp_dir("normal");
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("main.rs");
        std::fs::write(&file, "fn main() {}\n").unwrap();

        let opened = open_file(&file).expect("open");

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

        let opened = open_file(&file).expect("open");

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

        let opened = open_file(&file).expect("open");

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

        let opened = open_file(&file).expect("open");

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

        let opened = open_file(&file).expect("open");

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

        let opened = open_file(&file).expect("open");

        assert_eq!(opened.tier, FileSizeTier::Refused);
        assert!(opened.content.is_empty());

        cleanup(&dir);
    }

    #[test]
    fn 확장자에_따라_language_id가_매핑된다() {
        assert_eq!(language_id_for_path(Path::new("a.ts")), "typescript");
        assert_eq!(language_id_for_path(Path::new("a.TSX")), "typescriptreact");
        assert_eq!(language_id_for_path(Path::new("a.unknown")), "plaintext");
        assert_eq!(language_id_for_path(Path::new("Makefile")), "plaintext");
    }

    #[test]
    fn 루트_밖_경로는_거부된다() {
        let dir = temp_dir("root-guard");
        let root = dir.join("project");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::create_dir_all(dir.join("outside")).unwrap();

        let inside = root.join("src").join("main.rs");
        let outside = dir.join("outside").join("secret.txt");

        assert!(ensure_within_root(&root, &inside).is_ok());
        assert!(ensure_within_root(&root, &outside).is_err());

        cleanup(&dir);
    }

    #[test]
    fn 존재하지_않는_새_파일_경로도_루트_하위면_허용된다() {
        let dir = temp_dir("root-guard-new");
        let root = dir.join("project");
        std::fs::create_dir_all(&root).unwrap();

        let new_file = root.join("new-folder").join("new-file.rs");

        let resolved = ensure_within_root(&root, &new_file).expect("허용되어야 한다");
        assert!(resolved.starts_with(std::fs::canonicalize(&root).unwrap()));

        cleanup(&dir);
    }

    #[test]
    fn dirty_미러는_저장하고_삭제할_수_있다() {
        let dir = temp_dir("mirror");
        let paths = AppPaths::new(dir.clone());
        let project_id = ProjectId::new();
        let target = PathBuf::from("/workspace/src/main.rs");

        mirror_dirty(&paths, &project_id, &target, "dirty content").expect("mirror");
        let mirrors = list_mirrors(&paths, &project_id).expect("list");

        assert_eq!(mirrors.len(), 1);
        assert_eq!(mirrors[0].path, target.to_string_lossy().to_string());
        assert_eq!(mirrors[0].content, "dirty content");

        clear_mirror(&paths, &project_id, &target).expect("clear");
        let mirrors_after_clear = list_mirrors(&paths, &project_id).expect("list after clear");

        assert!(mirrors_after_clear.is_empty());

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
}
