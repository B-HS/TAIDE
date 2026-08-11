use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use sha2::{Digest, Sha256};

use crate::error::{AppError, AppResult};

const PROGRESS_EMIT_MIN_INTERVAL_MS: u128 = 100;
const PROGRESS_EMIT_MIN_BYTES: u64 = 256 * 1024;

pub struct DownloadProgress {
    pub received_bytes: u64,
    pub total_bytes: Option<u64>,
}

pub struct DownloadedFile {
    pub path: PathBuf,
    pub sha256: String,
    pub total_bytes: u64,
}

pub fn platform_key() -> String {
    format!("{}-{}", platform_os(), platform_arch())
}

fn platform_os() -> &'static str {
    match std::env::consts::OS {
        "macos" => "darwin",
        other => other,
    }
}

fn platform_arch() -> &'static str {
    std::env::consts::ARCH
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

pub fn verify_sha256(bytes: &[u8], expected_hex: &str) -> bool {
    sha256_hex(bytes).eq_ignore_ascii_case(expected_hex.trim())
}

pub fn hashes_match(actual_hex: &str, expected_hex: &str) -> bool {
    actual_hex.eq_ignore_ascii_case(expected_hex.trim())
}

fn set_executable(path: &Path) -> AppResult<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))?;
    }
    #[cfg(not(unix))]
    {
        let _ = path;
    }
    Ok(())
}

pub fn extract_tar_gz(source_path: &Path, dest: &Path) -> AppResult<()> {
    let file = std::fs::File::open(source_path)?;
    let decoder = flate2::read::GzDecoder::new(file);
    let mut archive = tar::Archive::new(decoder);
    archive
        .unpack(dest)
        .map_err(|error| AppError::Internal(format!("tar.gz 해제 실패: {error}")))
}

pub fn extract_zip(source_path: &Path, dest: &Path) -> AppResult<()> {
    let file = std::fs::File::open(source_path)?;
    let mut archive = zip::ZipArchive::new(file).map_err(|error| AppError::Internal(format!("zip 열기 실패: {error}")))?;

    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|error| AppError::Internal(format!("zip 항목 읽기 실패: {error}")))?;
        let Some(relative_path) = entry.enclosed_name() else {
            continue;
        };
        let out_path = dest.join(relative_path);

        if entry.is_dir() {
            std::fs::create_dir_all(&out_path)?;
            continue;
        }

        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut out_file = std::fs::File::create(&out_path)?;
        std::io::copy(&mut entry, &mut out_file)?;

        #[cfg(unix)]
        if let Some(mode) = entry.unix_mode() {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&out_path, std::fs::Permissions::from_mode(mode))?;
        }
    }

    Ok(())
}

pub fn write_binary_from_file(source_path: &Path, dest: &Path, bin_name: Option<&str>) -> AppResult<()> {
    std::fs::create_dir_all(dest)?;
    let out_path = dest.join(bin_name.unwrap_or("server"));
    std::fs::copy(source_path, &out_path)?;
    set_executable(&out_path)
}

pub fn write_gz_binary_from_file(source_path: &Path, dest: &Path, bin_name: Option<&str>) -> AppResult<()> {
    std::fs::create_dir_all(dest)?;
    let out_path = dest.join(bin_name.unwrap_or("server"));
    let file = std::fs::File::open(source_path)?;
    let mut decoder = flate2::read::GzDecoder::new(file);
    let mut out_file = std::fs::File::create(&out_path)?;
    std::io::copy(&mut decoder, &mut out_file).map_err(|error| AppError::Internal(format!("gz 압축 해제 실패: {error}")))?;
    set_executable(&out_path)
}

pub fn atomic_install(temp_dir: &Path, final_dir: &Path) -> AppResult<()> {
    if let Some(parent) = final_dir.parent() {
        std::fs::create_dir_all(parent)?;
    }
    if final_dir.exists() {
        std::fs::remove_dir_all(final_dir)?;
    }
    std::fs::rename(temp_dir, final_dir).map_err(AppError::from)
}

