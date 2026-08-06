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

## 6. 미저장 버퍼 미러 (dirty mirror)

- 에디터에서 dirty 상태의 파일 내용은 debounce(1~2초)로 `buffers/` 에 스냅샷된다.
- 복원 시 원본 파일 mtime 이 스냅샷 이후로 바뀌었으면 충돌로 간주하고 사용자에게
  (스냅샷 유지 / 디스크 내용 사용) 선택을 제공한다.
- 파일 저장 완료 시 해당 스냅샷은 즉시 삭제한다.
