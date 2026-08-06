# Rust git2 crate 로 VSCode 수준 Git 기능 구현

조사 기준일: 2026-08-06. 모든 버전·시그니처는 crates.io API, docs.rs(git2 0.21.0), GitHub 원본 파일로 직접 확인했다. 확인하지 못한 항목은 본문에 `미확인`으로 표기했다.

## 버전 확정 (2026-08 기준)

| 항목 | 값 | 확인 경로 |
|---|---|---|
| `git2` 최신 | **0.21.0** (2026-05-18 릴리스) | crates.io API |
| `libgit2-sys` 최신 | **0.18.7+1.9.6** (2026-07-22) | crates.io API |
| 번들 libgit2 | **1.9.6** (버전 문자열 `+` 뒤가 libgit2 버전) | libgit2-sys 버전 명명 규칙 |
| `git2` 0.21.0 의 의존 요구 | `libgit2-sys = "^0.18.4"` (= libgit2 1.9.3 이상) | crates.io dependencies API |
| Rust edition | 2021 (0.21.0 에서 마이그레이션) | CHANGELOG |
| `gix` (gitoxide) 최신 | **0.86.0** (2026-07-23). 0.82.0 은 yank됨 | crates.io API |

`git2 = "0.21"` 을 쓰면 Cargo 가 `libgit2-sys 0.18.7+1.9.6` 을 당겨오므로 실제 링크되는 libgit2 는 1.9.6 이다.

### git2 0.21.0 파괴적 변경 (0.20 → 0.21 업그레이드 시 필독)

CHANGELOG 및 Cargo.toml 로 확인한 내용이다.

1. **기본 feature 가 전부 제거되었다.** `Cargo.toml` 에 `default = []` 로 확정되어 있다. `ssh`, `https`, `cred` 를 명시적으로 켜지 않으면 push/fetch 인증이 통째로 동작하지 않는다. TAIDE 에서 가장 먼저 밟을 함정이다.
2. **문자열 접근자가 `Option` → `Result` 로 바뀌었다.** `Commit::message()` 는 `Result<&str, Error>`, `Commit::summary()` 는 `Result<Option<&str>, Error>`, `StatusEntry::path()` 는 `Result<&str, Error>`, `Remote::url()` 은 `Result<&str, Error>`. "값 없음" 과 "비-UTF-8" 을 구분하기 위한 변경이다.
3. **`BlameHunk` 의 `final_signature` / `final_committer` / `orig_signature` / `orig_committer` 가 `Option` 을 반환한다.** 기존에는 segfault 가능성이 있었다.
4. Rust 2021 edition 으로 이동.

추가된 것: `unstable-sha256` feature(실험적 SHA256 저장소), `Repository::object_format()`, `Repository::set_config()`, `merge_file()` / `MergeFileInput`, `Refdb` 타입, `Reference: Clone`, `BlameHunk` 의 committer/summary 접근자.

### Cargo.toml

```toml
[dependencies]
git2 = { version = "0.21", default-features = false, features = ["ssh", "https", "vendored-libgit2"] }
```

- `ssh` 는 `cred` 를 자동으로 포함하고, `https` 도 `cred` 를 포함한다 (Cargo.toml features 표에서 확인).
- `vendored-libgit2` 는 libgit2 를 소스에서 빌드해 정적 링크한다. Tauri 배포에서 사용자 머신의 libgit2 유무에 의존하지 않으려면 사실상 필수다.
- Linux 배포에서 OpenSSL 동적 링크를 피하려면 `vendored-openssl` 도 추가한다. macOS 는 SecureTransport 를 쓰므로 보통 불필요하다.

전체 feature 목록(Cargo.toml 원문 기준): `unstable`, `unstable-sha256`, `ssh`, `https`, `cred`, `vendored-libgit2`, `vendored-openssl`, `zlib-ng-compat`.

---

## 핵심 API·사용법

아래 시그니처는 전부 docs.rs/git2/0.21.0 에서 직접 추출한 것이다.

### 1. 저장소 열기 (discover / open)

```rust
use git2::{Repository, RepositoryOpenFlags};
use std::path::Path;

// 하위 디렉토리에서 위로 올라가며 .git 을 탐색 — IDE 가 폴더를 열 때 쓸 기본 동작
pub fn discover(start: &Path) -> Result<Repository, git2::Error> {
    Repository::discover(start)
}

// 정확한 경로만 (탐색 안 함)
// Repository::open(path)

// 상승 탐색 범위를 제한하고 싶을 때 (홈 디렉토리 밖으로 못 나가게)
pub fn discover_bounded(start: &Path, ceiling: &Path) -> Result<Repository, git2::Error> {
    Repository::open_ext(
        start,
        RepositoryOpenFlags::CROSS_FS,
        &[ceiling],
    )
}
```

확정 시그니처:

```rust
pub fn discover<P: AsRef<Path>>(path: P) -> Result<Repository, Error>
pub fn open<P: AsRef<Path>>(path: P) -> Result<Repository, Error>
pub fn open_ext<P, O, I>(path: P, flags: RepositoryOpenFlags, ceiling_dirs: I) -> Result<Repository, Error>
```

`Repository::discover` 는 bare 저장소와 `.git` 파일(worktree/submodule)을 모두 처리한다. `repo.workdir()` 가 `None` 이면 bare 저장소이므로 TAIDE 의 SCM 뷰는 비활성화해야 한다.

### 2. statuses() → VSCode M/A/D/U/R/C 매핑

`Status` 비트플래그 전체 목록(docs.rs 확인):

```
CURRENT
INDEX_NEW, INDEX_MODIFIED, INDEX_DELETED, INDEX_RENAMED, INDEX_TYPECHANGE
WT_NEW, WT_MODIFIED, WT_DELETED, WT_TYPECHANGE, WT_RENAMED, WT_UNREADABLE
IGNORED, CONFLICTED
```

`StatusShow` enum 변형은 `Index`, `Workdir`, `IndexAndWorkdir` 3개다.

핵심 개념: **`Status` 는 비트플래그이므로 한 파일이 staged 와 unstaged 를 동시에 가질 수 있다.** `INDEX_MODIFIED | WT_MODIFIED` 인 파일은 VSCode 에서 "Staged Changes" 와 "Changes" 양쪽에 각각 나타난다. 따라서 파일 1개 → 상태 1개가 아니라, 파일 1개 → (staged 상태 Option, unstaged 상태 Option) 로 모델링해야 한다.