/// Compares two version-like strings ("3.19.0", "1.60.0-202606262232") segment by segment,
/// splitting on `.`/`-`. Numeric segments compare numerically (so "3.19.0" > "3.9.0"); a plain
/// lexicographic `str::cmp` would get that backwards. Non-numeric segments fall back to string
/// comparison so arbitrary directory names still sort deterministically.
fn compare_version_segments(a: &str, b: &str) -> std::cmp::Ordering {
    use std::cmp::Ordering;

    let a_segments: Vec<&str> = a.split(['.', '-']).collect();
    let b_segments: Vec<&str> = b.split(['.', '-']).collect();

    for (a_segment, b_segment) in a_segments.iter().zip(b_segments.iter()) {
        let ordering = match (a_segment.parse::<u64>(), b_segment.parse::<u64>()) {
            (Ok(a_num), Ok(b_num)) => a_num.cmp(&b_num),
            _ => a_segment.cmp(b_segment),
        };
        if ordering != Ordering::Equal {
            return ordering;
        }
    }

    a_segments.len().cmp(&b_segments.len())
}

pub fn installed_versions(lsp_dir: &Path, server_id: &str) -> Vec<String> {
    let server_dir = lsp_dir.join(server_id);
    let Ok(entries) = std::fs::read_dir(&server_dir) else {
        return Vec::new();
    };

    let mut versions: Vec<String> = entries
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().is_dir())
        .filter_map(|entry| entry.file_name().to_str().map(str::to_string))
        .collect();
    versions.sort_by(|a, b| compare_version_segments(a, b));
    versions
}

pub fn latest_installed_version(lsp_dir: &Path, server_id: &str) -> Option<String> {
    installed_versions(lsp_dir, server_id).into_iter().next_back()
}

pub fn substitute_template_args(args: &[String], vars: &[(&str, String)]) -> Vec<String> {
    args.iter()
        .map(|arg| {
            vars.iter()
                .fold(arg.clone(), |acc, (key, value)| acc.replace(&format!("{{{key}}}"), value))
        })
        .collect()
}

/// Downloads `url` straight to `dest_path` while hashing it incrementally, instead of buffering
/// the whole archive in memory (some LSP archives run to several hundred MB — see kotlin-lsp).
/// Progress callbacks are throttled by time/byte-delta so a fast connection doesn't flood the
/// frontend with an IPC event per network chunk.
pub async fn download_to_file<F: FnMut(DownloadProgress)>(
    client: &reqwest::Client,
    url: &str,
    dest_path: &Path,
    cancel: &AtomicBool,
    mut on_progress: F,
) -> AppResult<DownloadedFile> {
    use futures_util::StreamExt;
    use tokio::io::AsyncWriteExt;

    let response = client
        .get(url)
        .send()
        .await
        .map_err(|error| AppError::Internal(format!("다운로드 요청 실패: {error}")))?;
    if !response.status().is_success() {
        return Err(AppError::Internal(format!("다운로드 실패: HTTP {}", response.status())));
    }

    let total_bytes = response.content_length();
    let mut stream = response.bytes_stream();

    if let Some(parent) = dest_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut file = tokio::fs::File::create(dest_path).await?;
    let mut hasher = Sha256::new();
    let mut received: u64 = 0;
    let mut last_emit = std::time::Instant::now();
    let mut last_emitted_bytes: u64 = 0;

    while let Some(chunk) = stream.next().await {
        if cancel.load(Ordering::SeqCst) {
            drop(file);
            std::fs::remove_file(dest_path).ok();
            return Err(AppError::Internal("설치가 취소되었습니다".to_string()));
        }

        let chunk = chunk.map_err(|error| AppError::Internal(format!("다운로드 스트림 오류: {error}")))?;
        hasher.update(&chunk);
        if let Err(error) = file.write_all(&chunk).await {
            drop(file);
            std::fs::remove_file(dest_path).ok();
            return Err(AppError::from(error));
        }
        received += chunk.len() as u64;

        let bytes_delta = received - last_emitted_bytes;
        if last_emit.elapsed().as_millis() >= PROGRESS_EMIT_MIN_INTERVAL_MS || bytes_delta >= PROGRESS_EMIT_MIN_BYTES {
            on_progress(DownloadProgress {
                received_bytes: received,
                total_bytes,
            });
            last_emit = std::time::Instant::now();
            last_emitted_bytes = received;
        }
    }

    file.flush().await?;
    on_progress(DownloadProgress {
        received_bytes: received,
        total_bytes: Some(received),
    });

    Ok(DownloadedFile {
        path: dest_path.to_path_buf(),
        sha256: format!("{:x}", hasher.finalize()),
        total_bytes: received,
    })
}

pub fn temp_install_dir(lsp_dir: &Path, server_id: &str) -> PathBuf {
    lsp_dir.join(".tmp").join(format!("{server_id}-{}", uuid::Uuid::new_v4()))
}

