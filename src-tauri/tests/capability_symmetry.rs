use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use regex::Regex;

/// The domain that declares the `ProjectCapability` trait itself rather than implementing it. Its
/// `capability.rs` holds the trait, the registry, and the registry's own test stubs (`KindOnly`,
/// `RecordingCapability`), none of which are registered capabilities — so it is skipped by the
/// implementation scan below.
const TRAIT_OWNER_DOMAIN: &str = "project";

/// One `impl ProjectCapability for X { … }` block found in a domain's `capability.rs`.
struct CapabilityImpl {
    domain: String,
    name: String,
    body: String,
}

impl CapabilityImpl {
    fn qualified(&self) -> String {
        format!("{}::{}", self.domain, self.name)
    }

    fn implements(&self, method: &str) -> bool {
        self.body.contains(&format!("fn {method}("))
    }
}

fn src_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

/// Drops line comments so a doc comment's prose (which may quote `ProjectAttachment::new` or an
/// unbalanced brace) never counts as code — the same treatment `domain_boundaries.rs` applies to
/// its own source scan, and for the same reason.
fn strip_comment_lines(source: &str) -> String {
    source
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Returns the text between the `{` at `open_index` and its matching `}`. Brace counting is enough
/// here because the input has already had comment lines stripped and this codebase's capability
/// bodies contain no string literal with an unbalanced brace; a body that grew one would make this
/// scan fail loudly (a truncated body missing its `fn detach`) rather than pass with a wrong answer.
fn balanced_body(source: &str, open_index: usize) -> String {
    let mut depth = 0usize;
    for (offset, character) in source[open_index..].char_indices() {
        match character {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return source[open_index + 1..open_index + offset].to_string();
                }
            }
            _ => {}
        }
    }
    panic!("중괄호가 닫히지 않았습니다 (capability impl 본문 추출 실패)");
}

fn capability_impls() -> Vec<CapabilityImpl> {
    let pattern = Regex::new(r"impl ProjectCapability for ([A-Za-z][A-Za-z0-9_]*)\s*\{").expect("유효한 정규식");
    let mut impls = Vec::new();

    let domains = fs::read_dir(src_dir().join("domain")).expect("domain 디렉터리");
    let mut domain_dirs: Vec<PathBuf> = domains
        .map(|entry| entry.expect("디렉터리 항목").path())
        .filter(|path| path.is_dir())
        .collect();
    domain_dirs.sort();

    for domain_dir in domain_dirs {
        let domain = domain_dir.file_name().expect("도메인 이름").to_string_lossy().to_string();
        if domain == TRAIT_OWNER_DOMAIN {
            continue;
        }
        let file = domain_dir.join("capability.rs");
        if !file.exists() {
            continue;
        }

        let source = strip_comment_lines(&fs::read_to_string(&file).expect("capability.rs 읽기"));
        for capture in pattern.captures_iter(&source) {
            let whole = capture.get(0).expect("전체 매치");
            impls.push(CapabilityImpl {
                domain: domain.clone(),
                name: capture[1].to_string(),
                body: balanced_body(&source, whole.end() - 1),
            });
        }
    }

    impls
}

/// The `domain::name` pairs `lib.rs`'s `project_capabilities()` registers, in registration order.
/// `lib.rs` already pins that order against a literal list; this reads the same block to compare it
/// against what the domains actually implement.
fn registered_capabilities() -> Vec<String> {
    const BLOCK_START: &str = "ProjectCapabilities::new(vec![";
    let source = fs::read_to_string(src_dir().join("lib.rs")).expect("lib.rs 읽기");
    let pattern = Regex::new(r"Box::new\(domain::([a-z_0-9]+)::capability::([A-Za-z][A-Za-z0-9_]*)\)").expect("유효한 정규식");
    let start = source.find(BLOCK_START).expect("capability 등록 블록") + BLOCK_START.len();
    let end = start + source[start..].find("])").expect("등록 벡터의 끝");

    pattern
        .captures_iter(&source[start..end])
        .map(|capture| format!("{}::{}", &capture[1], &capture[2]))
        .collect()
}