```rust
use git2::{Repository, Status, StatusOptions, StatusShow};
use serde::Serialize;

#[derive(Serialize, Clone, Copy, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum FileChange {
    Modified,   // M
    Added,      // A
    Deleted,    // D
    Renamed,    // R
    Copied,     // C  (statuses() 로는 나오지 않음 — 아래 주의 참고)
    Untracked,  // U
    TypeChange, // T
    Conflicted, // !
    Ignored,
}

#[derive(Serialize)]
pub struct StatusRow {
    pub path: String,
    pub orig_path: Option<String>, // rename 원본
    pub staged: Option<FileChange>,
    pub unstaged: Option<FileChange>,
    pub is_conflicted: bool,
}

fn staged_of(s: Status) -> Option<FileChange> {
    if s.contains(Status::INDEX_NEW) { Some(FileChange::Added) }
    else if s.contains(Status::INDEX_MODIFIED) { Some(FileChange::Modified) }
    else if s.contains(Status::INDEX_DELETED) { Some(FileChange::Deleted) }
    else if s.contains(Status::INDEX_RENAMED) { Some(FileChange::Renamed) }
    else if s.contains(Status::INDEX_TYPECHANGE) { Some(FileChange::TypeChange) }
    else { None }
}

fn unstaged_of(s: Status) -> Option<FileChange> {
    if s.contains(Status::CONFLICTED) { Some(FileChange::Conflicted) }
    else if s.contains(Status::WT_NEW) { Some(FileChange::Untracked) }
    else if s.contains(Status::WT_MODIFIED) { Some(FileChange::Modified) }
    else if s.contains(Status::WT_DELETED) { Some(FileChange::Deleted) }
    else if s.contains(Status::WT_RENAMED) { Some(FileChange::Renamed) }
    else if s.contains(Status::WT_TYPECHANGE) { Some(FileChange::TypeChange) }
    else { None }
}

pub fn collect_status(repo: &Repository) -> Result<Vec<StatusRow>, git2::Error> {
    let mut opts = StatusOptions::new();
    opts.show(StatusShow::IndexAndWorkdir)
        .include_untracked(true)
        .recurse_untracked_dirs(false)   // 디렉토리 단위로 접어서 보고 — 대형 리포 필수
        .include_ignored(false)
        .include_unmodified(false)
        .exclude_submodules(false)
        .renames_head_to_index(true)
        .renames_index_to_workdir(true)
        .update_index(true);             // stat 캐시 갱신 → 다음 호출 가속

    let statuses = repo.statuses(Some(&mut opts))?;

    let mut rows = Vec::with_capacity(statuses.len());
    for entry in statuses.iter() {
        let s = entry.status();
        if s.contains(Status::IGNORED) { continue; }

        // path() 는 0.21 에서 Result<&str, Error> — 비-UTF8 경로는 건너뛴다
        let Ok(path) = entry.path() else { continue };

        // rename 원본 경로는 delta 에서 꺼낸다
        let orig_path = entry
            .head_to_index()
            .or_else(|| entry.index_to_workdir())
            .and_then(|d| d.old_file().path().map(|p| p.to_string_lossy().into_owned()))
            .filter(|p| p != path);

        rows.push(StatusRow {
            path: path.to_owned(),
            orig_path,
            staged: staged_of(s),
            unstaged: unstaged_of(s),
            is_conflicted: s.contains(Status::CONFLICTED),
        });
    }
    Ok(rows)
}
```

`StatusEntry` 접근자(확인됨):

```rust
pub fn path_bytes(&self) -> &[u8]
pub fn path(&self) -> Result<&str, Error>
pub fn status(&self) -> Status
pub fn head_to_index(&self) -> Option<DiffDelta<'statuses>>
pub fn index_to_workdir(&self) -> Option<DiffDelta<'statuses>>
```

**C(Copied) 에 대한 중요한 사실**: `Status` 비트플래그에는 `INDEX_COPIED` / `WT_COPIED` 가 **없다**. Copy 검출은 `Delta::Copied` 로만 나오며, 이는 diff 에 `find_similar()` 를 적용해야 얻는다. `statuses()` 만으로는 C 를 절대 만들 수 없다. VSCode 도 실무에서 C 를 거의 표시하지 않는 이유가 이것이다.

`Delta` enum 전체 변형(11개, 확인됨): `Unmodified`, `Added`, `Deleted`, `Modified`, `Renamed`, `Copied`, `Ignored`, `Untracked`, `Typechange`, `Unreadable`, `Conflicted`.

### 3. Diff 3종 + gutter 용 라인 정보 추출

세 가지 diff 축(확인된 시그니처):

```rust
// staged 변경분:  HEAD tree ↔ index
pub fn diff_tree_to_index(&self, tree: &Tree, index: &Index, opts: Option<&mut DiffOptions>) -> Result<Diff, Error>
// unstaged 변경분: index ↔ workdir
pub fn diff_index_to_workdir(&self, index: &Index, opts: Option<&mut DiffOptions>) -> Result<Diff, Error>
// 총 변경분(gutter 용): HEAD tree ↔ workdir, index 를 경유해 정확도 확보
pub fn diff_tree_to_workdir_with_index(&self, tree: &Tree, opts: Option<&mut DiffOptions>) -> Result<Diff, Error>
pub fn diff_tree_to_workdir(&self, tree: &Tree, opts: Option<&mut DiffOptions>) -> Result<Diff, Error>
```

gutter 에는 `diff_tree_to_workdir_with_index` 를 쓴다. `diff_tree_to_workdir` 은 index 를 무시해서 staged 후 편집한 파일의 결과가 실제 git 과 어긋난다.

```rust
use git2::{Diff, DiffOptions, Repository};
use serde::Serialize;

#[derive(Serialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum GutterKind { Added, Modified, Deleted }

#[derive(Serialize)]
pub struct GutterHunk {
    pub kind: GutterKind,
    pub start: u32,  // 1-based, 신규 파일(workdir) 기준 라인
    pub end: u32,    // inclusive
}

pub fn gutter_for_file(repo: &Repository, rel_path: &str) -> Result<Vec<GutterHunk>, git2::Error> {
    let head_tree = repo.head()?.peel_to_commit()?.tree()?;

    let mut opts = DiffOptions::new();
    opts.pathspec(rel_path)
        .context_lines(0)          // gutter 에는 컨텍스트가 필요 없다
        .include_untracked(true)
        .show_untracked_content(true)
        .indent_heuristic(true);

    let diff = repo.diff_tree_to_workdir_with_index(Some(&head_tree), Some(&mut opts))?;

    let mut out = Vec::new();
    // hunk 단위만으로 gutter 3분류가 완성된다 — line 콜백 불필요
    diff.foreach(
        &mut |_delta, _progress| true,
        None,
        Some(&mut |_delta, hunk| {
            let (os, ns, nl) = (hunk.old_lines(), hunk.new_start(), hunk.new_lines());
            let h = if os == 0 {
                GutterHunk { kind: GutterKind::Added, start: ns, end: ns + nl - 1 }
            } else if nl == 0 {
                // 삭제는 신규 파일에 자리가 없다 → 남은 라인 경계에 마커를 찍는다
                GutterHunk { kind: GutterKind::Deleted, start: ns.max(1), end: ns.max(1) }
            } else {
                GutterHunk { kind: GutterKind::Modified, start: ns, end: ns + nl - 1 }
            };
            out.push(h);
            true
        }),
        None,
    )?;
    Ok(out)
}
```

`Diff::foreach` 확정 시그니처 — 콜백이 제네릭이 아니라 `&mut dyn FnMut` 이라는 점이 중요하다. 클로저를 직접 넘기면 컴파일되지 않고, 반드시 `&mut |...| {...}` 형태로 넘겨야 한다.