pub fn temp_download_path(lsp_dir: &Path, server_id: &str) -> PathBuf {
    lsp_dir.join(".tmp").join(format!("{server_id}-{}.download", uuid::Uuid::new_v4()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("taide-lsp-install-test-{name}-{}", uuid::Uuid::new_v4()))
    }

    #[test]
    fn sha256_해시가_일치하면_검증을_통과한다() {
        let bytes = b"hello world";
        let expected = sha256_hex(bytes);

        assert!(verify_sha256(bytes, &expected));
        assert!(!verify_sha256(
            bytes,
            "0000000000000000000000000000000000000000000000000000000000000000"
        ));
    }

    #[test]
    fn hashes_match은_대소문자를_구분하지_않고_비교한다() {
        assert!(hashes_match("ABCDEF", "abcdef"));
        assert!(!hashes_match("abcdef", "123456"));
    }

    #[test]
    fn sha256_대소문자를_구분하지_않는다() {
        let bytes = b"hello world";
        let expected = sha256_hex(bytes).to_uppercase();

        assert!(verify_sha256(bytes, &expected));
    }

    #[test]
    fn tar_gz_아카이브를_해제한다() {
        use std::io::Write;

        let src = temp_dir("targz-src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("bin"), b"binary-content").unwrap();

        let mut tar_bytes = Vec::new();
        {
            let mut builder = tar::Builder::new(&mut tar_bytes);
            builder.append_dir_all(".", &src).unwrap();
            builder.finish().unwrap();
        }

        let mut gz_bytes = Vec::new();
        {
            let mut encoder = flate2::write::GzEncoder::new(&mut gz_bytes, flate2::Compression::default());
            encoder.write_all(&tar_bytes).unwrap();
            encoder.finish().unwrap();
        }

        let archive_path = temp_dir("targz-archive.tar.gz");
        std::fs::write(&archive_path, &gz_bytes).unwrap();

        let dest = temp_dir("targz-dest");
        extract_tar_gz(&archive_path, &dest).unwrap();

        assert_eq!(std::fs::read(dest.join("bin")).unwrap(), b"binary-content");

        std::fs::remove_dir_all(&src).ok();
        std::fs::remove_file(&archive_path).ok();
        std::fs::remove_dir_all(&dest).ok();
    }

    #[test]
    fn gz_단일_바이너리를_해제한다() {
        use std::io::Write;

        let mut gz_bytes = Vec::new();
        {
            let mut encoder = flate2::write::GzEncoder::new(&mut gz_bytes, flate2::Compression::default());
            encoder.write_all(b"gz-binary-content").unwrap();
            encoder.finish().unwrap();
        }

        let archive_path = temp_dir("gz-archive.gz");
        std::fs::write(&archive_path, &gz_bytes).unwrap();

        let dest = temp_dir("gz-dest");
        write_gz_binary_from_file(&archive_path, &dest, Some("bin")).unwrap();

        assert_eq!(std::fs::read(dest.join("bin")).unwrap(), b"gz-binary-content");

        std::fs::remove_file(&archive_path).ok();
        std::fs::remove_dir_all(&dest).ok();
    }

    #[test]
    fn zip_아카이브를_해제한다() {
        use std::io::Write;
        use zip::write::SimpleFileOptions;

        let mut zip_bytes: Vec<u8> = Vec::new();
        {
            let cursor = std::io::Cursor::new(&mut zip_bytes);
            let mut writer = zip::ZipWriter::new(cursor);
            writer.start_file("nested/bin", SimpleFileOptions::default()).unwrap();
            writer.write_all(b"zip-binary-content").unwrap();
            writer.finish().unwrap();
        }

        let archive_path = temp_dir("zip-archive.zip");
        std::fs::write(&archive_path, &zip_bytes).unwrap();

        let dest = temp_dir("zip-dest");
        extract_zip(&archive_path, &dest).unwrap();

        assert_eq!(std::fs::read(dest.join("nested").join("bin")).unwrap(), b"zip-binary-content");

        std::fs::remove_file(&archive_path).ok();
        std::fs::remove_dir_all(&dest).ok();
    }

    #[test]
    fn binary_아카이브는_원본_파일을_그대로_복사한다() {
        let archive_path = temp_dir("binary-archive");
        std::fs::write(&archive_path, b"raw-binary-content").unwrap();

        let dest = temp_dir("binary-dest");
        write_binary_from_file(&archive_path, &dest, Some("server-bin")).unwrap();

        assert_eq!(std::fs::read(dest.join("server-bin")).unwrap(), b"raw-binary-content");

        std::fs::remove_file(&archive_path).ok();
        std::fs::remove_dir_all(&dest).ok();
    }

    #[test]
    fn 원자적_설치는_임시_디렉토리를_최종_경로로_옮긴다() {
        let temp = temp_dir("atomic-temp");
        std::fs::create_dir_all(&temp).unwrap();
        std::fs::write(temp.join("marker"), b"ok").unwrap();

        let final_dir = temp_dir("atomic-final");
        atomic_install(&temp, &final_dir).unwrap();

        assert!(final_dir.join("marker").exists());
        assert!(!temp.exists());

        std::fs::remove_dir_all(&final_dir).ok();
    }

    #[test]
    fn 원자적_설치는_기존_최종_경로를_덮어쓴다() {
        let final_dir = temp_dir("atomic-overwrite-final");
        std::fs::create_dir_all(&final_dir).unwrap();
        std::fs::write(final_dir.join("old"), b"old").unwrap();

        let temp = temp_dir("atomic-overwrite-temp");
        std::fs::create_dir_all(&temp).unwrap();
        std::fs::write(temp.join("new"), b"new").unwrap();

        atomic_install(&temp, &final_dir).unwrap();

        assert!(!final_dir.join("old").exists());
        assert!(final_dir.join("new").exists());

        std::fs::remove_dir_all(&final_dir).ok();
    }

    #[test]
    fn 설치된_버전_목록을_오름차순으로_반환한다() {
        let lsp_dir = temp_dir("versions");
        std::fs::create_dir_all(lsp_dir.join("marksman").join("1.2.0")).unwrap();
        std::fs::create_dir_all(lsp_dir.join("marksman").join("1.1.0")).unwrap();
        std::fs::write(lsp_dir.join("marksman").join("not-a-dir"), b"x").unwrap();

        let versions = installed_versions(&lsp_dir, "marksman");
        assert_eq!(versions, vec!["1.1.0".to_string(), "1.2.0".to_string()]);
        assert_eq!(latest_installed_version(&lsp_dir, "marksman"), Some("1.2.0".to_string()));

        std::fs::remove_dir_all(&lsp_dir).ok();
    }

    #[test]
    fn 설치_기록이_없으면_빈_목록을_반환한다() {
        let lsp_dir = temp_dir("versions-empty");
        assert!(installed_versions(&lsp_dir, "unknown").is_empty());
        assert_eq!(latest_installed_version(&lsp_dir, "unknown"), None);
    }

    #[test]
    fn 버전_정렬은_사전식이_아니라_숫자_세그먼트_기준이다() {
        let lsp_dir = temp_dir("versions-semver");
        for version in ["3.9.0", "3.19.0", "0.9.0", "0.10.0", "9.0.0", "22.1.6"] {
            std::fs::create_dir_all(lsp_dir.join("srv").join(version)).unwrap();
        }

        let versions = installed_versions(&lsp_dir, "srv");
        assert_eq!(
            versions,
            vec![
                "0.9.0".to_string(),
                "0.10.0".to_string(),
                "3.9.0".to_string(),
                "3.19.0".to_string(),
                "9.0.0".to_string(),
                "22.1.6".to_string(),
            ]
        );
        assert_eq!(latest_installed_version(&lsp_dir, "srv"), Some("22.1.6".to_string()));

        std::fs::remove_dir_all(&lsp_dir).ok();
    }

    #[test]
    fn 템플릿_변수를_치환한다() {
        let args = vec![
            "-data".to_string(),
            "{workspaceDir}/.jdtls".to_string(),
            "-jdk".to_string(),
            "{jdkPath}".to_string(),
        ];
        let vars = vec![("workspaceDir", "/proj".to_string()), ("jdkPath", "/usr/lib/jvm/17".to_string())];

        let substituted = substitute_template_args(&args, &vars);

        assert_eq!(substituted[1], "/proj/.jdtls");
        assert_eq!(substituted[3], "/usr/lib/jvm/17");
    }

    #[test]
    fn 매칭되는_변수가_없으면_인자를_그대로_둔다() {
        let args = vec!["--stdio".to_string()];
        let substituted = substitute_template_args(&args, &[("workspaceDir", "/proj".to_string())]);

        assert_eq!(substituted, args);
    }

    #[test]
    fn 플랫폼_키는_os와_아키텍처를_조합한다() {
        let key = platform_key();
        assert!(key.contains('-'));
    }
}
