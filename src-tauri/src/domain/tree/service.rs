use std::collections::{BTreeSet, HashMap, HashSet};
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

/// Sort key: directories ahead of files, then case-insensitive name. Computed once per entry by
/// `sort_by_cached_key` instead of inside a comparator — the comparator form lowercased **both**
/// names on every comparison, so one listing of n entries allocated ~2·n·log n `String`s to answer
/// the same question n allocations can (§2 H-4).
fn entry_sort_key(entry: &Entry) -> (u8, String) {
    let kind_rank = match entry.kind {
        TreeEntryKind::Directory => 0,
        TreeEntryKind::File => 1,
    };
    (kind_rank, entry.name.to_lowercase())
}

/// Lists one directory's visible entries, sorted by [`entry_sort_key`].
///
/// The kind comes from `DirEntry::file_type()`, not `DirEntry::metadata()`. Both answer the same
/// question here — std documents each as *not* traversing symlinks, so a symlink is a
/// [`TreeEntryKind::File`] either way, pointing at a directory or not — but on Unix `metadata()` is
/// documented as "the equivalent of calling `symlink_metadata`", i.e. one `lstat` syscall per
/// entry, while `file_type()` is free on most Unix platforms because `readdir` already returned
/// `d_type` (research 3b §2-F). Where `d_type` is `DT_UNKNOWN` — some network volumes — std falls
/// back to that same `symlink_metadata` internally, so the worst case is today's cost, never worse;
/// the `Err(_) => continue` arm below is that fallback's failure path, exactly as it was
/// `metadata()`'s.
fn read_children(dir: &Path) -> AppResult<Vec<Entry>> {
    let mut entries = Vec::new();

    for item in std::fs::read_dir(dir)? {
        let item = item?;
        let file_type = match item.file_type() {
            Ok(file_type) => file_type,
            Err(_) => continue,
        };
        let name = item.file_name().to_string_lossy().to_string();
        let kind = if file_type.is_dir() {
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

    entries.sort_by_cached_key(entry_sort_key);
    Ok(entries)
}

/// Directory listings read **outside** every lock — both `TreeStore`'s write lock and the app-wide
/// `AppState::begin_mutation` guard — and handed to the state mutators below so the locked half only
/// inserts what is already in memory (§2 H-4). `read_children` is a synchronous `read_dir` plus one
/// more per subdirectory (`has_children`); running that under the mutation guard queued every file
/// and git mutation behind a cold directory scan.
///
/// A directory whose read failed is simply absent, and the mutators fall back to reading it inline —
/// so an unreadable directory still surfaces the exact error it always did, and a plan that went
/// stale between the prefetch and the guard is a missed optimization rather than a wrong answer.
#[derive(Debug, Default)]
pub struct DirectoryListings(HashMap<PathBuf, Vec<Entry>>);

/// Blocking half of the off-lock prefetch — call from `spawn_blocking`. Duplicate entries in `dirs`
/// (`plan_refresh_reads` names the root twice when refreshing the root itself) are read once.
pub fn read_directories(dirs: Vec<PathBuf>) -> DirectoryListings {
    let mut listings: HashMap<PathBuf, Vec<Entry>> = HashMap::new();
    for dir in dirs {
        if listings.contains_key(&dir) {
            continue;
        }
        if let Ok(entries) = read_children(&dir) {
            listings.insert(dir, entries);
        }
    }
    DirectoryListings(listings)
}

fn take_children(listings: &mut DirectoryListings, dir: &Path) -> AppResult<Vec<Entry>> {
    match listings.0.remove(dir) {
        Some(entries) => Ok(entries),
        None => read_children(dir),
    }
}

fn plan_root_read_into(state: &TreeState, dirs: &mut Vec<PathBuf>) {
    if !state.cache.contains_key(&state.root) {
        dirs.push(state.root.clone());
    }
}

/// Directories a `tree_rows` miss has to read before it can serve a page.
pub fn plan_root_read(state: &TreeState) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    plan_root_read_into(state, &mut dirs);
    dirs
}

/// Directories [`toggle_expand`] would read. Collapsing reads nothing, so a toggle that closes a
/// node skips the prefetch entirely.
pub fn plan_toggle_reads(state: &TreeState, path: &Path) -> Vec<PathBuf> {
    if state.expanded.contains(path) {
        return Vec::new();
    }

    let mut dirs = Vec::new();
    plan_root_read_into(state, &mut dirs);
    if !state.cache.contains_key(path) {
        dirs.push(path.to_path_buf());
    }
    dirs
}

/// Directories [`reveal`] would read: every uncached ancestor between the root and `path`.
pub fn plan_reveal_reads(state: &TreeState, path: &Path) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    plan_root_read_into(state, &mut dirs);
    dirs.extend(
        ancestors_under_root(state, path)
            .into_iter()
            .filter(|dir| !state.cache.contains_key(dir)),
    );
    dirs
}

