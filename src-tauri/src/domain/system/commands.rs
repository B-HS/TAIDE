use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use parking_lot::Mutex;
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System};
use tauri::State;

use super::service::{self, file_url, normalize_cpu_percent, ProcessRecord};
use super::types::{AppDataPathKind, SystemUsage, SystemUsageProcess, SystemUsageProcessKind};
use crate::error::{AppError, AppErrorKind, AppResult};
use crate::infra::root_guard;
use crate::state::AppState;

const FALLBACK_CPU_COUNT: usize = 1;
const APP_PROCESS_LABEL: &str = "TAIDE";

pub type SystemUsageLabels = HashMap<u32, (SystemUsageProcessKind, String)>;
pub type SystemUsageLabelProvider = Box<dyn Fn(&tauri::AppHandle) -> SystemUsageLabels + Send + Sync>;

/// The pid → (kind, label) providers [`system_usage_breakdown`] consults to label terminal, agent,
/// and LSP child processes. `lib.rs`'s assembly registers one closure per owning domain
/// (`system_usage_label_providers`) so this domain never reads another domain's store directly
/// (audit R8#9, T1-I §1.4). Providers run in registration order and later entries overwrite
/// earlier ones for the same pid: with the registered terminal → agent → LSP order, an agent
/// process detected inside a terminal's foreground pid set labels as Agent and LSP labels apply
/// last. Each provider reads its own project snapshot, and registration order — not the old
/// hand-coded collection's per-project `HashMap` iteration order — decides every pid collision;
/// see the assembly doc (`lib.rs`) for why that determinization is intentional.
pub struct SystemUsageLabelProviders(Vec<SystemUsageLabelProvider>);

impl SystemUsageLabelProviders {
    pub fn new(providers: Vec<SystemUsageLabelProvider>) -> Self {
        Self(providers)
    }

    pub fn collect(&self, app: &tauri::AppHandle) -> SystemUsageLabels {
        let mut labels = HashMap::new();
        for provider in &self.0 {
            labels.extend(provider(app));
        }
        labels
    }
}

struct AppUsageInner {
    system: System,
    has_previous_sample: bool,
}

struct BreakdownUsageInner {
    system: System,
    known_pids: HashSet<u32>,
}

/// Two independent sysinfo `System` instances, one per command. sysinfo derives a process's
/// CPU delta from the elapsed time since *that `System` instance's* last refresh of that pid;
/// sharing one `System` between `system_usage_get` (app pid only, polled continuously by the
/// status bar) and `system_usage_breakdown` (full process table, polled only while the usage
/// modal is open) let the two independently-phased pollers refresh the app pid within
/// sysinfo's 200ms minimum interval of each other, which collapses the app row's CPU% to near
/// zero (ipc-contract.md §기능 확장 2차 계약 확정 추가).
pub struct SystemUsageStore {
    app: Arc<Mutex<AppUsageInner>>,
    breakdown: Arc<Mutex<BreakdownUsageInner>>,
}

impl SystemUsageStore {
    pub fn new() -> Self {
        Self {
            app: Arc::new(Mutex::new(AppUsageInner {
                system: System::new(),
                has_previous_sample: false,
            })),
            breakdown: Arc::new(Mutex::new(BreakdownUsageInner {
                system: System::new(),
                known_pids: HashSet::new(),
            })),
        }
    }
}

impl Default for SystemUsageStore {
    fn default() -> Self {
        Self::new()
    }
}

fn collect_system_usage(inner: &Mutex<AppUsageInner>) -> AppResult<SystemUsage> {
    let pid = sysinfo::get_current_pid().map_err(|error| AppError::Internal(error.to_string()))?;
    let mut guard = inner.lock();

    guard.system.refresh_processes_specifics(
        ProcessesToUpdate::Some(&[pid]),
        true,
        ProcessRefreshKind::nothing().with_cpu().with_memory(),
    );

    let (memory_bytes, cpu_usage) = {
        let process = guard
            .system
            .process(pid)
            .ok_or_else(|| AppError::Internal("failed to read the TAIDE process info".to_string()))?;
        (process.memory() as f64, process.cpu_usage())
    };

    let cpu_percent = guard.has_previous_sample.then(|| {
        let cpu_count = std::thread::available_parallelism()
            .map(|count| count.get())
            .unwrap_or(FALLBACK_CPU_COUNT);
        normalize_cpu_percent(cpu_usage, cpu_count)
    });
    guard.has_previous_sample = true;

    Ok(SystemUsage { cpu_percent, memory_bytes })
}

