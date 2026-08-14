# 데이터 모델 · 영속화 (세션 복원)

> 무엇이 어디에 저장되고, 재시작·리로드 시 무엇이 복원되는지의 정본.
> 상태 소유권 원칙은 ADR-0004, 저장 방식 결정은 ADR-0009.

## 1. 상태의 세 층위

| 층위 | 보관 위치 | 수명 | 예 |
|------|----------|------|-----|
| 영속 상태 | 디스크 (앱 데이터 디렉토리) | 앱 재시작 후에도 유지 | 프로젝트 목록, 탭·스플릿 레이아웃, 설정, 테마, 미저장 버퍼 |
| 런타임 상태 | Rust AppState (메모리) | 앱 실행 중 | pty 세션·스크롤백, git status 캐시, LSP 세션, watcher |
| 표시 상태 | React 로컬 | 컴포넌트 수명 | 호버, 드래그 중, 포커스, 입력 중 텍스트 |

- **view reload**: 영속 + 런타임 상태가 전부 살아 있으므로 전체 복원 (터미널 내용 포함).
- **앱 재시작**: 영속 상태만 복원. 터미널은 세션(탭·cwd·셸)만 복원되고 스크롤백 내용은 새로 시작
  (스크롤백 디스크 저장은 하지 않는다 — 용량·민감정보 노출 대비 이득 없음).

## 2. 디스크 레이아웃

Tauri path API 의 앱 데이터 디렉토리(`app_data_dir`, macOS: `~/Library/Application Support/TAIDE/`) 기준:

```
TAIDE/
├── settings.json            앱 전역 설정 (테마 선택, 폰트, 셸 오버라이드, 키바인딩 오버라이드)
├── session.json             전역 세션 — 열린 프로젝트 목록·순서, 활성 프로젝트, 윈도우 크기/위치
├── projects/
│   └── {projectId}/
│       ├── project.json     루트 경로, 이름, 부착 capability 설정 (프로젝트별 오버라이드 포함)
│       ├── layout.json      탭·스플릿 트리, 탭 순서, 활성 탭, 에디터별 viewState(커서·스크롤)
│       └── buffers/         미저장(dirty) 버퍼 미러 — 파일 경로 해시별 스냅샷
├── themes/                  사용자 커스텀 테마 (*.json) — theme-system.md 스키마
├── plugins/                 설치된 플러그인 (features/plugins.md)
└── logs/                    앱 로그 (회전)
```

## 3. 핵심 타입 (Rust 정본 — TS 는 자동 생성, ADR-0011)

```rust
struct SessionState {
    version: u32,                       // 스키마 버전 — 마이그레이션 기준
    projects: Vec<ProjectRef>,          // 순서 = 사이드바 표시 순서
    active_project: Option<ProjectId>,
}
// 윈도우 크기·위치·최대화는 tauri-plugin-window-state 가 담당 (ADR-0009)

struct ProjectRef { id: ProjectId, root: PathBuf, name: String }

struct ProjectLayout {
    version: u32,
    root: PaneNode,                     // 스플릿 트리
    focused_pane: PaneId,
}

enum PaneNode {
    Split { id: PaneId, dir: SplitDir, children: Vec<PaneNode>, sizes: Vec<f32> },
    Leaf  { id: PaneId, tabs: Vec<Tab>, active: Option<TabId> },
}

struct Tab {
    id: TabId,
    kind: TabKind,                      // File { path } | Terminal { session } | Settings | Diff { .. } | ...
    pinned: bool,
    preview: bool,
}
```

- `ProjectId` 는 **UUID** 다. 경로 해시가 아니므로 폴더가 이동해도 프로젝트 상태를 잇는다
  (재시작 시 root 부재이면 사용자에게 재연결/제거 선택 제공).
- `TabKind` 는 확장 가능한 enum — 새 탭 타입(remote-control 패널 등)이 여기 추가된다.
- 탭 복원 규칙: File 탭은 경로 존재 확인 후 복원(부재 시 탭 제거·알림), Terminal 탭은
  같은 cwd·셸로 새 pty 를 시작해 복원.

## 4. 쓰기 전략

- **쓰는 주체는 Rust 뿐이다** (view 는 localStorage 등 사용 금지 — ADR-0004).
- 변경 시 debounce(수 초) 저장 + 앱 종료 시 flush. 크래시 대비로 debounce 저장이 사실상의 스냅샷.
- **원자적 쓰기**: 임시 파일 작성 → fsync → rename. 부분 쓰기로 인한 세션 파괴 방지.
- 로드 실패(파손·버전 불일치) 시: 백업(`*.bak`) 시도 → 실패 시 해당 파일만 기본값으로 재생성하고
  사용자에게 알림. 전체 세션을 침묵 초기화하지 않는다.

