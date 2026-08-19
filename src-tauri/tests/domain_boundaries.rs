use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use regex::Regex;

/// Cross-domain references (`crate::domain::<other>::<module>`) that are **explicitly approved**
/// despite the domain-boundary rule (architecture.md §2: cross-domain function calls and store
/// references are forbidden; `types` references and the `project::capability` extension point are
/// always allowed and never need an entry here). Every entry is `(source file relative to `src/`,
/// `target-domain::module`)`; the tests below fail on any cross-domain reference **not** listed
/// here, and also fail on any entry that no longer matches a real reference — the whitelist can
/// only shrink by cleaning the edge up, never rot (the same "unlisted = rejected" shape as the
/// remote dispatch tables, T1-K).
///
/// Approval reasons, per entry (audit 2026-08-18 / contract T1-I §1.4):
/// - `agent/commands.rs → terminal::commands` — agent detection is *defined over* the terminal
///   foreground pid set (`TerminalStore::foreground_pids`); a read-only store query.
/// - `app/commands.rs → settings::commands`·`settings::service` — the `AppFileTarget::Settings`
///   editor must funnel through `apply_and_broadcast`, the single settings reapply path every
///   settings writer shares (its doc comment names these callers).
/// - `app/service.rs`·`app/types.rs → ai::prompt` — the app-file surface edits AI prompt template
///   files; their ids and bundled defaults are owned by `ai::prompt` and `PromptTemplateId` must
///   stay in lockstep with its `*_PROMPT_ID` constants.
/// - `file/commands.rs`·`git/commands.rs`·`ide/server.rs → plugin::service` — `ensure_loaded` is
///   the read-through plugin snapshot cache + `language_overlays` conversion every language-aware
///   domain needs; re-implementing the cache per domain would be worse than the edge.
/// - `ide/commands.rs → file::service` — `save_file_within_open_projects` is the root-guarded
///   single save path `file_save` itself uses (R6#2's fix: share it, don't clone it).
/// - `ide/server.rs`·`ide/service.rs → layout::service` — MCP tools (openFile/close_tab/
///   getOpenEditors) drive the tab lifecycle through the same layout orchestrators and pure
///   `PaneNode` helpers the layout commands use (R6#3's fix: service, not a second command entry).
/// - `layout/commands.rs → window::commands` + `window/service.rs → layout::service` — moving a
///   tab to another OS window spans both owners (layout owns tabs, window owns OS windows); the
///   two edges form a known cycle, reported for a future batch rather than half-fixed here.
/// - `layout/service.rs → ide::store`·`terminal::commands` — closing a tab must resolve its
///   pending Claude-diff responder and reap its pty session; both are part of close-tab's
///   correctness contract (moved verbatim from the old commands body in R2).
/// - `remote/login_page.rs → locale::service` — the served login HTML renders the current UI
///   language's strings; locale is a data provider here.
/// - `settings/service.rs → theme::service` — `set_theme` validates that the target theme exists
///   before persisting it.
/// - `sync/* → settings::*`·`theme::service`·`locale::service`·`ai::providers` — sync is the
///   aggregation domain (upload/download bundles settings+themes+locales; audit R5#14 judged the
///   aggregation edges unavoidable); `ai::providers::mask_provider_error` is the shared
///   secret-masking helper for GitHub API error bodies.
/// - `vsix/commands.rs → plugin::service` — vsix import installs *into* the plugin store and
///   reloads it; the deliberate single direction left after R7#4's cycle cut (plugin no longer
///   references vsix).
const ALLOWED_CROSS_DOMAIN_EDGES: &[(&str, &str)] = &[
    ("domain/agent/commands.rs", "terminal::commands"),
    ("domain/app/commands.rs", "settings::commands"),
    ("domain/app/commands.rs", "settings::service"),
    ("domain/app/service.rs", "ai::prompt"),
    ("domain/app/types.rs", "ai::prompt"),
    ("domain/file/commands.rs", "plugin::service"),
    ("domain/git/commands.rs", "plugin::service"),
    ("domain/ide/commands.rs", "file::service"),
    ("domain/ide/server.rs", "layout::service"),
    ("domain/ide/server.rs", "plugin::service"),
    ("domain/ide/service.rs", "layout::service"),
    ("domain/layout/commands.rs", "window::commands"),
    ("domain/layout/service.rs", "ide::store"),
    ("domain/layout/service.rs", "terminal::commands"),
    ("domain/remote/login_page.rs", "locale::service"),
    ("domain/settings/service.rs", "theme::service"),
    ("domain/sync/commands.rs", "settings::commands"),
    ("domain/sync/commands.rs", "settings::service"),
    ("domain/sync/github.rs", "ai::providers"),
    ("domain/sync/service.rs", "locale::service"),
    ("domain/sync/service.rs", "settings::service"),
    ("domain/sync/service.rs", "theme::service"),
    ("domain/vsix/commands.rs", "plugin::service"),
    ("domain/window/service.rs", "layout::service"),
];

