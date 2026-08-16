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
├── settings.json            앱 전역 설정 (테마 선택, 폰트, 셸 오버라이드, 키바인딩 오버라이드,
│                            LSP: organizeImportsOnSave·fixAllOnSave(기본 false)·
│                            editorCodeLensEnabled(기본 true) — Wave A, `features/lsp.md` §3.
│                            editorSemanticHighlighting(기본 true)·editorFormatOnType(기본 false)·
│                            editorFormatOnPaste(기본 false)·emmetEnabled(기본 true) — Wave F,
│                            `features/editor.md` §8/§9/§11.
│                            aiAutoTabEnabled(기본 false)·aiProvider·aiModel·aiOmlxBaseUrl — Wave G,
│                            `features/ai.md` §1, 필드 리네임은 §7)
├── prompts/                 사용자 AI 프롬프트 오버라이드(`<id>.json`, `auto-tab-default`·
│                            `inline-edit-default`·`commit-message-default`) — Wave G,
│                            `features/ai.md` §5
├── session.json             전역 세션 — 열린 프로젝트 목록·순서, 활성 프로젝트, 윈도우 크기/위치
├── projects/
│   └── {projectId}/
│       ├── project.json     루트 경로, 이름, 부착 capability 설정 (프로젝트별 오버라이드 포함)
│       ├── layout.json      탭·스플릿 트리, 탭 순서, 활성 탭, 에디터별 viewState(커서·스크롤)
│       └── buffers/         미저장(dirty) 버퍼 미러 — 파일 경로 해시별 스냅샷
├── themes/                  사용자 커스텀 테마 (*.json) — theme-system.md 스키마
├── snippets/                사용자 스니펫 (`<languageId>.json` + `*.code-snippets`, VS Code 호환
│                            포맷) — Wave F, `features/editor.md` §10
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
    version: u32,                       // Wave I: 2 (§8 마이그레이션 참조)
    root: PaneNode,                     // 스플릿 트리 — main 창
    focused_pane: PaneId,
    auxiliary_windows: Vec<AuxWindowLayout>,  // Wave I 신설 — §8
    shell_view: ShellViewState,               // Wave I 신설 — §8
}

enum PaneNode {
    Split { id: PaneId, dir: SplitDir, children: Vec<PaneNode>, sizes: Vec<f32> },
    Leaf  { id: PaneId, tabs: Vec<Tab>, active: Option<TabId> },
}