## 5. 스키마 버전·마이그레이션

- 모든 영속 파일은 `version` 필드를 가진다. 로더는 `현재 버전`까지 순차 마이그레이션 함수를 적용한다.
- 필드 추가는 `#[serde(default)]` 로 무마이그레이션 흡수, 구조 변경만 버전을 올린다.

## 6. 미저장 버퍼 미러 (dirty mirror, Hot Exit — 기능 확장 3차)

> 계약: `docs/acknowledge/2026-08-14-hotexit-remote-password-contract.md` §3.1.
> 구현: `src-tauri/src/domain/file/service.rs`(스키마·판정) · `commands.rs`(IPC) ·
> `src-tauri/src/lib.rs`(`WindowEvent::CloseRequested` 인터셉트).

### 6.1 경로 미러 — `buffers/<pathHash>.json`

파일이 있는 탭의 dirty 내용은 debounce 로 경로 해시(FNV-1a 64bit hex) 파일명으로 스냅샷된다.

```rust
struct MirrorFile {
    path: String,
    content: String,
    saved_at_ms: f64,
    #[serde(default)]
    disk_modified_ms: Option<f64>,  // 미러 작성 시점의 원본 baseline mtime(view 가 file.modifiedMs 전달)
}
```

- `file_list_mirrors(projectId)` 는 저장된 `MirrorFile` 각각에 대해 **현재** 디스크 mtime 을 다시 읽어
  `disk_modified_ms`(baseline) 와 비교한 뒤 `MirrorEntry { path, content, savedAtMs, diskModifiedMs, conflict }`
  로 반환한다. `conflict = 현재_mtime > baseline`(baseline 이 `None` 이면 항상 `false`).
- 미러의 원본 파일이 디스크에서 사라졌으면 **그 항목은 목록에서 제외한다**(유령 복원 금지). 미러
  JSON 자체는 지우지 않고 `file_prune_mirrors` 의 GC 대상으로 남긴다.
- `file_clear_mirror(projectId, path)` 로 개별 삭제(저장 성공 시 `file_save` 가 자동 호출, "디스크
  내용 사용" 선택 시 view 가 호출), `file_prune_mirrors(projectId, keepPaths)` 로 `keepPaths` 에 없는
  미러(닫힌 탭·삭제된 파일 등)를 일괄 삭제(프로젝트 열 때 view 가 호출).

### 6.2 untitled 미러 — `buffers/untitled/<tabId>.json`

경로가 없는 untitled 탭은 별도 축으로 `TabId` 를 키로 미러된다(`ensure_within_root` 비대상).

```rust
struct UntitledMirrorFile {
    tab_id: String,
    content: String,
    saved_at_ms: f64,
}
```

- `file_mirror_untitled(projectId, tabId, content)` / `file_list_untitled_mirrors(projectId)` /
  `file_clear_untitled_mirror(projectId, tabId)`.
- Untitled 탭 자체도 이번 계약으로 `layout::is_volatile` 대상에서 제외되어 **레이아웃에 영속**된다
  (`layout/service.rs`). 재시작 시 탭이 먼저 복원되고, 그 탭이 활성화될 때 view 가 이 미러로 내용을
  lazy 복원한다.

### 6.3 종료 시 0-손실 플러시

`WindowEvent::CloseRequested` 를 가로채 `prevent_close()` 후 `HotExitFlushRequested` 이벤트를
emit 한다. view 는 모든 dirty 모델을 미러 IPC 로 밀어넣은 뒤 `file_flush_complete` 를 호출해야
실제 종료가 재개된다. `HOT_EXIT_FLUSH_TIMEOUT_MS`(2.5초) 안에 응답이 없으면 타임아웃 폴백으로
강제 종료한다(하드행 방지). `AppState` 의 `Idle → Pending → Ready` 3단 상태로 재진입(연타)·이중
emit·이중 `exit()` 를 가드한다.

### 6.4 판정 기준 요약

| 판정 | 근거 |
|------|------|
| 복원 가능 | 미러 존재 + 원본 파일이 현재 디스크에 존재 |
| conflict | `현재 디스크 mtime > 미러의 disk_modified_ms baseline` |
| 목록에서 제외 | 원본 파일이 디스크에서 사라짐(경로 미러) — 유령 복원 금지 |
| GC(prune) | 열린 탭(`keepPaths`) 에 없는 경로 미러 — 프로젝트 열 때 |
| 미러 삭제(clear) | 저장 성공(자동) · "디스크 내용 사용" 선택 · 탭 닫기(view 정책) |