/// `infra → domain` references are forbidden even for types (layer direction — audit R4#6); these
/// four `types`-only references are the approved remainder, each a future inversion candidate in
/// the shape `infra::language::LanguageOverlay` already demonstrated:
/// - `asset_protocol.rs`·`root_guard.rs → project::types` — both take the open-projects map as a
///   parameter (never `AppState`) and only read `Project.root`; inverting would ripple a
///   lightweight root-set type through every guard call site.
/// - `self_write.rs`·`watcher.rs → file::types` — the watcher pipeline produces `FsChange`
///   directly; inverting needs an infra-owned raw-change type plus a domain-side classifier move.
const ALLOWED_INFRA_DOMAIN_REFS: &[(&str, &str)] = &[
    ("infra/asset_protocol.rs", "project::types"),
    ("infra/root_guard.rs", "project::types"),
    ("infra/self_write.rs", "file::types"),
    ("infra/watcher.rs", "file::types"),
];

/// The one file allowed to hold a bare `use crate::domain;` import and reference every domain's
/// `commands`: the remote gateway is a dispatch table over the whole command surface by design
/// (architecture.md §4 — default-deny table), and its own parity tests already pin that table.
const REMOTE_DISPATCH_GATEWAY: &str = "domain/remote/dispatch.rs";

/// Recursively collects every `.rs` file under `dir`, sorted for deterministic failure output.
fn rust_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let entries = fs::read_dir(dir).unwrap_or_else(|error| panic!("디렉터리를 읽을 수 없습니다 ({}): {error}", dir.display()));
    for entry in entries {
        let path = entry.expect("디렉터리 항목").path();
        if path.is_dir() {
            files.extend(rust_files(&path));
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            files.push(path);
        }
    }
    files.sort();
    files
}