struct Tab {
    id: TabId,
    kind: TabKind,                      // File { path } | Terminal { session } | Settings | Diff { .. } | AppFile { target } | ...
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

## 7. 설정 필드 리네임 — `ai_auto_tab_provider`/`ai_auto_tab_model` → `ai_provider`/`ai_model` (Wave G)

> 계약: `docs/acknowledge/2026-08-16-wave-g-ai-contract.md` §3.1. 상세: `features/ai.md` §1.

- 자동완성 전용이던 `Settings.ai_auto_tab_provider`/`ai_auto_tab_model` 이 Inline Edit·AI 커밋
  메시지까지 공유하는 필드로 일반화되며 `ai_provider`/`ai_model` 로 리네임됐다(TS:
  `aiAutoTabProvider`/`aiAutoTabModel` → `aiProvider`/`aiModel`). `ai_auto_tab_enabled` 는
  auto-tab 전용 토글이라 이름 그대로 유지한다.
- **하위호환 메커니즘 — `#[serde(alias = ...)]` 가 아니다.** 계약은 필드 단위 serde alias 를
  지시했으나, 이 프로젝트가 고정한 specta(`=2.0.0-rc.25`) 에서 필드 alias 를 붙이면
  `Settings`/`SettingsPatch` 의 생성된 TS 타입이 `Settings_Serialize`/`Settings_Deserialize` 유니온
  으로 쪼개지고, `aiProvider`/`aiModel` 과 무관한 필드(예: `editorCodeLensEnabled`)까지
  `Pick<Settings, ...>` 를 쓰는 모든 소비처(`use-lsp-session.ts` 등)에서 타입이 깨지는 것을 실물
  검증으로 확인했다. 대신 **raw `serde_json::Value` 사전 마이그레이션**
  (`domain::settings::service::migrate_legacy_ai_provider_keys`)으로 구현했다 — 역직렬화 직전에
  JSON 객체의 `aiAutoTabProvider`/`aiAutoTabModel` 키를 `aiProvider`/`aiModel` 로 옮긴다(신 키가
  이미 있으면 구 키는 건드리지 않고 그대로 둔다 — `Settings`/`SettingsPatch` 에 없는 필드라 역직렬화
  시 조용히 무시된다). 그 결과를 평범한 플랫 `Settings`/`SettingsPatch` 로 파싱한다.
  결과적으로 하위호환 기능은 계약과 동일하게 보장되지만, `Settings`/`SettingsPatch` 타입 자체는
  구 필드명을 전혀 모르는 순수 플랫 타입으로 남는다.
- **적용 지점은 디스크/동기화 페이로드 로드 두 곳뿐이다** — `settings::service::load_settings`
  (→ `read_settings_file`) 와 `sync::service::parse_synced_payload`(gist 다운로드). **살아있는
  IPC 경계(`settings_update` 로 오는 `SettingsPatch`)에는 적용되지 않는다** — `bindings.ts` 가 이미
  `aiProvider`/`aiModel` 만 노출하므로 정상 프론트 소비처가 구 필드명을 보낼 경로 자체가 없다.
- 검증: `migrate_legacy_ai_provider_keys` 자체의 역직렬화 테스트(`settings/service.rs`) +
  구 필드명으로 저장된 `settings.json` 을 읽는 통합 테스트 + gist 페이로드 쪽 동일 테스트
  (`sync/service.rs`). 필드 단위 alias 대비 이 방식의 트레이드오프: 마이그레이션 지점이 "역직렬화가
  일어나는 모든 곳"이 아니라 "JSON 을 처음 읽어들이는 두 진입점"으로 좁혀지므로, 향후 구 페이로드를
  읽는 새 진입점이 추가되면 이 사전 마이그레이션 호출을 함께 추가해야 한다(자동으로 따라오지 않음).

## 8. 레이아웃 스키마 v2 — 멀티 윈도우·Zen·AppFile (Wave I)

> 계약: `docs/acknowledge/2026-08-16-wave-i-shell-workspace-contract.md` §3.1/§3.2/§3.3.
> 상세: `layout-shell.md` §7, `window-chrome.md` §5, `tabs.md` §3.1/§4.4.

- **`LAYOUT_SCHEMA_VERSION` 1 → 2.** 순수 가산(additive) 변경 — v1 이 갖던 필드는 그대로이고
  `auxiliary_windows`/`shell_view` 두 필드만 새로 붙는다. 둘 다 `#[serde(default)]` 라 v1 JSON 은
  구조 변경 없이 현재 `ProjectLayout` 형태로 그대로 역직렬화된다.
- **마이그레이션은 이번에 처음 생겼다.** Wave I 이전에는 `load_layout` 이 `version` 이 현재와 다르면
  무조건 `default_layout()` 로 폴백해 **기존 탭을 전량 유실**시켰다(§2 확정 사실 5). 이제는
  `version <= LAYOUT_SCHEMA_VERSION` 이면 `migrate_layout` 이 버전 필드만 올려 스탬프하고 탭은 그대로
  보존한다. `default_layout()` 폴백은 ① JSON 자체가 파싱 불가(파손)이거나 ② 이 빌드가 모르는 **더
  높은** 버전일 때만 남는다(다운그레이드 마이그레이션은 안전하게 만들 수 없으므로). 회귀 테스트가
  실제 v1 형태 JSON(신필드 제거 후 저장)의 왕복을 고정한다(`layout/service.rs`
  `v1_레이아웃_파일은_기존_탭을_보존한채_v2로_마이그레이션된다`).
- **`AuxWindowLayout`** — 보조 편집 창(`editor-<n>`) 하나당 하나, main 창과 **구조적으로 동일한**
  자기만의 스플릿 트리를 가진다.

