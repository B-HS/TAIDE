use std::path::{Path, PathBuf};

use taide_infra::{
    archive, clock, crypto, external_url, home, http, language, lsp_install, persist, range_file, redact, root_guard, self_write,
    shell_quote, watch_policy, watcher,
};
use taide_lib::{constants, infra};
use taide_model::error::{AppErrorKind, AppResult};
use taide_model::file::{FsChange, FsChangeKind};

#[test]
fn 독립_infra_타입과_기존_facade가_같다() {
    let _: Option<infra::language::LanguageOverlay> = None::<language::LanguageOverlay>;
    assert_eq!(infra::clock::MS_PER_SECOND, clock::MS_PER_SECOND);
}

#[test]
fn 독립_infra_도구와_기존_경로가_같은_결과를_낸다() {
    assert!(crypto::constant_time_eq(b"same", b"same"));
    assert_eq!(
        infra::crypto::constant_time_eq(b"same", b"other"),
        crypto::constant_time_eq(b"same", b"other")
    );
    assert_eq!(
        infra::home::expand_home("~/repo", Some("/home/user")),
        home::expand_home("~/repo", Some("/home/user"))
    );
    assert_eq!(infra::shell_quote::posix_quote("a'b"), shell_quote::posix_quote("a'b"));
    assert_eq!(
        infra::language::language_id_for_path(Path::new("main.rs"), &[]),
        language::language_id_for_path(Path::new("main.rs"), &[])
    );
    assert_eq!(
        infra::redact::mask_provider_error("request failed"),
        redact::mask_provider_error("request failed")
    );
}

#[test]
fn self_write_추적기는_기존_경로와_같고_한_배치에서만_마킹을_소비한다() {
    let tracker = self_write::SelfWriteTracker::new();
    let _: &infra::self_write::SelfWriteTracker = &tracker;
    tracker.mark(Path::new("/repo/a.rs"));

    let change = FsChange {
        kind: FsChangeKind::Modified,
        paths: vec!["/repo/a.rs".to_string()],
        from_app: false,
    };
    let first = infra::self_write::resolve_from_app(&tracker, vec![change.clone(), change.clone()]);
    assert!(first.iter().all(|item| item.from_app));

    let second = self_write::resolve_from_app(&tracker, vec![change]);
    assert!(!second[0].from_app);
}

#[test]
fn root_guard_공개_경로는_같은_안전_컴포넌트_정책을_쓴다() {
    let _: fn(&str) -> AppResult<()> = infra::root_guard::ensure_safe_component;
    assert!(root_guard::ensure_safe_component("safe-name").is_ok());

    let old = infra::root_guard::ensure_safe_component("../escape").unwrap_err();
    let new = root_guard::ensure_safe_component("../escape").unwrap_err();
    assert_eq!(old.kind(), new.kind());
}

#[test]
fn persist_공개_경로는_같은_임시_파일_형식을_판별한다() {
    let _: fn(&Path, &[u8]) -> AppResult<()> = infra::persist::write_atomic;
    let sibling = Path::new("/repo/.main.rs.00000000-0000-0000-0000-000000000000.tmp");
    let unrelated = Path::new("/repo/.notes.tmp");

    assert!(persist::is_temp_sibling(sibling));
    assert_eq!(infra::persist::is_temp_sibling(sibling), persist::is_temp_sibling(sibling));
    assert!(!persist::is_temp_sibling(unrelated));
}

#[test]
fn watcher와_무시_디렉터리_정책은_기존_공개_경로를_유지한다() {
    let _: Option<infra::watcher::WatchScope> = Some(watcher::WatchScope::Project);
    assert_eq!(watch_policy::WATCH_DEBOUNCE_MS, constants::WATCH_DEBOUNCE_MS);
    assert_eq!(
        watch_policy::is_ignored_dir("node_modules"),
        constants::is_ignored_dir("node_modules")
    );

    let error = watcher::start_watch(PathBuf::new(), watcher::WatchScope::Project, |_| {})
        .err()
        .expect("빈 루트는 감시를 시작할 수 없어야 한다");
    assert_eq!(error.kind(), AppErrorKind::InvalidArgument);
}

#[test]
fn 파일_범위_파서는_기존_경로와_같은_상한과_csp를_쓴다() {
    assert_eq!(range_file::RANGE_RESPONSE_CSP, infra::range_file::RANGE_RESPONSE_CSP);
    assert_eq!(
        range_file::parse_range("bytes=-5", 10),
        infra::range_file::parse_range("bytes=-5", 10)
    );
    assert_eq!(range_file::parse_range("bytes=9-2", 10), None);
}

#[test]
fn 외부_url_검증은_기존_경로와_같은_위장_거부_정책을_쓴다() {
    let _: fn(&str) -> AppResult<String> = infra::external_url::validate_external_url;
    let suspicious = "https://trusted.example@evil.example/";
    let old = infra::external_url::validate_external_url(suspicious).unwrap_err();
    let new = external_url::validate_external_url(suspicious).unwrap_err();
    assert_eq!(old.kind(), new.kind());
}

#[test]
fn archive_보안_추출은_기존_경로와_같은_예산을_쓴다() {
    let _: fn(&Path, &Path) -> AppResult<()> = infra::archive::extract_hardened_zip;
    assert_eq!(archive::ARCHIVE_MAX_ENTRIES, infra::archive::ARCHIVE_MAX_ENTRIES);
    assert_eq!(archive::ARCHIVE_MAX_TOTAL_BYTES, infra::archive::ARCHIVE_MAX_TOTAL_BYTES);
}

#[test]
fn lsp_설치_자원은_기존_경로와_같은_타입과_해시를_쓴다() {
    let _: Option<infra::lsp_install::DownloadProgress> = None::<lsp_install::DownloadProgress>;
    let _: Option<infra::lsp_install::DownloadedFile> = None::<lsp_install::DownloadedFile>;
    let _: fn(&Path, &Path) -> AppResult<()> = lsp_install::atomic_install;

    assert_eq!(infra::lsp_install::platform_key(), lsp_install::platform_key());
    assert_eq!(infra::lsp_install::sha256_hex(b"lsp"), lsp_install::sha256_hex(b"lsp"));
    assert!(lsp_install::verify_sha256(b"lsp", &infra::lsp_install::sha256_hex(b"lsp")));
    assert_eq!(
        infra::lsp_install::substitute_template_args(&["{root}".to_string()], &[("root", "repo".to_string())]),
        lsp_install::substitute_template_args(&["{root}".to_string()], &[("root", "repo".to_string())])
    );
}

#[test]
fn http_클라이언트는_기존_프로필_타입과_반환_경로를_유지한다() {
    let profile: infra::http::HttpClientProfile = http::HttpClientProfile::Download;
    let _: reqwest::Client = infra::http::outbound_http_client(profile);
    let _: reqwest::Client = http::outbound_http_client(http::HttpClientProfile::Api);
}