/// Directories [`invalidate`] would read — mirrors its own reload condition, so a refresh of a
/// collapsed directory (which only drops the cache entry) plans no read.
pub fn plan_refresh_reads(state: &TreeState, dir: &Path) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    plan_root_read_into(state, &mut dirs);
    if dir == state.root.as_path() || state.expanded.contains(dir) {
        dirs.push(dir.to_path_buf());
    }
    dirs
}

pub fn ensure_root_loaded(state: &mut TreeState, listings: &mut DirectoryListings) -> AppResult<()> {
    if state.cache.contains_key(&state.root) {
        return Ok(());
    }
    let root = state.root.clone();
    let children = take_children(listings, &root)?;
    state.cache.insert(root, children);
    Ok(())
}

pub fn expand(state: &mut TreeState, path: &Path, listings: &mut DirectoryListings) -> AppResult<()> {
    ensure_root_loaded(state, listings)?;

    if !state.cache.contains_key(path) {
        let children = take_children(listings, path)?;
        state.cache.insert(path.to_path_buf(), children);
    }
    state.expanded.insert(path.to_path_buf());
    Ok(())
}

pub fn collapse(state: &mut TreeState, path: &Path) {
    state.expanded.remove(path);
}

pub fn toggle_expand(state: &mut TreeState, path: &Path, listings: &mut DirectoryListings) -> AppResult<()> {
    if state.expanded.contains(path) {
        collapse(state, path);
        return Ok(());
    }
    expand(state, path, listings)
}

pub fn invalidate(state: &mut TreeState, dir: &Path, listings: &mut DirectoryListings) -> AppResult<()> {
    state.cache.remove(dir);

    let should_reload = dir == state.root.as_path() || state.expanded.contains(dir);
    if should_reload {
        let children = take_children(listings, dir)?;
        prune_stale_descendants(state, dir, &children);
        state.cache.insert(dir.to_path_buf(), children);
    }
    Ok(())
}

/// `dir`'s immediate child on the way to `path`, or `None` when `path` is not a strict descendant of
/// `dir`. Component-wise, so a sibling named like a prefix (`src-old` next to `src`) never matches.
fn child_of(dir: &Path, path: &Path) -> Option<PathBuf> {
    let relative = path.strip_prefix(dir).ok()?;
    Some(dir.join(relative.components().next()?))
}

/// Drops the expanded flags and cached listings that a just-refreshed listing proves are gone
/// (§4-B C2).
///
/// Refreshing a directory only replaced that one row list, so deleting or renaming a folder left its
/// own cached children — and its whole expanded subtree — behind under the old path. Creating a
/// folder with the same name then resurrected all of it: the new folder rendered expanded and filled
/// with the deleted one's children (ghost rows). Pruning by path prefix is also what keeps both maps
/// from growing for the rest of the app's lifetime.
///
/// Two survival rules, both answered from the fresh listing alone (no extra syscalls):
/// - a cached listing survives only under a child directory that still reports children. An entry
///   that is now empty cannot own any cached descendant, so a folder emptied — or deleted and
///   recreated — between two refreshes can no longer serve its old rows.
/// - an expanded flag survives under any child directory that still exists, empty included. An empty
///   folder is legitimately expandable and clearing the flag would silently collapse a folder the
///   user opened; its stale listing is already gone by the rule above, so it simply renders empty.
fn prune_stale_descendants(state: &mut TreeState, dir: &Path, fresh: &[Entry]) {
    let mut present: HashSet<&Path> = HashSet::new();
    let mut populated: HashSet<&Path> = HashSet::new();
    for entry in fresh.iter().filter(|entry| entry.kind == TreeEntryKind::Directory) {
        present.insert(entry.path.as_path());
        if entry.has_children {
            populated.insert(entry.path.as_path());
        }
    }

    state.cache.retain(|path, _| survives_prune(dir, path, &populated));
    state.expanded.retain(|path| survives_prune(dir, path, &present));
}

