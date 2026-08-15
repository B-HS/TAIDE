use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::OnceLock;

use regex::Regex;
use serde::Deserialize;

use super::types::{Task, TaskSource};
use crate::infra::shell_quote::posix_quote;

const PACKAGE_MANIFEST_FILE: &str = "package.json";
const BUN_LOCKFILE_BINARY: &str = "bun.lockb";
const BUN_LOCKFILE_TEXT: &str = "bun.lock";
const PNPM_LOCKFILE: &str = "pnpm-lock.yaml";
const YARN_LOCKFILE: &str = "yarn.lock";

/// `make`'s own manual documents this exact lookup priority (`GNUmakefile`,
/// then `makefile`, then `Makefile`) when no `-f` is given — replicated here
/// so the detected targets correspond to whichever file `make` would
/// actually read.
const MAKEFILE_NAMES: &[&str] = &["GNUmakefile", "makefile", "Makefile"];

const CARGO_MANIFEST_FILE: &str = "Cargo.toml";
const CARGO_TASK_SUBCOMMANDS: &[&str] = &["build", "test", "run", "check", "clippy"];

#[derive(Debug, Deserialize)]
struct PackageManifest {
    #[serde(default)]
    scripts: BTreeMap<String, String>,
}

fn detect_package_manager(root: &Path) -> &'static str {
    if root.join(BUN_LOCKFILE_BINARY).is_file() || root.join(BUN_LOCKFILE_TEXT).is_file() {
        "bun"
    } else if root.join(PNPM_LOCKFILE).is_file() {
        "pnpm"
    } else if root.join(YARN_LOCKFILE).is_file() {
        "yarn"
    } else {
        "npm"
    }
}

fn detect_npm_tasks(root: &Path) -> Vec<Task> {
    let Ok(raw) = std::fs::read_to_string(root.join(PACKAGE_MANIFEST_FILE)) else {
        return Vec::new();
    };
    let Ok(manifest) = serde_json::from_str::<PackageManifest>(&raw) else {
        return Vec::new();
    };

    let package_manager = detect_package_manager(root);
    let cwd = root.to_string_lossy().to_string();

    manifest
        .scripts
        .into_keys()
        .map(|name| Task {
            command: format!("{package_manager} run {}", posix_quote(&name)),
            label: name,
            source: TaskSource::Npm,
            cwd: cwd.clone(),
        })
        .collect()
}

fn find_makefile(root: &Path) -> Option<std::path::PathBuf> {
    MAKEFILE_NAMES.iter().map(|name| root.join(name)).find(|path| path.is_file())
}

/// `^([a-zA-Z0-9_][^:=]*):` per the Wave E contract — a best-effort scan, not
/// a real Makefile parser (that's explicitly deferred, mirroring the
/// `Cargo.toml`/`toml` crate tradeoff below). Two extra guards are applied on
/// top of the literal contract regex, since `[^:=]*` alone stops right before
/// the *first* `:` or `=` and cannot by itself tell a variable assignment
/// apart from a rule target:
/// - a match immediately followed by `=` (i.e. the line was actually
///   `name := ...`, GNU Make's immediate-expansion form) is rejected, or
///   `CC := gcc` would be misread as a target literally named `CC `.
/// - a match immediately followed by `:=` (i.e. `name ::= ...`, POSIX 2012's
///   immediate-assignment form) is rejected the same way — otherwise only the
///   first `:` of `::=` is consumed by the pattern's trailing `:`, leaving
///   `:= ...` as the "rest" and slipping past the first guard, which checks
///   only for a *bare* leading `=`.
///
/// A bare double-colon **rule** (`target:: prereq`, GNU Make's independent
/// double-colon rule syntax — not an assignment) is deliberately left
/// alone by both guards: its "rest" after the first `:` is `: prereq` (a
/// colon-then-non-`=`), which neither `starts_with('=')` nor
/// `starts_with(":=")` matches, so `target` still detects correctly.
fn detect_make_tasks(root: &Path) -> Vec<Task> {
    let Some(makefile_path) = find_makefile(root) else {
        return Vec::new();
    };
    let Ok(content) = std::fs::read_to_string(&makefile_path) else {
        return Vec::new();
    };

    static TARGET_PATTERN: OnceLock<Regex> = OnceLock::new();
    let target_pattern = TARGET_PATTERN.get_or_init(|| Regex::new(r"^([a-zA-Z0-9_][^:=]*):").expect("유효한 정규식"));
    let cwd = root.to_string_lossy().to_string();
    let mut seen = BTreeSet::new();

    content
        .lines()
        .filter_map(|line| {
            let captures = target_pattern.captures(line)?;
            let whole_match = captures.get(0)?;
            let rest = &line[whole_match.end()..];
            if rest.starts_with('=') || rest.starts_with(":=") {
                return None;
            }
            Some(captures[1].trim().to_string())
        })
        .filter(|target| !target.is_empty() && seen.insert(target.clone()))
        .map(|target| Task {
            command: format!("make {}", posix_quote(&target)),
            label: target,
            source: TaskSource::Make,
            cwd: cwd.clone(),
        })
        .collect()
}