```rust
pub fn foreach(
    &self,
    file_cb: &mut (dyn FnMut(DiffDelta<'_>, f32) -> bool + '_),
    binary_cb: Option<&mut (dyn FnMut(DiffDelta<'_>, DiffBinary<'_>) -> bool + '_)>,
    hunk_cb: Option<&mut (dyn FnMut(DiffDelta<'_>, DiffHunk<'_>) -> bool + '_)>,
    line_cb: Option<&mut (dyn FnMut(DiffDelta<'_>, Option<DiffHunk<'_>>, DiffLine<'_>) -> bool + '_)>,
) -> Result<(), Error>
```

콜백에서 `false` 를 반환하면 순회가 중단되고 `Error` 로 돌아온다(취소 신호). 정상 완주하려면 항상 `true`.

라인 단위 패치 텍스트(diff 뷰어용)가 필요하면 `line_cb` 또는 `print` 를 쓴다:

```rust
pub fn print<F>(&self, format: DiffFormat, cb: F) -> Result<(), Error>
where F: FnMut(DiffDelta<'_>, Option<DiffHunk<'_>>, DiffLine<'_>) -> bool
```

`DiffHunk` 접근자(확인됨): `old_start() -> u32`, `old_lines() -> u32`, `new_start() -> u32`, `new_lines() -> u32`, `header() -> &[u8]`.

`DiffLine` 접근자(확인됨):

```rust
pub fn old_lineno(&self) -> Option<u32>   // 추가된 줄이면 None
pub fn new_lineno(&self) -> Option<u32>   // 삭제된 줄이면 None
pub fn num_lines(&self) -> u32
pub fn content(&self) -> &'a [u8]
pub fn content_offset(&self) -> i64
pub fn origin(&self) -> char              // '-', '+', '=', '>', '<', 'F', 'H', 'B'
pub fn origin_value(&self) -> DiffLineType
```

`content()` 는 개행을 **포함**한 바이트다. 프론트로 넘길 때 `trim_end_matches(['\n','\r'])` 처리를 잊지 말 것.

rename/copy 검출이 필요하면 diff 생성 후:

```rust
let mut diff = repo.diff_tree_to_index(Some(&head_tree), Some(&index), Some(&mut opts))?;
let mut find = git2::DiffFindOptions::new();
find.renames(true).copies(true);
diff.find_similar(Some(&mut find))?;   // &mut self 필요
```

유용한 `DiffOptions` 항목(확인됨): `context_lines`, `interhunk_lines`, `ignore_whitespace`, `ignore_whitespace_change`, `ignore_whitespace_eol`, `ignore_blank_lines`, `patience`, `minimal`, `indent_heuristic`, `force_text`, `force_binary`, `max_size`, `pathspec`, `skip_binary_check`, `update_index`, `show_untracked_content`.

### 4. Blame (인라인 blame)

```rust
pub fn blame_file(&self, path: &Path, opts: Option<&mut BlameOptions>) -> Result<Blame<'_>, Error>
```

`Blame` 메서드(확인됨): `len()`, `is_empty()`, `get_index(usize) -> Option<BlameHunk>`, `get_line(lineno: usize) -> Option<BlameHunk>`, `iter() -> BlameIter`, `blame_buffer(&self, buffer: &[u8]) -> Result<Blame, Error>`.

`BlameOptions` 메서드(확인됨): `new`, `track_copies_same_file`, `track_copies_same_commit_moves`, `track_copies_same_commit_copies`, `track_copies_any_commit_copies`, `first_parent`, `use_mailmap`, `ignore_whitespace`, `newest_commit`, `oldest_commit`, `min_line`, `max_line`.

```rust
use git2::{BlameOptions, Repository};
use serde::Serialize;

#[derive(Serialize)]
pub struct BlameLine {
    pub line: u32,
    pub commit: String,
    pub short_commit: String,
    pub author: Option<String>,
    pub email: Option<String>,
    pub time_unix: i64,
    pub is_boundary: bool,
}

pub fn blame_range(
    repo: &Repository,
    rel_path: &str,
    min_line: usize,
    max_line: usize,
) -> Result<Vec<BlameLine>, git2::Error> {
    let mut opts = BlameOptions::new();
    opts.first_parent(true)        // 머지 커밋을 파고들지 않아 크게 빨라진다
        .use_mailmap(true)
        .ignore_whitespace(true)
        .min_line(min_line)        // 뷰포트만 blame — 결과 크기를 줄인다
        .max_line(max_line);

    let blame = repo.blame_file(std::path::Path::new(rel_path), Some(&mut opts))?;

    let mut out = Vec::new();
    for hunk in blame.iter() {
        let oid = hunk.final_commit_id();
        // 0.21 부터 Option 반환 — unwrap 금지
        let sig = hunk.final_signature();
        let (author, email, time_unix) = match &sig {
            Some(s) => (
                s.name().map(str::to_owned),
                s.email().map(str::to_owned),
                s.when().seconds(),
            ),
            None => (None, None, 0),
        };
        let start = hunk.final_start_line() as u32;
        for i in 0..hunk.lines_in_hunk() as u32 {
            out.push(BlameLine {
                line: start + i,
                commit: oid.to_string(),
                short_commit: oid.to_string()[..7].to_owned(),
                author: author.clone(),
                email: email.clone(),
                time_unix,
                is_boundary: hunk.is_boundary(),
            });
        }
    }
    Ok(out)
}
```

**저장되지 않은 버퍼의 blame**: `Blame::blame_buffer(&self, buffer: &[u8])` 로 처리한다. 먼저 `blame_file` 로 기준 blame 을 만든 뒤, 에디터의 현재 버퍼 바이트를 넘기면 수정된 라인은 "Not Committed Yet" 에 해당하는 zero OID 훅으로 나온다. 이것이 VSCode/GitLens 의 인라인 blame 이 편집 중에도 동작하는 방식이며, 매 키 입력마다 `blame_file` 을 재실행하면 안 된다.

**성능 주의**: libgit2 의 blame 은 파일 히스토리 전체를 되짚는 O(히스토리) 연산이고, 증분 blame API 가 없다. 수천 커밋 규모 파일에서 수백 ms ~ 수 초가 나온다. 반드시 `spawn_blocking` 으로 빼고, 커서 이동 시 debounce 하고, `(path, HEAD oid)` 키로 캐시해야 한다. `min_line`/`max_line` 은 반환 결과를 좁힐 뿐 히스토리 순회 자체를 크게 줄여 주지는 않는다는 점에 유의한다.

### 5. Revwalk — 커밋 로그 + 그래프

```rust
pub fn revwalk(&self) -> Result<Revwalk, Error>
// Revwalk
pub fn push_head(&mut self) -> Result<(), Error>
pub fn push(&mut self, oid: Oid) -> Result<(), Error>
pub fn push_glob(&mut self, glob: &str) -> Result<(), Error>
pub fn push_range(&mut self, range: &str) -> Result<(), Error>
pub fn push_ref(&mut self, reference: &str) -> Result<(), Error>
pub fn hide(&mut self, oid: Oid) -> Result<(), Error>
pub fn set_sorting(&mut self, sort_mode: Sort) -> Result<(), Error>
pub fn simplify_first_parent(&mut self) -> Result<(), Error>
pub fn reset(&mut self) -> Result<(), Error>
```

`Sort` 상수(확인됨): `NONE`, `TOPOLOGICAL`, `TIME`, `REVERSE`.

