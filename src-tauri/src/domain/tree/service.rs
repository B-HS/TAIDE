use std::cmp::Ordering;
use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use crate::constants;
use crate::error::AppResult;

use super::types::{TreeEntryKind, TreeRow, TreeRowPage};

#[derive(Debug, Clone, PartialEq, Eq)]
struct Entry {
    path: PathBuf,
    name: String,
    kind: TreeEntryKind,
    has_children: bool,
}

#[derive(Debug, Clone)]
pub struct TreeState {
    root: PathBuf,
    expanded: BTreeSet<PathBuf>,
    cache: HashMap<PathBuf, Vec<Entry>>,
}

pub fn new_tree_state(root: PathBuf) -> TreeState {
    TreeState {
        root,
        expanded: BTreeSet::new(),
        cache: HashMap::new(),
    }
}

fn directory_has_children(path: &Path) -> bool {
    std::fs::read_dir(path).map(|mut iter| iter.next().is_some()).unwrap_or(false)
}

fn entry_order(a: &Entry, b: &Entry) -> Ordering {
    match (a.kind, b.kind) {
        (TreeEntryKind::Directory, TreeEntryKind::File) => Ordering::Less,
        (TreeEntryKind::File, TreeEntryKind::Directory) => Ordering::Greater,
        _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
    }
}

fn read_children(dir: &Path) -> AppResult<Vec<Entry>> {
    let mut entries = Vec::new();

    for item in std::fs::read_dir(dir)? {
        let item = item?;
        let metadata = match item.metadata() {
            Ok(metadata) => metadata,
            Err(_) => continue,
        };
        let name = item.file_name().to_string_lossy().to_string();
        let kind = if metadata.is_dir() {
            TreeEntryKind::Directory
        } else {
            TreeEntryKind::File
        };

        if kind == TreeEntryKind::Directory && constants::is_ignored_dir(&name) {
            continue;
        }

        let path = item.path();
        let has_children = if kind == TreeEntryKind::Directory {
            directory_has_children(&path)
        } else {
            false
        };

        entries.push(Entry {
            path,
            name,
            kind,
            has_children,
        });
    }

    entries.sort_by(entry_order);
    Ok(entries)
}

pub fn ensure_root_loaded(state: &mut TreeState) -> AppResult<()> {
    if state.cache.contains_key(&state.root) {
        return Ok(());
    }
    let children = read_children(&state.root)?;
    state.cache.insert(state.root.clone(), children);
    Ok(())
}

pub fn expand(state: &mut TreeState, path: &Path) -> AppResult<()> {
    ensure_root_loaded(state)?;

    if !state.cache.contains_key(path) {
        let children = read_children(path)?;
        state.cache.insert(path.to_path_buf(), children);
    }
    state.expanded.insert(path.to_path_buf());
    Ok(())
}

pub fn collapse(state: &mut TreeState, path: &Path) {
    state.expanded.remove(path);
}

pub fn toggle_expand(state: &mut TreeState, path: &Path) -> AppResult<()> {
    if state.expanded.contains(path) {
        collapse(state, path);
        return Ok(());
    }
    expand(state, path)
}

pub fn invalidate(state: &mut TreeState, dir: &Path) -> AppResult<()> {
    state.cache.remove(dir);

    let should_reload = dir == state.root.as_path() || state.expanded.contains(dir);
    if should_reload {
        let children = read_children(dir)?;
        state.cache.insert(dir.to_path_buf(), children);
    }
    Ok(())
}

pub fn reveal(state: &mut TreeState, path: &Path) -> AppResult<()> {
    ensure_root_loaded(state)?;

    let mut ancestors = Vec::new();
    let mut current = path.parent();
    while let Some(dir) = current {
        if dir == state.root.as_path() || !dir.starts_with(&state.root) {
            break;
        }
        ancestors.push(dir.to_path_buf());
        current = dir.parent();
    }
    ancestors.reverse();

    for dir in ancestors {
        expand(state, &dir)?;
    }
    Ok(())
}

