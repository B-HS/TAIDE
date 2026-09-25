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
/// - `project/commands.rs → file::capability`·`git::watch`·`layout::service`·`settings::service`
///   — boot-time restore (`restore_state`/`projects_pending_watcher_restore`/
///   `restore_project_watchers`, moved verbatim from `lib.rs`'s former top-level boot-restore
///   helpers, called from `setup()`, d-32 R1) re-attaches every domain's per-project watcher and
///   reloads every domain's persisted state before the first window shows; the assembly (`lib.rs`
///   `setup()`) still owns the boot call order, only the step bodies live here. An assembly-owned
///   deferred-attach provider (a build/register split on the capability registry) could remove
///   the `file::capability`/`git::watch` halves; the `layout::service`/`settings::service` halves
///   are boot state loads and are not capability-shaped. Deferred — d-35 §4-f upheld the deferral
///   (precondition unchanged: a `ProjectCapability` build/register split, still not undertaken).
const ALLOWED_CROSS_DOMAIN_EDGES: &[(&str, &str)] = &[
    ("domain/project/commands.rs", "file::capability"),
    ("domain/project/commands.rs", "git::watch"),
    ("domain/project/commands.rs", "layout::service"),
    ("domain/project/commands.rs", "settings::service"),
];

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
/// a re-export would still evade this regex. The import-form ban below shrinks that surface by
/// rejecting every `use` shape that lets later code spell a cross-domain path without the full
/// `crate::domain::<domain>::<module>` text this scan matches (bare, domain-module, and
/// brace-group imports), so an import-side evasion has to be written out by hand instead of
/// falling out of an idiomatic import style.
fn cross_domain_references() -> BTreeSet<(String, String)> {
    let pattern = Regex::new(r"crate::domain::([a-z_0-9]+)::([A-Za-z_][A-Za-z0-9_]*)").expect("유효한 정규식");
    let mut found = BTreeSet::new();

    for file in rust_files(&src_dir().join("domain")) {
        let relative = relative_source_path(&file);
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
fn infra는_domain_참조를_가질_수_없다() {
    let pattern = Regex::new(r"crate::domain::([a-z_0-9]+)::([A-Za-z_][A-Za-z0-9_]*)").expect("유효한 정규식");
    let mut found = BTreeSet::new();

    for file in rust_files(&src_dir().join("infra")) {
        let relative = relative_source_path(&file);
        let source = fs::read_to_string(&file).expect("소스 파일 읽기");
        for capture in pattern.captures_iter(&strip_comment_lines(&source)) {
            found.insert((relative.clone(), format!("{}::{}", &capture[1], &capture[2])));
        }
    }

    assert!(
        found.is_empty(),
        "infra 는 domain 을 참조할 수 없습니다 (architecture.md §2):\n{found:#?}"
    );
}

/// Import forms that let later code reference another domain without ever spelling the full
/// `crate::domain::<domain>::<module>` path the boundary regex above matches: a bare
/// `use crate::domain;` (with or without `as`), a domain-module import
/// (`use crate::domain::layout;`, idiomatic Rust, with or without `as`), and every shallow
/// brace-group form (`use crate::{…}`, `use crate::domain::{…}`,
/// `use crate::domain::layout::{…}`). A group nested deeper
/// (`use crate::domain::layout::service::{…}`) already carries the `<domain>::<module>` text the
/// scan matches, so it needs no ban.
#[test]
fn 경계_스캔이_못_보는_import_형태는_domain과_infra에서_금지된다() {
    let pattern = Regex::new(
        r"(?m)^\s*use crate::(\{|domain\s*(as\s+[A-Za-z_][A-Za-z0-9_]*)?\s*;|domain::\{|domain::[a-z_0-9]+\s*(as\s+[A-Za-z_][A-Za-z0-9_]*)?\s*;|domain::[a-z_0-9]+::\{)",
    )
    .expect("유효한 정규식");
    let mut violations = Vec::new();

    for root in ["domain", "infra"] {
        for file in rust_files(&src_dir().join(root)) {
            let relative = relative_source_path(&file);
            let source = fs::read_to_string(&file).expect("소스 파일 읽기");
            if pattern.is_match(&strip_comment_lines(&source)) {
                violations.push(relative);
            }
        }
    }

    assert!(
        violations.is_empty(),
        "경계 스캔이 볼 수 없는 import 형태(bare `use crate::domain;`·`use crate::domain::x;`·중괄호 그룹)는 domain·infra에서 금지됩니다 — `use crate::domain::x::y;` 단일 경로로 풀어 쓰십시오 (도메인 경계 스캔 우회 방지):\n{violations:#?}"
    );
}