```rust
use git2::{Repository, Sort};
use serde::Serialize;

#[derive(Serialize)]
pub struct LogEntry {
    pub id: String,
    pub parents: Vec<String>,   // 그래프 렌더의 핵심
    pub summary: Option<String>,
    pub author: Option<String>,
    pub email: Option<String>,
    pub time_unix: i64,
}

pub fn commit_log(repo: &Repository, skip: usize, take: usize) -> Result<Vec<LogEntry>, git2::Error> {
    let mut walk = repo.revwalk()?;
    // 그래프 렌더에는 TOPOLOGICAL | TIME 조합이 정답이다.
    // TIME 만 쓰면 시계가 어긋난 커밋에서 부모가 자식보다 먼저 나와 그래프가 꼬인다.
    walk.set_sorting(Sort::TOPOLOGICAL | Sort::TIME)?;
    walk.push_head()?;
    // 모든 브랜치를 포함하려면: walk.push_glob("refs/heads/*")?;

    let mut out = Vec::with_capacity(take);
    for oid in walk.skip(skip).take(take) {
        let oid = oid?;
        let c = repo.find_commit(oid)?;
        let a = c.author();
        out.push(LogEntry {
            id: oid.to_string(),
            parents: (0..c.parent_count())
                .filter_map(|i| c.parent_id(i).ok().map(|p| p.to_string()))
                .collect(),
            // summary() 는 Result<Option<&str>, Error>
            summary: c.summary().ok().flatten().map(str::to_owned),
            author: a.name().map(str::to_owned),
            email: a.email().map(str::to_owned),
            time_unix: c.time().seconds(),
        });
    }
    Ok(out)
}
```

`Revwalk` 는 `Iterator<Item = Result<Oid, Error>>` 이므로 `skip`/`take` 로 페이지네이션이 그대로 된다. 10만 커밋 리포에서도 필요한 페이지만 walk 하므로 저렴하다.

브랜치/태그 목록:

```rust
use git2::{BranchType, Repository};

pub fn refs_snapshot(repo: &Repository) -> Result<(Vec<String>, Vec<String>, Vec<String>), git2::Error> {
    let mut locals = Vec::new();
    for b in repo.branches(Some(BranchType::Local))? {
        let (b, _) = b?;
        if let Ok(Some(n)) = b.name() { locals.push(n.to_owned()); }
    }

    let mut remotes = Vec::new();
    for b in repo.branches(Some(BranchType::Remote))? {
        let (b, _) = b?;
        if let Ok(Some(n)) = b.name() { remotes.push(n.to_owned()); }
    }

    // tag_names 는 StringArray 를 반환 — Option<&str> 를 내놓는다
    let tags: Vec<String> = repo.tag_names(None)?.iter().flatten().map(str::to_owned).collect();

    Ok((locals, remotes, tags))
}
```

ahead/behind 배지(VSCode 상태바의 ↑1↓2):

```rust
pub fn ahead_behind(repo: &Repository) -> Result<(usize, usize), git2::Error> {
    let head = repo.head()?;
    let local = head.target().ok_or_else(|| git2::Error::from_str("detached/unborn HEAD"))?;
    let branch = git2::Branch::wrap(head);
    let upstream = branch.upstream()?;
    let up_oid = upstream.get().target().ok_or_else(|| git2::Error::from_str("no upstream target"))?;
    repo.graph_ahead_behind(local, up_oid)   // -> (ahead, behind)
}
```

### 6. Stage / Unstage

```rust
use git2::{IndexAddOption, Repository};
use std::path::Path;

pub fn stage(repo: &Repository, paths: &[String]) -> Result<(), git2::Error> {
    let mut index = repo.index()?;
    for p in paths {
        let path = Path::new(p);
        // 삭제된 파일은 add_path 가 실패한다 → remove_path 로 분기
        if repo.workdir().map(|w| w.join(path).exists()).unwrap_or(false) {
            index.add_path(path)?;
        } else {
            index.remove_path(path)?;
        }
    }
    index.write()  // 디스크 반영 필수. 잊으면 아무 일도 일어나지 않는다.
}

// Stage All
pub fn stage_all(repo: &Repository) -> Result<(), git2::Error> {
    let mut index = repo.index()?;
    index.add_all(["*"], IndexAddOption::DEFAULT, None)?;
    index.write()
}

// Unstage — HEAD tree 의 해당 경로 엔트리로 index 를 되돌린다 (워킹트리는 건드리지 않음)
pub fn unstage(repo: &Repository, paths: &[String]) -> Result<(), git2::Error> {
    match repo.head() {
        Ok(head) => {
            let obj = head.peel(git2::ObjectType::Commit)?;
            repo.reset_default(Some(&obj), paths)
        }
        // unborn HEAD (최초 커밋 전): target=None 이면 엔트리를 index 에서 제거
        Err(_) => repo.reset_default(None, paths),
    }
}
```

확정 시그니처:

```rust
pub fn reset_default<T, I>(&self, target: Option<&Object<'_>>, paths: I) -> Result<(), Error>
where T: IntoCString, I: IntoIterator<Item = T>
pub fn add_path(&mut self, path: &Path) -> Result<(), Error>
pub fn add_all<T, I>(&mut self, pathspecs: I, flag: IndexAddOption, cb: Option<&mut IndexMatchedPath<'_>>) -> Result<(), Error>
pub fn remove_path(&mut self, path: &Path) -> Result<(), Error>
pub fn write(&mut self) -> Result<(), Error>
pub fn write_tree(&mut self) -> Result<Oid, Error>
```

`add_path` 의 경로는 반드시 **워크디렉토리 기준 상대 경로**여야 한다. 프론트에서 절대 경로가 넘어오면 `strip_prefix(repo.workdir())` 로 변환할 것.

### 7. Commit 생성

```rust
use git2::{Repository, Signature};

pub fn commit(repo: &Repository, message: &str) -> Result<String, git2::Error> {
    let mut index = repo.index()?;
    let tree_oid = index.write_tree()?;      // index → tree 객체 작성
    let tree = repo.find_tree(tree_oid)?;

    // git config 의 user.name / user.email 을 사용. 미설정이면 에러 → UI 로 안내해야 한다.
    let sig: Signature<'static> = repo.signature()?;

    // unborn HEAD 면 부모 없음(최초 커밋)
    let parents: Vec<git2::Commit> = match repo.head() {
        Ok(h) => vec![h.peel_to_commit()?],
        Err(_) => vec![],
    };
    let parent_refs: Vec<&git2::Commit> = parents.iter().collect();

    let oid = repo.commit(
        Some("HEAD"),   // 이 ref 를 자동 갱신
        &sig,           // author
        &sig,           // committer
        message,
        &tree,
        &parent_refs,
    )?;

    Ok(oid.to_string())
}
```

확정 시그니처:

```rust
pub fn commit(&self, update_ref: Option<&str>, author: &Signature, committer: &Signature,
              message: &str, tree: &Tree, parents: &[&Commit]) -> Result<Oid, Error>
pub fn signature(&self) -> Result<Signature<'static>, Error>
```

Amend 는 `Commit::amend(...)` 를 쓴다. GPG/SSH 서명 커밋은 2단계다(둘 다 존재 확인됨):