/// Drops every line whose first non-whitespace token starts a line comment (`//`, `///`, `//!`),
/// so doc-comment mentions of cross-domain paths don't count as references. Block comments
/// (`/* */`) are not handled — the codebase's comment convention forbids them, and a reference
/// smuggled into one would *fail* this scan loudly rather than pass silently.
fn strip_comment_lines(source: &str) -> String {
    source
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn src_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

fn relative_source_path(file: &Path) -> String {
    file.strip_prefix(src_dir())
        .expect("src/ 하위 경로")
        .to_string_lossy()
        .replace('\\', "/")
}

/// Scans `domain/**/*.rs` for `crate::domain::<other>::<module>` references. Detection is
/// source-text based (the same approach as `lib.rs`'s `collect_commands!` parity test and
/// `dispatch.rs`'s match-arm scan): a reference spelled through a deep `super::super::…` chain or
/// a re-export would evade the regex — the ban on bare `use crate::domain;` imports below closes
/// the one evasion the codebase has actually used, and anything else is a conscious act the
/// whitelist review would catch.
fn cross_domain_references() -> BTreeSet<(String, String)> {
    let pattern = Regex::new(r"crate::domain::([a-z_0-9]+)::([A-Za-z_][A-Za-z0-9_]*)").expect("유효한 정규식");
    let mut found = BTreeSet::new();

    for file in rust_files(&src_dir().join("domain")) {
        let relative = relative_source_path(&file);
        if relative == REMOTE_DISPATCH_GATEWAY {
            continue;
        }
        let Some(own_domain) = relative.strip_prefix("domain/").and_then(|rest| rest.split('/').next()) else {
            continue;
        };
        if own_domain.ends_with(".rs") {
            continue;
        }

        let source = fs::read_to_string(&file).expect("소스 파일 읽기");
        for capture in pattern.captures_iter(&strip_comment_lines(&source)) {
            let (target_domain, target_module) = (&capture[1], &capture[2]);
            if target_domain == own_domain {
                continue;
            }
            found.insert((relative.clone(), format!("{target_domain}::{target_module}")));
        }
    }

    found
}

#[test]
fn 도메인_간_참조는_types와_capability_확장점과_화이트리스트만_허용된다() {
    let structurally_allowed = |target: &str| target.ends_with("::types") || target == "project::capability";

    let found: BTreeSet<(String, String)> = cross_domain_references()
        .into_iter()
        .filter(|(_, target)| !structurally_allowed(target))
        .collect();
    let allowed: BTreeSet<(String, String)> = ALLOWED_CROSS_DOMAIN_EDGES
        .iter()
        .map(|(file, target)| (file.to_string(), target.to_string()))
        .collect();

    let violations: Vec<_> = found.difference(&allowed).collect();
    assert!(
        violations.is_empty(),
        "화이트리스트에 없는 도메인 간 실행 경로 참조가 있습니다 (architecture.md §2 — 정리하거나, 불가피하면 사유와 함께 ALLOWED_CROSS_DOMAIN_EDGES 에 등재하십시오):\n{violations:#?}"
    );

    let stale: Vec<_> = allowed.difference(&found).collect();
    assert!(
        stale.is_empty(),
        "ALLOWED_CROSS_DOMAIN_EDGES 에 더 이상 실재하지 않는 항목이 있습니다 — 엣지를 정리했다면 화이트리스트에서도 제거해 최소성을 유지하십시오:\n{stale:#?}"
    );
}

#[test]
fn infra는_화이트리스트_밖의_domain_참조를_가질_수_없다() {
    let pattern = Regex::new(r"crate::domain::([a-z_0-9]+)::([A-Za-z_][A-Za-z0-9_]*)").expect("유효한 정규식");
    let mut found = BTreeSet::new();

    for file in rust_files(&src_dir().join("infra")) {
        let relative = relative_source_path(&file);
        let source = fs::read_to_string(&file).expect("소스 파일 읽기");
        for capture in pattern.captures_iter(&strip_comment_lines(&source)) {
            found.insert((relative.clone(), format!("{}::{}", &capture[1], &capture[2])));
        }
    }

    let allowed: BTreeSet<(String, String)> = ALLOWED_INFRA_DOMAIN_REFS
        .iter()
        .map(|(file, target)| (file.to_string(), target.to_string()))
        .collect();

    let violations: Vec<_> = found.difference(&allowed).collect();
    assert!(
        violations.is_empty(),
        "infra 는 domain 을 참조할 수 없습니다 (계층 역방향 — architecture.md §2; infra 측 경량 타입을 정의하고 domain 이 변환해 전달하십시오, `infra::language::LanguageOverlay` 선례):\n{violations:#?}"
    );

    let stale: Vec<_> = allowed.difference(&found).collect();
    assert!(
        stale.is_empty(),
        "ALLOWED_INFRA_DOMAIN_REFS 에 더 이상 실재하지 않는 항목이 있습니다 — 역참조를 정리했다면 화이트리스트에서도 제거하십시오:\n{stale:#?}"
    );
}

/// A bare `use crate::domain;` (or `use crate::domain as …;`) import lets later code write
/// `domain::x::y` paths the `crate::domain::` regex above never sees — the remote dispatch
/// gateway is the only file that legitimately needs that form (it references all twenty domains'
/// commands), so everywhere else it is rejected as an evasion vector rather than matched.
#[test]
fn bare_domain_import는_remote_dispatch_게이트웨이에서만_허용된다() {
    let pattern = Regex::new(r"(?m)^\s*use crate::domain(\s+as\s+[A-Za-z_][A-Za-z0-9_]*)?;").expect("유효한 정규식");
    let mut violations = Vec::new();

    for root in ["domain", "infra"] {
        for file in rust_files(&src_dir().join(root)) {
            let relative = relative_source_path(&file);
            if relative == REMOTE_DISPATCH_GATEWAY {
                continue;
            }
            let source = fs::read_to_string(&file).expect("소스 파일 읽기");
            if pattern.is_match(&strip_comment_lines(&source)) {
                violations.push(relative);
            }
        }
    }

    assert!(
        violations.is_empty(),
        "bare `use crate::domain;` import 는 remote dispatch 게이트웨이 전용입니다 (도메인 경계 스캔 우회 방지):\n{violations:#?}"
    );
}