fn survives_prune(dir: &Path, path: &Path, surviving_children: &HashSet<&Path>) -> bool {
    match child_of(dir, path) {
        Some(child) => surviving_children.contains(child.as_path()),
        None => true,
    }
}

fn ancestors_under_root(state: &TreeState, path: &Path) -> Vec<PathBuf> {
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
    ancestors
}

pub fn reveal(state: &mut TreeState, path: &Path, listings: &mut DirectoryListings) -> AppResult<()> {
    ensure_root_loaded(state, listings)?;

    for dir in ancestors_under_root(state, path) {
        expand(state, &dir, listings)?;
    }
    Ok(())
}

struct PageCursor<'a> {
    start: usize,
    end: Option<usize>,
    index: usize,
    rows: &'a mut Vec<TreeRow>,
}

impl PageCursor<'_> {
    fn is_within_window(&self) -> bool {
        match self.end {
            Some(end) => self.index >= self.start && self.index < end,
            None => self.index >= self.start,
        }
    }
}

/// Walks the visible rows in source order but materializes a `TreeRow` only inside the cursor's
/// window. Rows outside it are still walked — `TreeRowPage::total` is the full visible count, so the
/// walk can't stop early — yet they now cost nothing beyond the expanded-set lookup the descent
/// needs anyway, instead of the path/name `String` pair every row used to allocate before being
/// sliced away (§2 H-4).
fn push_page_rows(state: &TreeState, entries: &[Entry], depth: u32, cursor: &mut PageCursor) {
    for entry in entries {
        let expanded = state.expanded.contains(&entry.path);
        if cursor.is_within_window() {
            cursor.rows.push(TreeRow {
                path: entry.path.to_string_lossy().to_string(),
                name: entry.name.clone(),
                kind: entry.kind,
                depth,
                expanded,
                has_children: entry.has_children,
            });
        }
        cursor.index += 1;

        if entry.kind == TreeEntryKind::Directory && expanded {
            if let Some(children) = state.cache.get(&entry.path) {
                push_page_rows(state, children, depth + 1, cursor);
            }
        }
    }
}

pub fn flatten(state: &TreeState) -> Vec<TreeRow> {
    full_page(state).rows
}

/// `limit: None` returns every remaining row from `offset` onward — the caller-side "give me the
/// whole tree" case (`docs/acknowledge/2026-08-19-xa-wiring-cleanup-contract.md` §1.3(7)), which used
/// to be spelled as a `u32::MAX` sentinel instead of a real "no limit" value. The frontend now sends
/// this as `limit: null` directly (`tree.query.ts`'s `treeRowsQueryOptions`) — the old sentinel
/// constant no longer exists on either side.
pub fn rows_page(state: &TreeState, offset: u32, limit: Option<u32>) -> TreeRowPage {
    let start = offset as usize;
    let mut rows = Vec::new();
    let mut cursor = PageCursor {
        start,
        end: limit.map(|limit| start.saturating_add(limit as usize)),
        index: 0,
        rows: &mut rows,
    };

    if let Some(children) = state.cache.get(&state.root) {
        push_page_rows(state, children, 0, &mut cursor);
    }
    let total = cursor.index as u32;

    TreeRowPage { rows, total }
}

pub fn full_page(state: &TreeState) -> TreeRowPage {
    rows_page(state, 0, None)
}

pub fn expanded_paths(state: &TreeState) -> Vec<String> {
    state.expanded.iter().map(|path| path.to_string_lossy().to_string()).collect()
}