  ```rust
  struct AuxWindowLayout {
      slot: u32,             // 창의 논리적 식별자 — OS 창 라벨(editor-<n>)과 1:1 이 아니다.
                              // 라벨은 Rust 가 재사용 가능한 최소 미사용 번호로 발급하고, slot 은
                              // 레이아웃이 "이 트리는 어느 보조 창의 것인가"를 기억하는 값이다.
      root: PaneNode,
      focused_pane: PaneId,
  }
  ```

  - 보조 창을 닫으면(플레인 ✕/⌘W 로 마지막 탭까지 다 닫아 빈 트리가 된 경우는 예외 — 아래 참조)
    Rust 가 그 슬롯의 탭 전체를 main 창 포커스 pane 말미로 옮기고 슬롯 자체를 제거한다
    (`window::service::plan_return_of_auxiliary_window_tabs`) — VS Code 는 보조 창을 닫으면 탭이
    그냥 사라지지만, TAIDE 는 "0-손실" 철학상 항상 main 으로 복귀시킨다.
  - `layout_move_tab_to_window` 로 빈 슬롯이 만들어지면 그 자리에서 즉시 슬롯이 제거된다
    (`cleanup_emptied_auxiliary_windows`). 사용자가 보조 창 안에서 그냥 마지막 탭을 닫아 트리가
    비는 경로는 이 정리를 타지 않는다 — 프론트(`auxiliary-window-shell.tsx`)가 자기 트리가 빈 것을
    감지해 OS 창을 스스로 닫는 것으로 대칭을 완성한다(재시작 시 빈 슬롯은 저장 단계에서 제외되어
    복원되지 않는다).
  - 재시작 시 열려 있던(활성 프로젝트 여부와 무관하게, 세션에 남아있는 모든 프로젝트의) 보조 창을
    Rust 가 전부 재생성한다(`lib.rs::restore_auxiliary_windows`).
- **`ShellViewState`** — main 창 전용 표시 상태(보조 창은 사이드바·상태바가 원래 없는 에디터 전용
  크롬이라 이 값의 영향을 받지 않는다).

  ```rust
  struct ShellViewState {
      zen: bool,               // 기본 false
      sidebar_collapsed: bool, // 기본 false — 폭 자체는 여전히 프론트 로컬 debounce(ADR-0004 예외)
  }
  ```

  - Zen 모드는 사이드바·탭바·(설정에 따라) 상태바를 숨긴다. `Settings.zen_fullscreen`(기본 false)이
    켜져 있으면 Zen 진입 시 OS 창도 함께 전체화면 전환한다(`window_set_fullscreen`).
    `Settings.zen_hide_status_bar`(기본 true)가 상태바까지 숨길지를 결정한다.
- **`TabKind::AppFile`** — 프로젝트 폴더 밖에 있는 앱 소유 파일(`settings.json`, 프롬프트 템플릿
  오버라이드)을 일반 파일 탭과 같은 UX(탭·dirty 표시·닫기 확인)로 열되, 그 파일들이 실제로 어느
  경로에 있는지는 프론트/레이아웃에 전혀 노출하지 않는다.

  ```rust
  enum AppFileTarget {
      Settings,
      Prompt { id: PromptTemplateId },  // 'auto-tab-default' | 'inline-edit-default' | 'commit-message-default'
  }
  ```

  - 읽기/쓰기는 `app_file_read`/`app_file_write` 전용 커맨드로만 하며, 둘 다 `root_guard` 의 프로젝트
    루트 검증 대상이 아니다(경로는 Rust `AppPaths` 내부에서만 유도된다).
  - **hot-exit 미러는 1차 제외**(계약 명시) — 저장하지 않은 `AppFile` 탭 내용은 창이 비정상 종료되면
    복구되지 않는다. dirty 표시와 "닫기 시 확인" 다이얼로그는 기존 파일 탭과 동일한 흐름을 그대로
    재사용한다.
  - 동일 `target` 을 여러 pane 에 중복해서 여는 정책은 일반 파일 탭과 동일(같은 leaf 안에서는 재사용,
    다른 leaf 로는 명시적 분할 이동으로만 중복 — `tabs.md` §3).
- **`Settings` 신규 필드 2종**: `zen_fullscreen: bool`(기본 false) · `zen_hide_status_bar: bool`
  (기본 true). 둘 다 `SettingsPatch`/`emptySettingsPatch`/sync 페이로드에 포함된다.