/// Cargo tasks use a fixed command set rather than parsing `Cargo.toml` for
/// `[[bin]]`/workspace members/aliases — the `toml` crate is deliberately not
/// added as a dependency for this (see the Wave E contract's dependency
/// decision); precise Cargo task parsing is backlog.
fn detect_cargo_tasks(root: &Path) -> Vec<Task> {
    if !root.join(CARGO_MANIFEST_FILE).is_file() {
        return Vec::new();
    }

    let cwd = root.to_string_lossy().to_string();
    CARGO_TASK_SUBCOMMANDS
        .iter()
        .map(|subcommand| Task {
            label: format!("cargo {subcommand}"),
            command: format!("cargo {subcommand}"),
            source: TaskSource::Cargo,
            cwd: cwd.clone(),
        })
        .collect()
}

pub fn detect_tasks(root: &Path) -> Vec<Task> {
    let mut tasks = detect_npm_tasks(root);
    tasks.extend(detect_make_tasks(root));
    tasks.extend(detect_cargo_tasks(root));
    tasks
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_root(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("taide-task-test-{name}-{}", uuid::Uuid::new_v4()))
    }

    fn cleanup(root: &Path) {
        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    fn package_json_스크립트를_npm_run으로_변환한다() {
        let root = temp_root("npm-basic");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("package.json"), r#"{"scripts":{"build":"tsc","test":"vitest run"}}"#).unwrap();

        let tasks = detect_npm_tasks(&root);

        assert_eq!(tasks.len(), 2);
        assert!(tasks.iter().all(|task| task.source == TaskSource::Npm));
        let build = tasks.iter().find(|task| task.label == "build").expect("build 태스크 존재");
        assert_eq!(build.command, "npm run 'build'");

        cleanup(&root);
    }

    #[test]
    fn bun_락파일이_있으면_bun_run을_사용한다() {
        let root = temp_root("bun-lock");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("package.json"), r#"{"scripts":{"dev":"vite"}}"#).unwrap();
        std::fs::write(root.join("bun.lock"), "").unwrap();

        let tasks = detect_npm_tasks(&root);

        assert_eq!(tasks[0].command, "bun run 'dev'");

        cleanup(&root);
    }

    #[test]
    fn pnpm_락파일이_있으면_pnpm_run을_사용한다() {
        let root = temp_root("pnpm-lock");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("package.json"), r#"{"scripts":{"dev":"vite"}}"#).unwrap();
        std::fs::write(root.join("pnpm-lock.yaml"), "").unwrap();

        let tasks = detect_npm_tasks(&root);

        assert_eq!(tasks[0].command, "pnpm run 'dev'");

        cleanup(&root);
    }

    #[test]
    fn 스크립트_이름에_셸_특수문자가_있어도_안전하게_인용된다() {
        let root = temp_root("npm-injection");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("package.json"), r#"{"scripts":{"a; rm -rf ~ #":"echo hi"}}"#).unwrap();

        let tasks = detect_npm_tasks(&root);

        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].command, "npm run 'a; rm -rf ~ #'");

        cleanup(&root);
    }

    #[test]
    fn package_json이_없으면_빈_목록을_반환한다() {
        let root = temp_root("no-package-json");
        std::fs::create_dir_all(&root).unwrap();

        assert!(detect_npm_tasks(&root).is_empty());

        cleanup(&root);
    }

    #[test]
    fn package_json_파싱에_실패하면_빈_목록을_반환한다() {
        let root = temp_root("bad-package-json");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("package.json"), "{ not json").unwrap();

        assert!(detect_npm_tasks(&root).is_empty());

        cleanup(&root);
    }

    #[test]
    fn makefile_타겟을_추출한다() {
        let root = temp_root("makefile-basic");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(
            root.join("Makefile"),
            "build:\n\techo building\ntest: build\n\techo testing\n.PHONY: build test\n",
        )
        .unwrap();

        let tasks = detect_make_tasks(&root);

        let labels: Vec<_> = tasks.iter().map(|task| task.label.as_str()).collect();
        assert_eq!(labels, vec!["build", "test"]);
        assert_eq!(tasks[0].command, "make 'build'");

        cleanup(&root);
    }

    #[test]
    fn 콜론_대입_행은_타겟으로_오인되지_않는다() {
        let root = temp_root("makefile-assign");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("Makefile"), "CC := gcc\nbuild:\n\t$(CC) main.c\n").unwrap();

        let tasks = detect_make_tasks(&root);

        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].label, "build");

        cleanup(&root);
    }

    #[test]
    fn 이중_콜론_대입_행도_타겟으로_오인되지_않는다() {
        let root = temp_root("makefile-double-colon-assign");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("Makefile"), "OBJS ::= a.o b.o\nbuild:\n\t$(CC) $(OBJS)\n").unwrap();

        let tasks = detect_make_tasks(&root);

        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].label, "build");

        cleanup(&root);
    }

    #[test]
    fn 이중_콜론_규칙은_타겟으로_정상_감지된다() {
        let root = temp_root("makefile-double-colon-rule");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("Makefile"), "build:: prereq\n\techo building\n").unwrap();

        let tasks = detect_make_tasks(&root);

        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].label, "build");

        cleanup(&root);
    }

    #[test]
    fn gnumakefile가_있으면_makefile보다_우선한다() {
        let root = temp_root("makefile-priority");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("Makefile"), "from-makefile:\n\techo x\n").unwrap();
        std::fs::write(root.join("GNUmakefile"), "from-gnumakefile:\n\techo x\n").unwrap();

        let tasks = detect_make_tasks(&root);

        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].label, "from-gnumakefile");

        cleanup(&root);
    }

    #[test]
    fn makefile가_없으면_빈_목록을_반환한다() {
        let root = temp_root("no-makefile");
        std::fs::create_dir_all(&root).unwrap();

        assert!(detect_make_tasks(&root).is_empty());

        cleanup(&root);
    }

    #[test]
    fn cargo_toml이_있으면_고정_명령_세트를_반환한다() {
        let root = temp_root("cargo-basic");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("Cargo.toml"), "[package]\nname = \"x\"\n").unwrap();

        let tasks = detect_cargo_tasks(&root);

        let commands: Vec<_> = tasks.iter().map(|task| task.command.as_str()).collect();
        assert_eq!(
            commands,
            vec!["cargo build", "cargo test", "cargo run", "cargo check", "cargo clippy"]
        );
        assert!(tasks.iter().all(|task| task.source == TaskSource::Cargo));

        cleanup(&root);
    }

    #[test]
    fn cargo_toml이_없으면_빈_목록을_반환한다() {
        let root = temp_root("no-cargo-toml");
        std::fs::create_dir_all(&root).unwrap();

        assert!(detect_cargo_tasks(&root).is_empty());

        cleanup(&root);
    }

    #[test]
    fn detect_tasks는_세_소스를_모두_합친다() {
        let root = temp_root("all-sources");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("package.json"), r#"{"scripts":{"build":"tsc"}}"#).unwrap();
        std::fs::write(root.join("Makefile"), "lint:\n\techo lint\n").unwrap();
        std::fs::write(root.join("Cargo.toml"), "[package]\nname = \"x\"\n").unwrap();

        let tasks = detect_tasks(&root);

        assert_eq!(tasks.iter().filter(|task| task.source == TaskSource::Npm).count(), 1);
        assert_eq!(tasks.iter().filter(|task| task.source == TaskSource::Make).count(), 1);
        assert_eq!(tasks.iter().filter(|task| task.source == TaskSource::Cargo).count(), 5);

        cleanup(&root);
    }

    #[test]
    fn 모든_태스크는_프로젝트_루트를_cwd로_가진다() {
        let root = temp_root("cwd-check");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("package.json"), r#"{"scripts":{"build":"tsc"}}"#).unwrap();

        let tasks = detect_tasks(&root);
        let expected_cwd = root.to_string_lossy().to_string();

        assert!(tasks.iter().all(|task| task.cwd == expected_cwd));

        cleanup(&root);
    }
}