#[tauri::command]
#[specta::specta]
pub async fn system_usage_get(store: State<'_, SystemUsageStore>) -> AppResult<SystemUsage> {
    let inner = store.app.clone();
    tauri::async_runtime::spawn_blocking(move || collect_system_usage(&inner))
        .await
        .map_err(|error| AppError::Internal(error.to_string()))?
}

/// Full-process-table sysinfo scan, reduced to what `build_usage_processes` needs. Only ever
/// invoked while the usage breakdown modal is open (query `enabled` gate on the frontend) —
/// refreshed against its own `System` instance (see [`SystemUsageStore`]) so it never shares
/// cpu-delta timing state with `system_usage_get`'s app-only refresh. `known_pids` from the
/// previous breakdown call marks which pids already have a valid cpu delta; a pid missing from
/// it is this `System` instance's first refresh of that process, so it is flagged via
/// `has_previous_cpu_sample: false` rather than trusting sysinfo's meaningless first-sample 0.0.
fn refresh_all_process_records(inner: &Mutex<BreakdownUsageInner>) -> Vec<ProcessRecord> {
    let mut guard = inner.lock();
    guard
        .system
        .refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::nothing().with_cpu().with_memory());

    let records: Vec<ProcessRecord> = guard
        .system
        .processes()
        .values()
        .map(|process| {
            let pid = process.pid().as_u32();
            ProcessRecord {
                pid,
                parent_pid: process.parent().map(|pid| pid.as_u32()),
                name: process.name().to_string_lossy().to_string(),
                cpu_usage: process.cpu_usage(),
                memory: process.memory(),
                has_previous_cpu_sample: guard.known_pids.contains(&pid),
            }
        })
        .collect();

    guard.known_pids = records.iter().map(|record| record.pid).collect();
    records
}

#[tauri::command]
#[specta::specta]
pub async fn system_usage_breakdown(
    app: tauri::AppHandle,
    store: State<'_, SystemUsageStore>,
    providers: State<'_, SystemUsageLabelProviders>,
) -> AppResult<Vec<SystemUsageProcess>> {
    let root_pid = sysinfo::get_current_pid()
        .map_err(|error| AppError::Internal(error.to_string()))?
        .as_u32();

    let domain_labels = providers.collect(&app);

    let cpu_count = std::thread::available_parallelism()
        .map(|count| count.get())
        .unwrap_or(FALLBACK_CPU_COUNT);

    let inner = store.breakdown.clone();
    let records = tauri::async_runtime::spawn_blocking(move || refresh_all_process_records(&inner))
        .await
        .map_err(|error| AppError::Internal(error.to_string()))?;

    Ok(service::build_usage_processes(
        &records,
        root_pid,
        APP_PROCESS_LABEL,
        &domain_labels,
        cpu_count,
    ))
}

/// 열린 프로젝트 루트 안의 경로만 OS 셸로 넘긴다 — opener 플러그인 권한을 열지 않고
/// 이 커맨드를 유일한 통로로 두기 위한 게이트다(ipc-contract §4).
fn resolve_within_open_project(state: &AppState, path: &str) -> AppResult<PathBuf> {
    let projects = state.projects.read().clone();
    let (_, resolved) = root_guard::resolve_owning_project(&projects, Path::new(path))?;
    Ok(resolved)
}

#[tauri::command]
#[specta::specta]
pub async fn system_open_path(state: State<'_, AppState>, path: String) -> AppResult<()> {
    let resolved = resolve_within_open_project(&state, &path)?;
    tauri_plugin_opener::open_path(resolved, None::<&str>).map_err(|error| AppError::Internal(error.to_string()))
}