fn push_children(state: &TreeState, entries: &[Entry], depth: u32, rows: &mut Vec<TreeRow>) {
    for entry in entries {
        let expanded = state.expanded.contains(&entry.path);
        rows.push(TreeRow {
            path: entry.path.to_string_lossy().to_string(),
            name: entry.name.clone(),
            kind: entry.kind,
            depth,
            expanded,
            has_children: entry.has_children,
        });

        if entry.kind == TreeEntryKind::Directory && expanded {
            if let Some(children) = state.cache.get(&entry.path) {
                push_children(state, children, depth + 1, rows);
            }
        }
    }
}

pub fn flatten(state: &TreeState) -> Vec<TreeRow> {
    let mut rows = Vec::new();
    if let Some(children) = state.cache.get(&state.root) {
        push_children(state, children, 0, &mut rows);
    }
    rows
}

/// `limit: None` returns every remaining row from `offset` onward — the caller-side "give me the
/// whole tree" case (`docs/acknowledge/2026-08-19-xa-wiring-cleanup-contract.md` §1.3(7)), which used
/// to be spelled as a `u32::MAX` sentinel (`TREE_ROWS_UNBOUNDED_LIMIT` on the frontend) instead of a
/// real "no limit" value.
pub fn rows_page(state: &TreeState, offset: u32, limit: Option<u32>) -> TreeRowPage {
    let rows = flatten(state);
    let total = rows.len() as u32;

    let start = (offset as usize).min(rows.len());
    let end = match limit {
        Some(limit) => start.saturating_add(limit as usize).min(rows.len()),
        None => rows.len(),
    };

    TreeRowPage {
        rows: rows[start..end].to_vec(),
        total,
    }
}

pub fn full_page(state: &TreeState) -> TreeRowPage {
    let rows = flatten(state);
    let total = rows.len() as u32;
    TreeRowPage { rows, total }
}

pub fn expanded_paths(state: &TreeState) -> Vec<String> {
    state.expanded.iter().map(|path| path.to_string_lossy().to_string()).collect()
}