```rust
pub fn commit_create_buffer(&self, author: &Signature, committer: &Signature,
                            message: &str, tree: &Tree, parents: &[&Commit]) -> Result<Buf, Error>
pub fn commit_signed(&self, commit_content: &str, signature: &str,
                     signature_field: Option<&str>) -> Result<Oid, Error>
```

libgit2 는 서명을 **직접 만들지 않는다.** `commit_create_buffer` 로 얻은 내용을 외부 `gpg`/`ssh-keygen` 프로세스에 넘겨 서명 문자열을 받은 뒤 `commit_signed` 로 커밋 객체를 만들고, HEAD 는 별도로 `repo.reference(...)` 로 갱신해야 한다(`commit_signed` 는 ref 를 갱신하지 않는다).

### 8. Push + 인증

```rust
pub fn push<Str: AsRef<str> + IntoCString + Clone>(
    &mut self, refspecs: &[Str], opts: Option<&mut PushOptions<'_>>,
) -> Result<(), Error>
```

`Cred` 생성자 전체(확인됨):

```rust
pub fn default() -> Result<Cred, Error>
pub fn username(username: &str) -> Result<Cred, Error>
pub fn ssh_key_from_agent(username: &str) -> Result<Cred, Error>
pub fn ssh_key(username: &str, publickey: Option<&Path>, privatekey: &Path, passphrase: Option<&str>) -> Result<Cred, Error>
pub fn ssh_key_from_memory(username: &str, publickey: Option<&str>, privatekey: &str, passphrase: Option<&str>) -> Result<Cred, Error>
pub fn userpass_plaintext(username: &str, password: &str) -> Result<Cred, Error>
pub fn credential_helper(config: &Config, url: &str, username: Option<&str>) -> Result<Cred, Error>
```

`RemoteCallbacks::credentials` 콜백 시그니처(확인됨):

```rust
pub fn credentials<F>(&mut self, cb: F) -> &mut RemoteCallbacks<'a>
where F: FnMut(&str, Option<&str>, CredentialType) -> Result<Cred, Error> + 'a
```

실전 구현 — libgit2 는 인증 실패 시 **같은 콜백을 여러 번 재호출**한다. 상태 없이 짜면 무한 루프에 빠지므로 시도한 방식을 기록해야 한다.

```rust
use git2::{Cred, CredentialType, PushOptions, RemoteCallbacks, Repository};
use std::cell::RefCell;

pub struct PushAuth {
    pub pat: Option<String>,          // HTTPS Personal Access Token
    pub ssh_key_path: Option<String>, // 명시적 키 경로 (agent 실패 시 폴백)
    pub ssh_passphrase: Option<String>,
}

pub fn push_branch(
    repo: &Repository,
    remote_name: &str,
    branch: &str,
    auth: &PushAuth,
) -> Result<(), git2::Error> {
    let mut remote = repo.find_remote(remote_name)?;
    let cfg = repo.config()?;

    // 시도한 방식을 비트로 기록해 재시도 루프를 차단한다
    let tried = RefCell::new(0u8);
    const T_SSH_AGENT: u8 = 1;
    const T_SSH_KEY: u8   = 2;
    const T_HELPER: u8    = 4;
    const T_PAT: u8       = 8;

    let mut cbs = RemoteCallbacks::new();
    cbs.credentials(move |url, username_from_url, allowed| {
        let mut t = tried.borrow_mut();

        // SSH: 서버가 먼저 username 을 요구하는 단계가 따로 있다
        if allowed.contains(CredentialType::USERNAME) {
            return Cred::username(username_from_url.unwrap_or("git"));
        }

        if allowed.contains(CredentialType::SSH_KEY) {
            let user = username_from_url.unwrap_or("git");
            if *t & T_SSH_AGENT == 0 {
                *t |= T_SSH_AGENT;
                if let Ok(c) = Cred::ssh_key_from_agent(user) { return Ok(c); }
            }
            if *t & T_SSH_KEY == 0 {
                *t |= T_SSH_KEY;
                if let Some(k) = &auth.ssh_key_path {
                    return Cred::ssh_key(
                        user, None, std::path::Path::new(k), auth.ssh_passphrase.as_deref(),
                    );
                }
            }
            return Err(git2::Error::from_str("SSH 인증 수단이 모두 실패했습니다"));
        }

        if allowed.contains(CredentialType::USER_PASS_PLAINTEXT) {
            // 1순위: OS 자격증명 헬퍼(keychain / credential manager)
            if *t & T_HELPER == 0 {
                *t |= T_HELPER;
                if let Ok(c) = Cred::credential_helper(&cfg, url, username_from_url) { return Ok(c); }
            }
            // 2순위: 사용자가 UI 에 입력한 PAT
            if *t & T_PAT == 0 {
                *t |= T_PAT;
                if let Some(pat) = &auth.pat {
                    // GitHub/GitLab: username 은 아무 값이나 가능, password 에 PAT
                    return Cred::userpass_plaintext("git", pat);
                }
            }
            return Err(git2::Error::from_str("HTTPS 자격증명이 없습니다. PAT 을 등록하세요"));
        }

        if allowed.contains(CredentialType::DEFAULT) {
            return Cred::default();
        }
        Err(git2::Error::from_str("지원하지 않는 인증 방식입니다"))
    });

    // 서버가 거부한 ref 는 push() 자체는 성공으로 끝나므로 여기서 잡아야 한다
    cbs.push_update_reference(|refname, status| match status {
        None => Ok(()),
        Some(msg) => Err(git2::Error::from_str(&format!("{refname} 거부됨: {msg}"))),
    });

    let mut opts = PushOptions::new();
    opts.remote_callbacks(cbs);

    let refspec = format!("refs/heads/{branch}:refs/heads/{branch}");
    remote.push(&[refspec.as_str()], Some(&mut opts))
}
```

인증 실패 판별:

```rust
use git2::{ErrorClass, ErrorCode};

pub fn is_auth_failure(e: &git2::Error) -> bool {
    e.class() == ErrorClass::Http && e.code() == ErrorCode::Auth
        || e.class() == ErrorClass::Ssh
        || e.code() == ErrorCode::Auth
}
```

`push()` 가 `Ok` 를 반환해도 개별 ref 는 거부됐을 수 있다(non-fast-forward, 보호 브랜치). 위처럼 `push_update_reference` 를 반드시 등록하고, 여기서 에러를 반환하면 그 에러가 `push()` 의 `Err` 로 전파된다.

기타 유용한 콜백(확인됨): `transfer_progress(FnMut(Progress) -> bool)`, `sideband_progress(FnMut(&[u8]) -> bool)` — 서버의 remote 메시지를 UI 에 흘려보낼 때 쓴다. `certificate_check(FnMut(&Cert, &str) -> Result<CertificateCheckStatus, Error>)`.

### 9. 파일 단위 Discard

VSCode 의 "Discard Changes" 는 상태에 따라 동작이 갈린다.