#[tauri::command]
#[specta::specta]
pub async fn system_reveal_path(state: State<'_, AppState>, path: String) -> AppResult<()> {
    let resolved = resolve_within_open_project(&state, &path)?;
    tauri_plugin_opener::reveal_item_in_dir(resolved).map_err(|error| AppError::Internal(error.to_string()))
}

#[tauri::command]
#[specta::specta]
pub async fn system_open_in_browser(state: State<'_, AppState>, path: String) -> AppResult<()> {
    let resolved = resolve_within_open_project(&state, &path)?;
    tauri_plugin_opener::open_url(file_url(&resolved), None::<&str>).map_err(|error| AppError::Internal(error.to_string()))
}

/// Schemes `system_open_external_url` accepts, checked ASCII-case-insensitively against the
/// (trimmed) start of the URL.
const EXTERNAL_URL_ALLOWED_SCHEMES: &[&str] = &["http://", "https://"];

/// Unicode "format" (Cf category) characters commonly abused to visually spoof a URL's displayed
/// host — bidi overrides/embeddings (e.g. U+202E RIGHT-TO-LEFT OVERRIDE can make `evil.com` render
/// reversed inside an otherwise-trustworthy-looking string), zero-width joiners/spaces that can
/// split a hostname into something a human skims past, and a stray BOM. `char::is_control()` only
/// covers the Cc category and lets every one of these through. This is a fixed denylist of the
/// characters actually used for this kind of spoofing rather than a full Cf-category check — no
/// unicode-category crate is pulled in for it, the same "no new dependency" call
/// `EXTERNAL_URL_ALLOWED_SCHEMES` already makes.
const UNICODE_SPOOFING_CONTROL_CHARS: &[char] = &[
    '\u{200B}', '\u{200C}', '\u{200D}', '\u{200E}', '\u{200F}', '\u{202A}', '\u{202B}', '\u{202C}', '\u{202D}', '\u{202E}', '\u{2060}',
    '\u{2066}', '\u{2067}', '\u{2068}', '\u{2069}', '\u{FEFF}',
];

/// Whitelists `http(s)://` for `tauri_plugin_opener::open_url` — the only caller is a clicked
/// link inside the embedded terminal (xterm's `WebLinksAddon`), which only ever recognizes
/// `http(s)://` text to begin with, so a scheme prefix check alone is enough to keep this from
/// becoming a generic "open anything the OS shell understands" primitive (`file://`,
/// `javascript:`, a custom app-registered scheme, etc. are all rejected). No `url` crate is
/// pulled in for this — a prefix check plus the two checks below are the whole job. Leading/
/// trailing whitespace is trimmed first (a terminal selection commonly carries it); anything left
/// after that — a control character, interior whitespace, or a [`UNICODE_SPOOFING_CONTROL_CHARS`]
/// character — is rejected, since a crafted OSC 8 hyperlink or a language-server-generated string
/// routed here some other way could otherwise smuggle a shell-hostile or visually-spoofed value
/// through to the OS opener. A userinfo segment (`@`) before the host is rejected too — browsers
/// still navigate `https://trusted.example@evil.example/` to `evil.example`, so without this a
/// URL that *displays* a trusted-looking prefix can open a completely different site (the same
/// spoofing pattern `settings::service::is_valid_allowed_host` already rejects for allowed-host
/// entries).
fn validate_external_url(url: &str) -> AppResult<String> {
    let trimmed = url.trim();
    let matched_scheme_len = EXTERNAL_URL_ALLOWED_SCHEMES
        .iter()
        .find(|scheme| {
            trimmed
                .get(..scheme.len())
                .is_some_and(|prefix| prefix.eq_ignore_ascii_case(scheme))
        })
        .map(|scheme| scheme.len());
    let Some(scheme_len) = matched_scheme_len else {
        return Err(AppError::localized(
            AppErrorKind::InvalidArgument,
            "error.system.urlSchemeNotAllowed",
            "only URLs starting with http:// or https:// can be opened",
        ));
    };
    if trimmed
        .chars()
        .any(|c| c.is_control() || c.is_whitespace() || UNICODE_SPOOFING_CONTROL_CHARS.contains(&c))
    {
        return Err(AppError::localized(
            AppErrorKind::InvalidArgument,
            "error.system.urlHasControlChars",
            "the URL cannot contain control characters or whitespace",
        ));
    }
    let authority = &trimmed[scheme_len..];
    let authority_end = authority.find(['/', '?', '#']).unwrap_or(authority.len());
    if authority[..authority_end].contains('@') {
        return Err(AppError::localized(
            AppErrorKind::InvalidArgument,
            "error.system.urlHasUserInfo",
            "the URL cannot contain user info (@) before the host",
        ));
    }
    Ok(trimmed.to_string())
}