/// A capability that exists but is never registered attaches and detaches nothing (dead code that
/// reads as live); a registration whose implementation moved domains would not compile, but one
/// whose `capability.rs` lost its `impl` while a same-named struct survived elsewhere would.
/// `architecture.md` §3 makes `lib.rs`'s list the single registration point, so the two sets must
/// be equal in both directions.
#[test]
fn 모든_capability_구현체는_lib_rs_에_정확히_한_번_등록된다() {
    let implemented: BTreeSet<String> = capability_impls().iter().map(CapabilityImpl::qualified).collect();
    let registered_order = registered_capabilities();
    let registered: BTreeSet<String> = registered_order.iter().cloned().collect();

    assert_eq!(
        registered.len(),
        registered_order.len(),
        "같은 capability 가 두 번 등록되면 attach·detach 가 두 번 실행됩니다: {registered_order:#?}"
    );

    let unregistered: Vec<&String> = implemented.difference(&registered).collect();
    assert!(
        unregistered.is_empty(),
        "구현만 되고 lib.rs 의 project_capabilities() 에 등록되지 않은 capability 가 있습니다 (attach·detach 가 아예 실행되지 않습니다):\n{unregistered:#?}"
    );

    let unimplemented: Vec<&String> = registered.difference(&implemented).collect();
    assert!(
        unimplemented.is_empty(),
        "등록되었지만 해당 도메인의 capability.rs 에 impl 이 없는 항목이 있습니다:\n{unimplemented:#?}"
    );
}

/// `architecture.md` §3.1·§6.3: "build 에서 만든 것 ↔ detach 에서 회수하는 것" must be 1:1. A
/// capability whose build returns a real [`ProjectAttachment`] writes something into `AppState`
/// under the commit guard, and every such write is listed in §6.3's reclaim table — which means it
/// must have a `detach` to reclaim it. A build that only returns `ProjectAttachment::none()`
/// (side-effect-only attaches such as the agent hooks reconcile) is exempt: it registers nothing,
/// so there is nothing asymmetric about having no detach.
///
/// **What this does not check.** It is a source scan, so the symmetry it pins is *presence* — a
/// build that registers has a detach — not *identity*: a capability whose build writes
/// `state.foo` while its detach reclaims `state.bar` still passes. Only a running app could tell
/// those apart by observing `AppState`, and this codebase has no `tauri::test` mock-app harness
/// (the constraint `domain::project::commands`'s own tests document). §6.3's reclaim table stays
/// the authority on which field pairs with which.
#[test]
fn attachment_을_등록하는_capability_는_detach_를_구현한다() {
    let missing: Vec<String> = capability_impls()
        .into_iter()
        .filter(|capability| capability.body.contains("ProjectAttachment::new("))
        .filter(|capability| !capability.implements("detach"))
        .map(|capability| capability.qualified())
        .collect();

    assert!(
        missing.is_empty(),
        "ProjectAttachment::new 로 AppState 에 자원을 등록하면서 detach 를 구현하지 않은 capability 가 있습니다 (architecture.md §3.1·§6.3 대칭 요구 — 프로젝트를 닫아도 자원이 남습니다):\n{missing:#?}"
    );
}

/// Every registered capability must override at least one trait hook. All three defaults are
/// no-ops, so a capability that overrides none is a registration that costs a vtable walk on every
/// open and close and does nothing — almost certainly a half-finished move rather than an intent.
#[test]
fn 등록된_capability_는_적어도_하나의_훅을_구현한다() {
    let inert: Vec<String> = capability_impls()
        .into_iter()
        .filter(|capability| {
            !capability.implements("detected_kind") && !capability.implements("build_attachment") && !capability.implements("detach")
        })
        .map(|capability| capability.qualified())
        .collect();

    assert!(
        inert.is_empty(),
        "세 훅(detected_kind·build_attachment·detach) 중 어느 것도 구현하지 않은 capability 가 있습니다 — 등록을 지우거나 훅을 구현하십시오:\n{inert:#?}"
    );
}

/// The reclaim half of §6.3 is order-sensitive (`project_close` walks the registration list
/// forward), so a capability that only reclaims must still be reachable through that walk. This
/// pins the other direction of the same table: every `detach` implementation belongs to a
/// registered capability, i.e. no domain reclaims project state through a `capability.rs` the
/// registry never visits.
#[test]
fn detach_를_구현한_capability_는_모두_등록_순회에_들어있다() {
    let registered: BTreeSet<String> = registered_capabilities().into_iter().collect();

    let orphaned: Vec<String> = capability_impls()
        .into_iter()
        .filter(|capability| capability.implements("detach"))
        .map(|capability| capability.qualified())
        .filter(|qualified| !registered.contains(qualified))
        .collect();

    assert!(
        orphaned.is_empty(),
        "detach 를 구현했지만 등록 순회에 없어 project_close 가 절대 호출하지 않는 capability 가 있습니다:\n{orphaned:#?}"
    );
}