pub fn restore_expanded(state: &mut TreeState, paths: Vec<String>) {
    for raw in paths {
        let _ = expand(state, Path::new(&raw));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fixture {
        root: PathBuf,
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.root).ok();
        }
    }

    fn build_fixture() -> Fixture {
        let root = std::env::temp_dir().join(format!("taide-tree-{}", uuid::Uuid::new_v4()));

        std::fs::create_dir_all(root.join("src").join("utils")).unwrap();
        std::fs::create_dir_all(root.join("node_modules").join("pkg")).unwrap();
        std::fs::write(root.join("src").join("main.rs"), "fn main() {}").unwrap();
        std::fs::write(root.join("src").join("utils").join("deep.rs"), "// deep").unwrap();
        std::fs::write(root.join("README.md"), "# readme").unwrap();
        std::fs::write(root.join("Cargo.toml"), "[package]").unwrap();

        Fixture { root }
    }

    #[test]
    fn 무시_디렉토리는_자식_목록에서_제외된다() {
        let fixture = build_fixture();
        let children = read_children(&fixture.root).unwrap();

        assert!(!children.iter().any(|entry| entry.name == "node_modules"));
    }

    #[test]
    fn 디렉토리가_파일보다_먼저_오고_이름순으로_정렬된다() {
        let fixture = build_fixture();
        let children = read_children(&fixture.root).unwrap();

        let names: Vec<&str> = children.iter().map(|entry| entry.name.as_str()).collect();
        assert_eq!(names, vec!["src", "Cargo.toml", "README.md"]);
    }

    #[test]
    fn flatten_은_펼쳐진_노드만_깊이순으로_평탄화한다() {
        let fixture = build_fixture();
        let mut state = new_tree_state(fixture.root.clone());
        ensure_root_loaded(&mut state).unwrap();

        let rows_before_expand = flatten(&state);
        assert_eq!(rows_before_expand.len(), 3);
        assert!(rows_before_expand.iter().all(|row| row.depth == 0));

        let src_path = fixture.root.join("src");
        expand(&mut state, &src_path).unwrap();

        let rows = flatten(&state);
        let src_row = rows.iter().find(|row| row.path == src_path.to_string_lossy()).unwrap();
        assert!(src_row.expanded);
        assert!(src_row.has_children);

        let child_rows: Vec<&TreeRow> = rows.iter().filter(|row| row.depth == 1).collect();
        assert_eq!(child_rows.len(), 2);
        assert_eq!(child_rows[0].name, "utils");
        assert_eq!(child_rows[1].name, "main.rs");

        let utils_row = child_rows[0];
        assert!(utils_row.has_children);
    }

    #[test]
    fn 접힌_노드의_자식은_평탄화_결과에_없다() {
        let fixture = build_fixture();
        let mut state = new_tree_state(fixture.root.clone());
        ensure_root_loaded(&mut state).unwrap();

        let src_path = fixture.root.join("src");
        expand(&mut state, &src_path).unwrap();
        assert_eq!(flatten(&state).len(), 5);

        collapse(&mut state, &src_path);
        let rows = flatten(&state);
        assert_eq!(rows.len(), 3);
        assert!(rows.iter().all(|row| row.depth == 0));
    }

    #[test]
    fn rows_page_는_offset_limit_경계를_벗어나지_않는다() {
        let fixture = build_fixture();
        let mut state = new_tree_state(fixture.root.clone());
        ensure_root_loaded(&mut state).unwrap();

        let page = rows_page(&state, 0, Some(2));
        assert_eq!(page.rows.len(), 2);
        assert_eq!(page.total, 3);

        let overflow_page = rows_page(&state, 10, Some(5));
        assert!(overflow_page.rows.is_empty());
        assert_eq!(overflow_page.total, 3);

        let clipped_page = rows_page(&state, 2, Some(10));
        assert_eq!(clipped_page.rows.len(), 1);
    }

    #[test]
    fn rows_page_는_limit이_none이면_offset_이후_전체를_반환한다() {
        let fixture = build_fixture();
        let mut state = new_tree_state(fixture.root.clone());
        ensure_root_loaded(&mut state).unwrap();

        let page = rows_page(&state, 0, None);
        assert_eq!(page.rows.len(), 3);
        assert_eq!(page.total, 3);

        let offset_page = rows_page(&state, 1, None);
        assert_eq!(offset_page.rows.len(), 2);
        assert_eq!(offset_page.total, 3);
    }

    #[test]
    fn reveal_은_대상_파일까지의_조상을_전부_펼친다() {
        let fixture = build_fixture();
        let mut state = new_tree_state(fixture.root.clone());
        ensure_root_loaded(&mut state).unwrap();

        let deep_file = fixture.root.join("src").join("utils").join("deep.rs");
        reveal(&mut state, &deep_file).unwrap();

        let src_path = fixture.root.join("src");
        let utils_path = fixture.root.join("src").join("utils");

        let rows = flatten(&state);
        assert!(rows.iter().any(|row| row.path == src_path.to_string_lossy() && row.expanded));
        assert!(rows.iter().any(|row| row.path == utils_path.to_string_lossy() && row.expanded));
        assert!(rows.iter().any(|row| row.name == "deep.rs"));
    }

    #[test]
    fn 펼침_상태는_직렬화하고_복원해도_동일하다() {
        let fixture = build_fixture();
        let mut state = new_tree_state(fixture.root.clone());
        ensure_root_loaded(&mut state).unwrap();

        let src_path = fixture.root.join("src");
        expand(&mut state, &src_path).unwrap();
        let saved = expanded_paths(&state);

        let mut restored = new_tree_state(fixture.root.clone());
        ensure_root_loaded(&mut restored).unwrap();
        restore_expanded(&mut restored, saved);

        assert_eq!(flatten(&restored).len(), flatten(&state).len());
    }

    #[test]
    fn invalidate_는_대상_디렉토리만_재조회한다() {
        let fixture = build_fixture();
        let mut state = new_tree_state(fixture.root.clone());
        ensure_root_loaded(&mut state).unwrap();

        let src_path = fixture.root.join("src");
        expand(&mut state, &src_path).unwrap();
        assert_eq!(flatten(&state).len(), 5);

        std::fs::write(src_path.join("new_file.rs"), "// new").unwrap();
        invalidate(&mut state, &src_path).unwrap();

        let rows = flatten(&state);
        assert_eq!(rows.len(), 6);
        assert!(rows.iter().any(|row| row.name == "new_file.rs"));
    }
}