#[tauri::command]
#[specta::specta]
pub async fn system_open_external_url(url: String) -> AppResult<()> {
    let validated = validate_external_url(&url)?;
    tauri_plugin_opener::open_url(validated, None::<&str>).map_err(|error| AppError::Internal(error.to_string()))
}

#[tauri::command]
#[specta::specta]
pub async fn system_open_app_data_path(state: State<'_, AppState>, kind: AppDataPathKind) -> AppResult<()> {
    let dir = match kind {
        AppDataPathKind::Plugins => state.paths.plugins_dir(),
        AppDataPathKind::Themes => state.paths.themes_dir(),
        AppDataPathKind::Locales => state.paths.locales_dir(),
        AppDataPathKind::Snippets => state.paths.snippets_dir(),
    };
    std::fs::create_dir_all(&dir)?;
    tauri_plugin_opener::reveal_item_in_dir(dir).map_err(|error| AppError::Internal(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn http_와_https_스킴은_허용된다() {
        assert!(validate_external_url("http://example.com").is_ok());
        assert!(validate_external_url("https://example.com/path?query=1").is_ok());
    }

    #[test]
    fn 대소문자가_섞인_https_스킴도_허용된다() {
        assert!(validate_external_url("HTTPS://example.com").is_ok());
        assert!(validate_external_url("HtTp://example.com").is_ok());
    }

    #[test]
    fn file_스킴은_거부된다() {
        assert!(validate_external_url("file:///etc/passwd").is_err());
    }

    #[test]
    fn 제어_문자가_섞이면_거부된다() {
        assert!(validate_external_url("http://example.com/\u{7}").is_err());
    }

    #[test]
    fn 빈_문자열은_거부된다() {
        assert!(validate_external_url("").is_err());
    }

    #[test]
    fn url_중간의_공백은_거부되지만_전후_공백은_트림된다() {
        assert!(
            validate_external_url("http://example .com").is_err(),
            "URL 중간 공백은 거부되어야 한다"
        );
        assert_eq!(
            validate_external_url("  http://example.com  ").expect("전후 공백은 트림되어 통과해야 한다"),
            "http://example.com"
        );
    }

    #[test]
    fn bidi_override_등_유니코드_시각_위장_문자가_섞이면_거부된다() {
        assert!(
            validate_external_url("https://exa\u{202E}mple.com").is_err(),
            "U+202E RIGHT-TO-LEFT OVERRIDE 는 거부되어야 한다"
        );
        assert!(
            validate_external_url("https://example.com/\u{200B}path").is_err(),
            "U+200B ZERO WIDTH SPACE 는 거부되어야 한다"
        );
        assert!(
            validate_external_url("\u{FEFF}https://example.com").is_err(),
            "U+FEFF BOM 은 거부되어야 한다"
        );
    }

    #[test]
    fn 호스트_앞_userinfo는_거부된다() {
        assert!(
            validate_external_url("https://github.com@evil.example/login").is_err(),
            "userinfo(@) 로 신뢰 도메인을 위장한 URL 은 거부되어야 한다"
        );
        assert!(
            validate_external_url("https://user:pass@evil.example").is_err(),
            "user:pass@ 형태의 userinfo 도 거부되어야 한다"
        );
    }

    #[test]
    fn 경로나_쿼리에_있는_at_기호는_허용된다() {
        assert!(validate_external_url("https://example.com/path@2x.png").is_ok());
        assert!(validate_external_url("https://example.com/search?q=a@b.com").is_ok());
    }
}