```rust
use git2::{build::CheckoutBuilder, Repository};

// 1) unstaged 변경 되돌리기: index 내용으로 워킹트리 덮어쓰기
pub fn discard_unstaged(repo: &Repository, paths: &[String]) -> Result<(), git2::Error> {
    let mut co = CheckoutBuilder::new();
    co.force();                    // 필수. 없으면 dirty 파일을 건드리지 않는다
    co.remove_untracked(false);
    for p in paths { co.path(p); }
    repo.checkout_index(None, Some(&mut co))  // None = 저장소의 현재 index
}

// 2) staged 까지 전부 되돌리기: HEAD 로 index + 워킹트리 복원
pub fn discard_all(repo: &Repository, paths: &[String]) -> Result<(), git2::Error> {
    let mut co = CheckoutBuilder::new();
    co.force().update_index(true);
    for p in paths { co.path(p); }
    repo.checkout_head(Some(&mut co))
}

// 3) untracked 파일은 git 이 아니라 파일시스템에서 지운다 — checkout 은 손대지 않는다
pub fn delete_untracked(repo: &Repository, rel: &str) -> std::io::Result<()> {
    let full = repo.workdir().expect("bare repo").join(rel);
    if full.is_dir() { std::fs::remove_dir_all(full) } else { std::fs::remove_file(full) }
}
```

확정 시그니처:

```rust
pub fn checkout_head(&self, opts: Option<&mut CheckoutBuilder>) -> Result<(), Error>
pub fn checkout_index(&self, index: Option<&mut Index>, opts: Option<&mut CheckoutBuilder>) -> Result<(), Error>
pub fn checkout_tree(&self, treeish: &Object, opts: Option<&mut CheckoutBuilder>) -> Result<(), Error>
```

`CheckoutBuilder::force()` 를 빼먹으면 조용히 아무 일도 일어나지 않는다(기본은 safe 모드라 수정된 파일을 보호한다). Discard 기능의 1순위 버그 원인이다.

### 10. Stash

```rust
pub fn stash_save(&mut self, stasher: &Signature<'_>, message: &str, flags: Option<StashFlags>) -> Result<Oid, Error>
pub fn stash_save2(&mut self, stasher: &Signature<'_>, message: Option<&str>, flags: Option<StashFlags>) -> Result<Oid, Error>
pub fn stash_save_ext(&mut self, opts: Option<&mut StashSaveOptions<'_>>) -> Result<Oid, Error>
pub fn stash_apply(&mut self, index: usize, opts: Option<&mut StashApplyOptions<'_>>) -> Result<(), Error>
pub fn stash_pop(&mut self, index: usize, opts: Option<&mut StashApplyOptions<'_>>) -> Result<(), Error>
pub fn stash_drop(&mut self, index: usize) -> Result<(), Error>
pub fn stash_foreach<T>(&mut self, cb: T) -> Result<(), Error>
```

**stash 계열은 전부 `&mut self` 를 요구한다.** 다른 API 는 대부분 `&self` 이므로, TAIDE 의 저장소 핸들을 `&Repository` 로만 들고 다니면 stash 에서 컴파일이 막힌다. 설계 단계에서 `Mutex<Repository>` 등으로 가변 접근 경로를 확보해야 한다.

`StashFlags` 상수(확인됨): `DEFAULT`, `KEEP_INDEX`, `INCLUDE_UNTRACKED`, `INCLUDE_IGNORED`, `KEEP_ALL`.

```rust
use git2::{Repository, StashFlags};

pub fn stash_push(repo: &mut Repository, message: Option<&str>, keep_index: bool) -> Result<String, git2::Error> {
    let sig = repo.signature()?;
    let mut flags = StashFlags::INCLUDE_UNTRACKED;
    if keep_index { flags |= StashFlags::KEEP_INDEX; }
    let oid = repo.stash_save2(&sig, message, Some(flags))?;
    Ok(oid.to_string())
}

#[derive(serde::Serialize)]
pub struct StashItem { pub index: usize, pub message: String, pub id: String }

pub fn stash_list(repo: &mut Repository) -> Result<Vec<StashItem>, git2::Error> {
    let mut out = Vec::new();
    repo.stash_foreach(|index, message, oid| {
        out.push(StashItem { index, message: message.to_owned(), id: oid.to_string() });
        true   // false 면 순회 중단
    })?;
    Ok(out)
}
```

`stash_foreach` 의 콜백은 `FnMut(usize, &str, &Oid) -> bool` 이다. `stash_drop` 후에는 뒤쪽 인덱스가 전부 당겨지므로, 여러 개를 지울 때는 **인덱스 내림차순**으로 처리해야 한다.

---

## TAIDE 적용 가이드

### 스레드 모델 — 가장 먼저 정할 것

**`Repository` 는 `Send` 이지만 `!Sync` 다** (docs.rs 에서 `impl !Sync for Repository` 확인). Tauri command 는 여러 스레드에서 동시 호출되므로 `State<Repository>` 는 컴파일되지 않는다.

`Mutex<Repository>` 는 `Repository: Send` 이므로 `Sync` 가 되어 `State` 에 넣을 수 있다. 다만 blame 처럼 수 초가 걸리는 작업이 뮤텍스를 잡으면 status 갱신까지 멈춘다. 권장 구조:

```rust
use std::sync::Mutex;
use tauri::State;

pub struct GitState {
    pub repo: Mutex<Option<Repository>>,   // stash 등 &mut 작업용, 짧게만 점유
    pub root: Mutex<Option<std::path::PathBuf>>,
}

// 빠른 작업(status, stage, commit): 뮤텍스 사용
#[tauri::command]
pub async fn git_status(state: State<'_, GitState>) -> Result<Vec<StatusRow>, String> {
    let guard = state.repo.lock().map_err(|e| e.to_string())?;
    let repo = guard.as_ref().ok_or("저장소가 열려 있지 않습니다")?;
    collect_status(repo).map_err(|e| e.to_string())
}

// 느린 작업(blame, log, push): 경로만 넘겨 blocking 스레드에서 새로 open
#[tauri::command]
pub async fn git_blame(
    state: State<'_, GitState>, path: String, from: usize, to: usize,
) -> Result<Vec<BlameLine>, String> {
    let root = state.root.lock().map_err(|e| e.to_string())?
        .clone().ok_or("저장소가 열려 있지 않습니다")?;

    tauri::async_runtime::spawn_blocking(move || {
        let repo = Repository::open(&root)?;   // open 은 저렴하다
        blame_range(&repo, &path, from, to)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e: git2::Error| e.to_string())
}
```

`Repository::open` 은 mmap 기반이라 반복 호출 비용이 낮다. 무거운 작업마다 새로 여는 편이 락 경합보다 훨씬 낫다.

### 레이어 배치 (FSD)

git2 호출은 Rust 쪽에만 존재하고, 프론트는 `entities/git/git.api.ts` 에서 `invoke()` 로만 접근한다. `Status` 비트플래그를 프론트로 그대로 넘기지 말고, 위 `StatusRow` 처럼 **Rust 에서 이미 VSCode 의미론으로 정규화한 DTO** 를 넘긴다. 프론트가 비트 조합을 해석하는 순간 매핑 로직이 두 군데로 갈라진다.

TanStack Query 키 예시:

```
QUERY_KEY.GIT = {
    STATUS: (root) => ['git', 'status', root],
    LOG:    (root, page) => ['git', 'log', root, page],
    BLAME:  (root, path) => ['git', 'blame', root, path],
    GUTTER: (root, path) => ['git', 'gutter', root, path],
}
```

### 파일 감시 연동

`notify` crate 로 워크디렉토리를 감시하되:

- `.git/` 내부 이벤트와 워크디렉토리 이벤트를 구분한다. `.git/index` 변경 → status 무효화, `.git/HEAD` / `.git/refs/**` 변경 → log·브랜치·ahead/behind 무효화.
- `.git/objects/**` 는 무시한다. fetch 한 번에 수천 개 이벤트가 쏟아진다.
- 200~300ms debounce 를 걸고 status 를 재조회한다. 저장 1회에 여러 이벤트가 온다.
- 변경된 파일 경로만 `StatusOptions::pathspec()` 으로 좁혀 부분 갱신하고, 전체 status 는 훨씬 긴 주기로만 돌린다.

### 구현 우선순위

1. discover + status + gutter diff (SCM 뷰의 뼈대)
2. stage/unstage/commit
3. discard (`force()` 주의)
4. log + revwalk 그래프
5. push/fetch + 인증 (feature 플래그 확인 필수)
6. blame (성능 격리가 끝난 뒤에)
7. stash

---

## 함정·주의

### 빌드·설정

- **`default-features = false` 를 쓰든 안 쓰든 0.21 에서는 결과가 같다.** `default = []` 이므로 `features = ["ssh","https"]` 를 명시하지 않으면 push 가 런타임에 "unsupported URL protocol" 로 실패한다. 컴파일은 통과하기 때문에 발견이 늦다.
- Tauri 배포에는 `vendored-libgit2` 를 켠다. 사용자 머신의 libgit2 버전에 의존하면 배포판이 깨진다.
- Linux 크로스 빌드 시 `vendored-openssl` 이 없으면 glibc/OpenSSL 버전 문제가 난다.

### API 사용

- **`Diff::foreach` 의 콜백은 `&mut dyn FnMut` 이다.** 클로저를 값으로 넘기면 타입 에러가 난다. `&mut |a, b| { ... }` 로 감싸야 한다.
- **콜백에서 `false` 반환 = 취소**이고, 이는 `Err` 로 돌아온다. 정상 순회에서 실수로 `false` 를 반환하면 원인 불명의 에러가 된다.
- **`CheckoutBuilder::force()` 누락** — discard 가 조용히 무동작한다.
- **`Index::write()` 누락** — stage 가 메모리에서만 일어나고 디스크에 반영되지 않는다.
- **stash 는 `&mut Repository`** 를 요구한다. 나머지 API 와 가변성이 달라 뒤늦게 리팩토링을 유발한다.
- **0.21 의 `Result` 반환 문자열 접근자**: `path()`, `message()`, `url()`, `name()` 등이 전부 `Result` 다. `.unwrap()` 대신 비-UTF8 경로를 건너뛰거나 `*_bytes()` 변형을 쓴다. 한글 파일명은 UTF-8 이라 괜찮지만, 윈도우의 레거시 인코딩 경로에서 터진다.
- **`BlameHunk::final_signature()` 는 `Option`** — 0.21 이전 코드에서 옮겨왔다면 반드시 확인한다.
- **`statuses()` 에 pathspec 을 주면 rename 검출 결과가 부정확해진다** (docs.rs 원문 경고). 이름 변경을 정확히 보려면 pathspec 없이 전체 status 를 돌려야 한다. 부분 갱신 최적화와 정면으로 충돌하는 지점이므로, rename 정확도가 필요한 화면에서는 전체 status 를 쓴다.
- **`Status` 에 Copied 플래그가 없다.** C 배지는 `find_similar(copies(true))` 를 적용한 diff 로만 만들 수 있다.
- **`diff_tree_to_workdir` 대신 `diff_tree_to_workdir_with_index`** 를 쓴다. 전자는 index 를 무시해 staged 파일에서 잘못된 gutter 를 그린다.

### libgit2 자체의 한계 (VSCode 대비 기능 격차)

- **git hook 을 실행하지 않는다.** `pre-commit`, `commit-msg`, `pre-push` 가 전부 무시된다. husky/lefthook 을 쓰는 프로젝트에서 TAIDE 로 커밋하면 훅이 건너뛰어진다. 사용자에게 명시하거나, 커밋 전 훅 스크립트를 직접 실행하는 레이어가 필요하다.
- **Git LFS 를 지원하지 않는다.** LFS 포인터 파일이 그대로 체크아웃된다.
- **커밋 서명을 직접 하지 못한다.** `commit_create_buffer` + 외부 프로세스 + `commit_signed` 조합이 필요하다.
- **`git rebase -i`, `worktree` 관리, sparse-checkout** 등 고급 기능은 부분적이거나 없다. 필요하면 `git` CLI 를 서브프로세스로 호출하는 폴백을 준비한다. VSCode 자체가 git CLI 를 호출한다는 점을 상기할 것.
- **credential helper 는 자동으로 동작하지 않는다.** 콜백에서 `Cred::credential_helper` 를 직접 호출해야 macOS Keychain / Windows Credential Manager 가 쓰인다.
- **SSH 는 libssh2 를 쓴다.** OpenSSH agent 와 대체로 호환되지만, 일부 최신 키 타입·설정에서 `git` CLI 는 되는데 libgit2 는 실패하는 사례가 알려져 있다. 이 경우 `Cred::ssh_key` 로 키 경로를 직접 지정하는 폴백 UI 가 필요하다. (구체적 실패 키 타입 목록은 이번 조사에서 확정하지 못했다 — `미확인`)

### 성능

- **blame 은 캐시하지 않으면 IDE 를 멈춘다.** 반드시 `spawn_blocking` + `(path, HEAD oid)` 캐시 + debounce.
- **`recurse_untracked_dirs(true)` 는 대형 리포에서 치명적이다.** `node_modules` 가 gitignore 되어 있어도, ignore 되지 않은 대형 디렉토리가 있으면 수십만 경로를 순회한다. 기본은 `false`.
- **`include_ignored(true)` 를 켜지 말 것.** 무시된 파일 전체를 열거한다.

---

## 대형 리포 성능 전략

### status

1. `StatusOptions::update_index(true)` — stat 캐시를 갱신해 다음 호출을 가속한다. 단 index 쓰기가 발생하므로 읽기 전용 상황에서는 끈다.
2. `no_refresh(true)` 는 index 재로드를 건너뛰어 빠르지만, 외부 git CLI 가 index 를 바꿨을 때 결과가 낡는다. 파일 감시로 `.git/index` 변경을 잡고 있을 때만 켠다.
3. `recurse_untracked_dirs(false)` — untracked 디렉토리를 디렉토리 단위로 접는다. VSCode 도 동일하게 동작한다.
4. `exclude_submodules(true)` — 서브모듈 status 는 별도 요청으로 분리한다. 서브모듈 재귀는 status 비용의 대부분을 차지할 수 있다.
5. **부분 갱신**: 파일 감시가 알려준 경로만 `pathspec()` 으로 좁혀 status 를 돌리고 캐시를 병합한다. 다만 위 "함정" 의 rename 부정확 경고를 감안해, rename 표시가 필요한 전체 뷰는 주기적 전체 status 로 보정한다.
6. Rust 쪽에 `HashMap<PathBuf, StatusRow>` 캐시를 두고, 프론트로는 변경분만 이벤트로 emit 한다. 매번 전체 배열을 직렬화하면 5만 파일 리포에서 IPC 가 병목이 된다.