pub fn restore_expanded(state: &mut TreeState, paths: Vec<String>) {
    let mut listings = DirectoryListings::default();
    for raw in paths {
        let _ = expand(state, Path::new(&raw), &mut listings);
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

    /// Symlinks are the one place where `file_type()` and `metadata()` could have disagreed, since
    /// `metadata()` on Unix is `symlink_metadata` while `file_type()` reads `readdir`'s `d_type`.
    /// std documents both as non-traversing, so a symlink is a plain row whatever it points at —
    /// this fixture pins that so the syscall-shedding swap (research 3b §2-F) cannot change what
    /// the tree shows.
    #[cfg(unix)]
    fn build_symlink_fixture() -> Fixture {
        let root = std::env::temp_dir().join(format!("taide-tree-link-{}", uuid::Uuid::new_v4()));

        std::fs::create_dir_all(root.join("real-dir").join("nested")).unwrap();
        std::fs::write(root.join("real-file.rs"), "fn main() {}").unwrap();
        std::os::unix::fs::symlink(root.join("real-dir"), root.join("link-to-dir")).unwrap();
        std::os::unix::fs::symlink(root.join("real-file.rs"), root.join("link-to-file")).unwrap();
        std::os::unix::fs::symlink(root.join("gone"), root.join("link-broken")).unwrap();

        Fixture { root }
    }

    #[cfg(unix)]
    #[test]
    fn 심링크는_대상과_무관하게_파일로_표시된다() {
        let fixture = build_symlink_fixture();
        let children = read_children(&fixture.root).unwrap();

        let kind_of = |name: &str| {
            children
                .iter()
                .find(|entry| entry.name == name)
                .unwrap_or_else(|| panic!("{name} 엔트리가 없다"))
                .kind
        };

        assert_eq!(kind_of("real-dir"), TreeEntryKind::Directory);
        assert_eq!(kind_of("real-file.rs"), TreeEntryKind::File);
        assert_eq!(kind_of("link-to-dir"), TreeEntryKind::File, "디렉토리 심링크도 파일 행이다");
        assert_eq!(kind_of("link-to-file"), TreeEntryKind::File);
        assert_eq!(kind_of("link-broken"), TreeEntryKind::File, "끊어진 심링크도 목록에 남는다");
    }

    #[cfg(unix)]
    #[test]
    fn 심링크는_자식이_없는_것으로_취급되고_실제_디렉토리만_펼침_가능하다() {
        let fixture = build_symlink_fixture();
        let children = read_children(&fixture.root).unwrap();

        let has_children_of = |name: &str| {
            children
                .iter()
                .find(|entry| entry.name == name)
                .unwrap_or_else(|| panic!("{name} 엔트리가 없다"))
                .has_children
        };

        assert!(has_children_of("real-dir"));
        assert!(!has_children_of("link-to-dir"), "심링크는 펼침 화살표를 얻지 않는다");
        assert!(!has_children_of("link-broken"));
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
        ensure_root_loaded(&mut state, &mut DirectoryListings::default()).unwrap();

        let rows_before_expand = flatten(&state);
        assert_eq!(rows_before_expand.len(), 3);
        assert!(rows_before_expand.iter().all(|row| row.depth == 0));

        let src_path = fixture.root.join("src");
        expand(&mut state, &src_path, &mut DirectoryListings::default()).unwrap();

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
        ensure_root_loaded(&mut state, &mut DirectoryListings::default()).unwrap();

        let src_path = fixture.root.join("src");
        expand(&mut state, &src_path, &mut DirectoryListings::default()).unwrap();
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
        ensure_root_loaded(&mut state, &mut DirectoryListings::default()).unwrap();

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
        ensure_root_loaded(&mut state, &mut DirectoryListings::default()).unwrap();

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
        ensure_root_loaded(&mut state, &mut DirectoryListings::default()).unwrap();

        let deep_file = fixture.root.join("src").join("utils").join("deep.rs");
        reveal(&mut state, &deep_file, &mut DirectoryListings::default()).unwrap();

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
        ensure_root_loaded(&mut state, &mut DirectoryListings::default()).unwrap();

        let src_path = fixture.root.join("src");
        expand(&mut state, &src_path, &mut DirectoryListings::default()).unwrap();
        let saved = expanded_paths(&state);

        let mut restored = new_tree_state(fixture.root.clone());
        ensure_root_loaded(&mut restored, &mut DirectoryListings::default()).unwrap();
        restore_expanded(&mut restored, saved);

        assert_eq!(flatten(&restored).len(), flatten(&state).len());
    }

    #[test]
    fn invalidate_는_대상_디렉토리만_재조회한다() {
        let fixture = build_fixture();
        let mut state = new_tree_state(fixture.root.clone());
        ensure_root_loaded(&mut state, &mut DirectoryListings::default()).unwrap();

        let src_path = fixture.root.join("src");
        expand(&mut state, &src_path, &mut DirectoryListings::default()).unwrap();
        assert_eq!(flatten(&state).len(), 5);

        std::fs::write(src_path.join("new_file.rs"), "// new").unwrap();
        invalidate(&mut state, &src_path, &mut DirectoryListings::default()).unwrap();

        let rows = flatten(&state);
        assert_eq!(rows.len(), 6);
        assert!(rows.iter().any(|row| row.name == "new_file.rs"));
    }

    #[test]
    fn rows_page_는_창_밖_행을_만들지_않고도_전체_수를_센다() {
        let fixture = build_fixture();
        let mut state = new_tree_state(fixture.root.clone());
        ensure_root_loaded(&mut state, &mut DirectoryListings::default()).unwrap();
        expand(&mut state, &fixture.root.join("src"), &mut DirectoryListings::default()).unwrap();

        let page = rows_page(&state, 1, Some(2));
        let names: Vec<&str> = page.rows.iter().map(|row| row.name.as_str()).collect();

        assert_eq!(names, vec!["utils", "main.rs"], "창 안의 행만 그대로 만들어져야 한다");
        assert_eq!(page.rows[0].depth, 1, "창이 펼쳐진 하위 트리 중간에서 시작해도 깊이가 맞아야 한다");
        assert_eq!(page.total, 5, "창 밖 행도 전체 수에는 세어져야 한다");

        let tail = rows_page(&state, 3, Some(10));
        let tail_names: Vec<&str> = tail.rows.iter().map(|row| row.name.as_str()).collect();
        assert_eq!(tail_names, vec!["Cargo.toml", "README.md"]);
        assert_eq!(tail.total, 5);
    }

    #[test]
    fn 미리_읽은_목록은_잠금_안에서_디스크를_다시_읽지_않는다() {
        let fixture = build_fixture();
        let mut state = new_tree_state(fixture.root.clone());
        let src_path = fixture.root.join("src");
        let mut listings = read_directories(vec![fixture.root.clone(), src_path.clone()]);

        std::fs::remove_dir_all(&src_path).unwrap();
        expand(&mut state, &src_path, &mut listings).unwrap();

        let rows = flatten(&state);
        assert!(
            rows.iter().any(|row| row.name == "main.rs"),
            "디스크에서 사라진 뒤에도 미리 읽은 목록만으로 삽입이 끝나야 한다"
        );
    }

    #[test]
    fn 미리_읽지_못한_디렉토리는_잠금_안에서_같은_오류로_이어진다() {
        let fixture = build_fixture();
        let mut state = new_tree_state(fixture.root.clone());
        let missing = fixture.root.join("gone");

        let mut listings = read_directories(vec![missing.clone()]);
        assert!(
            expand(&mut state, &missing, &mut listings).is_err(),
            "미리 읽기 실패는 조용히 넘어가지 않고 기존과 같은 오류가 되어야 한다"
        );
    }

    #[test]
    fn 읽기_계획은_캐시에_없는_디렉토리만_고른다() {
        let fixture = build_fixture();
        let mut state = new_tree_state(fixture.root.clone());
        let src_path = fixture.root.join("src");
        let utils_path = src_path.join("utils");

        assert_eq!(plan_toggle_reads(&state, &src_path), vec![fixture.root.clone(), src_path.clone()]);

        ensure_root_loaded(&mut state, &mut DirectoryListings::default()).unwrap();
        assert_eq!(plan_toggle_reads(&state, &src_path), vec![src_path.clone()]);
        assert_eq!(
            plan_reveal_reads(&state, &utils_path.join("deep.rs")),
            vec![src_path.clone(), utils_path.clone()]
        );

        expand(&mut state, &src_path, &mut DirectoryListings::default()).unwrap();
        assert!(plan_toggle_reads(&state, &src_path).is_empty(), "접는 토글은 디스크를 읽지 않는다");
        assert_eq!(plan_refresh_reads(&state, &src_path), vec![src_path.clone()]);
        assert!(
            plan_refresh_reads(&state, &utils_path).is_empty(),
            "펼쳐지지 않은 디렉토리의 갱신은 캐시만 버리므로 읽을 것이 없다"
        );
    }

    #[test]
    fn 삭제_후_동명_재생성된_폴더에는_옛_자식이_남지_않는다() {
        let fixture = build_fixture();
        let mut state = new_tree_state(fixture.root.clone());
        ensure_root_loaded(&mut state, &mut DirectoryListings::default()).unwrap();

        let src_path = fixture.root.join("src");
        let utils_path = src_path.join("utils");
        expand(&mut state, &src_path, &mut DirectoryListings::default()).unwrap();
        expand(&mut state, &utils_path, &mut DirectoryListings::default()).unwrap();

        std::fs::remove_dir_all(&src_path).unwrap();
        invalidate(&mut state, &fixture.root, &mut DirectoryListings::default()).unwrap();

        let expanded = expanded_paths(&state);
        assert!(
            !expanded.contains(&src_path.to_string_lossy().to_string()),
            "지운 폴더의 펼침 상태가 남으면 안 된다"
        );
        assert!(
            !expanded.contains(&utils_path.to_string_lossy().to_string()),
            "지운 폴더 아래의 펼침 상태도 함께 정리되어야 한다"
        );

        std::fs::create_dir(&src_path).unwrap();
        std::fs::write(src_path.join("fresh.rs"), "// fresh").unwrap();
        invalidate(&mut state, &fixture.root, &mut DirectoryListings::default()).unwrap();
        expand(&mut state, &src_path, &mut DirectoryListings::default()).unwrap();

        let rows = flatten(&state);
        let child_names: Vec<&str> = rows.iter().filter(|row| row.depth == 1).map(|row| row.name.as_str()).collect();
        assert_eq!(
            child_names,
            vec!["fresh.rs"],
            "동명으로 다시 만든 폴더에 옛 자식이 되살아나면 안 된다"
        );
    }

    #[test]
    fn 비워진_폴더의_캐시된_자식은_상위_갱신에서_버려진다() {
        let fixture = build_fixture();
        let mut state = new_tree_state(fixture.root.clone());
        ensure_root_loaded(&mut state, &mut DirectoryListings::default()).unwrap();

        let src_path = fixture.root.join("src");
        expand(&mut state, &src_path, &mut DirectoryListings::default()).unwrap();
        assert_eq!(flatten(&state).len(), 5);

        std::fs::remove_dir_all(src_path.join("utils")).unwrap();
        std::fs::remove_file(src_path.join("main.rs")).unwrap();
        invalidate(&mut state, &fixture.root, &mut DirectoryListings::default()).unwrap();

        let rows = flatten(&state);
        assert!(rows.iter().all(|row| row.depth == 0), "비워진 폴더가 옛 자식을 계속 그리면 안 된다");
        assert!(
            expanded_paths(&state).contains(&src_path.to_string_lossy().to_string()),
            "폴더 자체는 남아 있으므로 사용자가 연 상태는 유지되어야 한다"
        );
    }

    #[test]
    fn 개명된_폴더의_옛_경로_상태는_상위_갱신에서_사라진다() {
        let fixture = build_fixture();
        let mut state = new_tree_state(fixture.root.clone());
        ensure_root_loaded(&mut state, &mut DirectoryListings::default()).unwrap();

        let src_path = fixture.root.join("src");
        let utils_path = src_path.join("utils");
        expand(&mut state, &src_path, &mut DirectoryListings::default()).unwrap();
        expand(&mut state, &utils_path, &mut DirectoryListings::default()).unwrap();

        let renamed = fixture.root.join("lib");
        std::fs::rename(&src_path, &renamed).unwrap();
        invalidate(&mut state, &fixture.root, &mut DirectoryListings::default()).unwrap();

        let expanded = expanded_paths(&state);
        assert!(!expanded.contains(&src_path.to_string_lossy().to_string()));
        assert!(!expanded.contains(&utils_path.to_string_lossy().to_string()));

        let rows = flatten(&state);
        assert!(rows.iter().any(|row| row.name == "lib" && !row.expanded));
        assert!(rows.iter().all(|row| row.depth == 0), "새 경로는 접힌 채로 새로 읽혀야 한다");
    }

    #[test]
    fn 이름이_접두사인_형제는_정리_대상이_아니다() {
        let fixture = build_fixture();
        let sibling = fixture.root.join("src-old");
        std::fs::create_dir_all(sibling.join("kept")).unwrap();

        let mut state = new_tree_state(fixture.root.clone());
        ensure_root_loaded(&mut state, &mut DirectoryListings::default()).unwrap();
        expand(&mut state, &sibling, &mut DirectoryListings::default()).unwrap();

        let src_path = fixture.root.join("src");
        std::fs::remove_dir_all(&src_path).unwrap();
        invalidate(&mut state, &fixture.root, &mut DirectoryListings::default()).unwrap();

        assert!(
            expanded_paths(&state).contains(&sibling.to_string_lossy().to_string()),
            "지운 폴더와 이름 접두사만 겹치는 형제는 그대로 남아야 한다"
        );
        assert!(flatten(&state).iter().any(|row| row.name == "kept"));
    }

    /// The prefetch plan is what keeps disk IO out of the mutation guard (`architecture.md` §2.1):
    /// whatever the guarded operation reads must have been planned, and a plan that walked outside
    /// the project root would prefetch — and cache — directories the tree can never show.
    #[test]
    fn 루트_밖_경로의_reveal_계획은_루트_읽기에서_멈춘다() {
        let fixture = build_fixture();
        let mut state = new_tree_state(fixture.root.clone());
        let outside = fixture.root.parent().expect("상위 디렉토리").join("elsewhere").join("a.rs");

        assert_eq!(plan_reveal_reads(&state, &outside), vec![fixture.root.clone()]);

        ensure_root_loaded(&mut state, &mut DirectoryListings::default()).unwrap();
        assert!(
            plan_reveal_reads(&state, &outside).is_empty(),
            "루트가 이미 캐시되어 있으면 루트 밖 reveal 은 읽을 것이 없다"
        );
    }

    #[test]
    fn 접기는_캐시를_남겨_다시_펼칠_때_디스크를_읽지_않는다() {
        let fixture = build_fixture();
        let mut state = new_tree_state(fixture.root.clone());
        let src_path = fixture.root.join("src");
        expand(&mut state, &src_path, &mut DirectoryListings::default()).unwrap();

        collapse(&mut state, &src_path);

        assert!(
            !expanded_paths(&state).contains(&src_path.to_string_lossy().to_string()),
            "접기는 펼침 표시만 지운다"
        );
        assert!(
            plan_toggle_reads(&state, &src_path).is_empty(),
            "캐시가 남아 있으므로 다시 펼치는 토글은 잠금 안에서 디스크를 읽지 않는다"
        );

        toggle_expand(&mut state, &src_path, &mut DirectoryListings::default()).unwrap();
        assert!(flatten(&state).iter().any(|row| row.name == "main.rs"));
    }

    #[test]
    fn 복원_대상에_사라진_경로가_섞여_있어도_나머지는_복원된다() {
        let fixture = build_fixture();
        let mut state = new_tree_state(fixture.root.clone());
        let src_path = fixture.root.join("src");
        let gone = fixture.root.join("gone");

        restore_expanded(
            &mut state,
            vec![gone.to_string_lossy().to_string(), src_path.to_string_lossy().to_string()],
        );

        let expanded = expanded_paths(&state);
        assert!(expanded.contains(&src_path.to_string_lossy().to_string()));
        assert!(
            !expanded.contains(&gone.to_string_lossy().to_string()),
            "읽을 수 없는 경로는 펼침 상태로 남지 않는다"
        );
        assert!(flatten(&state).iter().any(|row| row.name == "main.rs"));
    }

    #[test]
    fn 펼침_상태_복원은_기존_상태에_더해진다() {
        let fixture = build_fixture();
        let mut state = new_tree_state(fixture.root.clone());
        let src_path = fixture.root.join("src");
        let utils_path = src_path.join("utils");
        expand(&mut state, &src_path, &mut DirectoryListings::default()).unwrap();

        restore_expanded(&mut state, vec![utils_path.to_string_lossy().to_string()]);

        let expanded = expanded_paths(&state);
        assert!(expanded.contains(&src_path.to_string_lossy().to_string()));
        assert!(expanded.contains(&utils_path.to_string_lossy().to_string()));
    }
}