### blame

1. 뷰포트 + 여유분만 `min_line`/`max_line` 으로 요청한다.
2. `first_parent(true)` 로 머지 커밋 내부 탐색을 생략한다. 체감 효과가 가장 크다.
3. `(path, HEAD oid)` 를 키로 결과를 캐시한다. HEAD 가 바뀌지 않는 한 파일이 커밋되지 않았으면 유효하다.
4. 편집 중에는 `blame_file` 재호출 대신 `Blame::blame_buffer(현재 버퍼 바이트)` 로 라인 매핑만 갱신한다.
5. 커서 이동 debounce 300ms 이상. 인라인 blame 은 커서가 멈춘 뒤에만 표시한다.
6. `track_copies_*` 는 전부 끈다(기본 off). 켜면 수 배 느려진다.

### log / 그래프

- `Revwalk` 는 지연 평가되므로 `.skip(n).take(m)` 페이지네이션이 저렴하다. 전체 커밋을 미리 수집하지 말 것.
- 그래프 레인 계산은 부모 OID 목록만 있으면 되므로 프론트에서 수행하고, Rust 는 `(id, parents[])` 만 넘긴다.
- `Sort::TOPOLOGICAL | Sort::TIME` 을 쓴다. `TIME` 단독은 시계 왜곡 커밋에서 그래프가 꼬인다.

### diff / gutter

- `context_lines(0)` — gutter 에는 컨텍스트 라인이 불필요하고, 파싱량이 줄어든다.
- 열려 있는 파일만 `pathspec()` 으로 좁혀 diff 한다. 저장소 전체 diff 를 매번 돌리지 않는다.
- `max_size()` 로 대용량 파일을 binary 처리해 diff 를 건너뛴다.

---

## gitoxide(gix) 대안 평가

버전: **gix 0.86.0** (2026-07-23).

`crate-status.md` (main 브랜치, 2026-08-06 확인) 기준 구현 현황:

| 기능 | 상태 | 원문 |
|---|---|---|
| **push** | **미구현** | `* [ ] push` / `* [ ] report-status, sideband, delete-refs, push-options and atomic pushes`. 상위 로드맵에도 `* [ ] push and self-contained clone/fetch over file:// and ssh://` 로 미완료 표기 |
| status | 구현됨 | `* [x] differences between index and worktree`, rename tracking·untracked 포함. 단 `* [ ] fs-monitor 지원`, `* [ ] sparse-index 가속` 미구현 |
| blame | 부분 구현 | `* [x] commit-annotations for a single file` 이나 `- [ ] progress`, `- [ ] interruptibility`, `- [ ] streaming`, **`- [ ] support for worktree changes`** 미구현 |
| diff | 구현됨 | tree/blob diff, rename tracking 포함 |
| worktree checkout | 구현됨 | `* [x] checkout an index of files ... just as fast as git` |
| stash | 구현됨 | 생성·list·show·drop·branch |
| client-side hooks | 미구현 | `* [ ] client-side hooks for commit, checkout, rebase, merge, am and push` |

### 판정: 2026-08 기준 TAIDE 는 git2 를 쓴다

근거는 세 가지다.

1. **push 가 없다.** VSCode 수준을 목표로 하는 IDE 에서 push 는 타협 불가 기능이다. 웹 검색 요약에는 "push 지원" 이라는 서술이 돌아다니지만, 공식 `crate-status.md` 원문의 체크박스는 명확히 미완료(`[ ]`)다. 요약문이 아니라 원문을 근거로 삼아야 한다.
2. **blame 이 워크트리 변경을 지원하지 않는다.** 인라인 blame 은 편집 중인 버퍼에서 동작해야 하는데, gix 는 `HEAD` 위에 가상 커밋을 올리는 기능이 미구현이다. git2 는 `blame_buffer` 로 이미 해결된 영역이다.
3. **API 가 아직 breaking 변경을 자주 낸다.** 0.78 → 0.86 이 7개월 사이에 나왔고 중간에 yank(0.82.0)도 있었다. 안정성이 필요한 제품 코드에는 부담이다.

gix 의 장점(순수 Rust, C 의존성·빌드 복잡도 없음, status/diff 성능 우수, 병렬성 설계가 현대적)은 실재하지만, push 부재 하나로 결론이 난다.

### 현실적 절충안

- **주 엔진은 git2** 로 전부 구현한다.
- git2 로 불가능한 영역(hook 실행, LFS, interactive rebase, 서명)은 **`git` CLI 서브프로세스 폴백**을 쓴다. gix 로 우회하려 하지 말 것 — gix 도 hook 을 실행하지 않는다.
- 데이터 접근 계층을 trait 로 한 겹 추상화해 두면(`trait GitBackend`), gix 가 push 를 완성했을 때 백엔드만 교체할 수 있다. 다만 지금 gix 를 병행 도입하는 것은 비용 대비 이득이 없다.
- gix 도입 재검토 시점: `crate-status.md` 의 `push` 항목이 `[x]` 가 되고 `gix-blame` 의 worktree 지원이 들어왔을 때.

---

## 참고 링크

- git2 crates.io: https://crates.io/crates/git2
- git2 0.21.0 API 문서: https://docs.rs/git2/0.21.0/git2/
- git2 CHANGELOG (0.21.0 파괴적 변경): https://github.com/rust-lang/git2-rs/blob/master/CHANGELOG.md
- git2 Cargo.toml (feature 정의 원문): https://github.com/rust-lang/git2-rs/blob/master/Cargo.toml
- git2-rs 저장소 및 예제: https://github.com/rust-lang/git2-rs / https://github.com/rust-lang/git2-rs/tree/master/examples
- libgit2-sys crates.io (번들 libgit2 버전 확인): https://crates.io/crates/libgit2-sys
- Repository API: https://docs.rs/git2/0.21.0/git2/struct.Repository.html
- Status 플래그: https://docs.rs/git2/0.21.0/git2/struct.Status.html
- StatusOptions: https://docs.rs/git2/0.21.0/git2/struct.StatusOptions.html
- Diff / DiffLine / DiffHunk: https://docs.rs/git2/0.21.0/git2/struct.Diff.html
- Blame / BlameOptions: https://docs.rs/git2/0.21.0/git2/struct.Blame.html
- Revwalk / Sort: https://docs.rs/git2/0.21.0/git2/struct.Revwalk.html
- RemoteCallbacks / Cred: https://docs.rs/git2/0.21.0/git2/struct.RemoteCallbacks.html / https://docs.rs/git2/0.21.0/git2/struct.Cred.html
- libgit2 공식: https://libgit2.org/
- gitoxide 저장소: https://github.com/GitoxideLabs/gitoxide
- gitoxide crate-status.md (push 미구현 근거): https://github.com/GitoxideLabs/gitoxide/blob/main/crate-status.md
- gix crates.io: https://crates.io/crates/gix
