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
│                            `features/ai.md` §1, 필드 리네임은 §7. aiOmlxBaseUrl(sync 업로드·원격
│                            dispatch 대상에서 제외 — d-41,
│                            `acknowledge/2026-08-25-d41-omlx-baseurl-strip-contract.md`).
│                            remoteAllowedHosts(기본 []) — Wave B,
│                            `acknowledge/2026-08-15-wave-b-hardening-contract.md`(원격 세션 허용
│                            호스트명 화이트리스트, sync 업로드·원격 dispatch 대상에서 제외).
│                            항목에 `*.` 접두 와일드카드 허용 — 손 QA 1차 수정(2026-08-18),
│                            `acknowledge/2026-08-18-hand-qa-fix-contract.md` §2.3(선두 1레이블만
│                            매칭, 베이스 도메인 자신은 불포함 — RFC 6125).
│                            themeId 를 `settings_set_theme` 로 바꾸면 followSystemTheme 이 자동으로
│                            false 로 꺼진다 — 손 QA 1차 수정(2026-08-18), 이전엔 이 플래그가 켜진
│                            채로 테마를 골라도 `theme_get_current` 가 계속 OS 테마로 재해석해 선택이
│                            조용히 무시됐다.
│                            recentSearches(기본 [], 상한 20·중복 제거는 프론트 담당·서버는 순수
│                            passthrough) — Wave D,
│                            `acknowledge/2026-08-15-wave-d-search-nav-contract.md` §3.5.
│                            zenFullscreen(기본 false)·zenHideStatusBar(기본 true) — Wave I, §8)
├── prompts/                 사용자 AI 프롬프트 오버라이드(`<id>.json`, `auto-tab-default`·
│                            `inline-edit-default`·`commit-message-default`) — Wave G,
│                            `features/ai.md` §5
├── session.json             전역 세션 — 열린 프로젝트 목록·순서, 활성 프로젝트, 윈도우 크기/위치
├── projects/
│   └── {projectId}/
│       ├── project.json     루트 경로, 이름, 부착 capability 설정 (프로젝트별 오버라이드 포함),
│                            lastOpenedAt(밀리초 epoch, `#[serde(default)]` — d-27, §18)
│       ├── layout.json      탭·스플릿 트리, 탭 순서, 활성 탭, 에디터별 viewState(커서·스크롤)
│       └── buffers/         미저장(dirty) 버퍼 미러 — 파일 경로 해시별 스냅샷
├── themes/                  사용자 커스텀 테마 (*.json) — theme-system.md 스키마
├── snippets/                사용자 스니펫 (`<languageId>.json` + `*.code-snippets`, VS Code 호환
│                            포맷) — Wave F, `features/editor.md` §10
├── plugins/                 설치된 플러그인 (features/plugins.md)
├── locales/                 사용자 언어팩 (*.json — domain/locale·sync 가 읽기/쓰기, paths.rs locales_dir)
└── lsp/                     다운로드된 LSP 서버 바이너리 (`lsp/<serverId>/<version>/` — paths.rs lsp_dir 계열)
```

> 앱 로그는 이 트리(app_data_dir) 밖의 `~/Library/Logs/{identifier}/TAIDE.log` 에 쓰인다 —
> tauri-plugin-log 기본 타겟(Stdout + LogDir)이며 `app_log_dir` 은 macOS 에서 `~/Library/Logs/{identifier}` 다.
> `identifier` 는 dev(`bun run tauri dev`)와 설치본(prod)이 서로 다르다(계약 d-49) — dev 는
> `dev.taide.app.dev`, 설치본은 `dev.taide.app`. `app_data_dir` 를 포함한 이 절 전체(위 트리)도
> 마찬가지로 identifier 별로 갈라진다.

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
    revision: u32,                      // 낙관적 동시성 카운터 — LayoutChanged{projectId, revision} 로 브로드캐스트
    closed_tabs: Vec<ClosedTab>,        // 최근 닫은 탭 스택(≤ CLOSED_TAB_STACK_LIMIT=20) — 재-오픈 지원
    auxiliary_windows: Vec<AuxWindowLayout>,  // Wave I 신설 — §8
    shell_view: ShellViewState,               // Wave I 신설 — §8
}

enum PaneNode {
    Split { id: PaneId, dir: SplitDir, children: Vec<PaneNode>, sizes: Vec<f32> },
    Leaf  { id: PaneId, tabs: Vec<Tab>, active: Option<TabId> },
}

struct Tab {
    id: TabId,
    kind: TabKind,                      // File{path} | Terminal{sessionId, cwd?} | Settings
                                         // | Diff{path,staged,compareWith?,rev?,parentRev?,beforePath?}
                                         //   (rev? 이하 3필드는 손 QA 1차 수정(2026-08-18) 신설 —
                                         //   커밋 diff 확장, 전부 #[serde(default)] 로 하위 호환)
                                         // | ClaudeDiff{requestId,path} | Welcome | Untitled{index} | SearchEditor{query}(Wave D)
                                         // | AppFile{target}(Wave I) — 9종, §8
    title: String,
    pinned: bool,
    preview: bool,
    dirty: bool,
    view_state: Option<String>,         // 에디터별 viewState(커서·스크롤) — monaco 가 직렬화한 불투명 JSON 문자열
}

struct ClosedTab { tab: Tab, pane_id: PaneId, index: u32 }
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
    Rust 가 전부 재생성한다(`domain::window::commands::restore_auxiliary_windows`).
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

## 9. Wave A~I 신규 도메인 타입 — 영속 스키마에 포함되는 것

> Rust 정본은 각 도메인의 `types.rs`, TS 는 `src/shared/api/bindings.ts` 자동 생성(ADR-0011).
> 아래 세 타입은 §1의 "영속 상태"에 실제로 쓰인다 — 나머지 신규 타입(Task·git·AI 요청/응답 등)은
> 디스크에 쓰이지 않는 순수 IPC 페이로드라 §10 에 별도 정리한다.

### 9.1 스니펫 파일 — `snippets/<languageId>.json` · `*.code-snippets` (Wave F)

```rust
enum SnippetStringOrList { Single(String), Multiple(Vec<String>) }  // #[serde(untagged)] → TS: string | string[]

struct SnippetEntry {
    prefix: SnippetStringOrList,
    body: SnippetStringOrList,
    description: Option<SnippetStringOrList>,  // 없으면 직렬화 생략
    scope: Option<String>,                     // `.code-snippets` 전용(languageId 콤마 목록), 프론트가 해석
}

struct SnippetFile { file_name: String, snippets: BTreeMap<String, SnippetEntry> }
```

- VS Code 스니펫 스키마 호환. `isFileTemplate`/`include`/`exclude` 는 의도적으로 모델링하지 않는다
  (알 수 없는 JSON 필드는 serde 기본 동작으로 조용히 버려짐 — `2026-08-15-wave-f-editor-presentation-contract.md`
  §3.3 "1차 무시" 결정).
- 파싱 실패 파일은 목록에서 스킵 + 로그만 남긴다(테마 목록 로딩과 동일한 견고성 원칙). Rust 는
  캐시하지 않고 프론트 TanStack Query 가 `snippet_save`/`snippet_delete` 성공 시 invalidate 로 재조회한다.
- **실측 확인**: `bindings.ts` 는 `SnippetEntry`/`SnippetFile` 을 각각 `SnippetEntry_Serialize` |
  `SnippetEntry_Deserialize`(`SnippetFile` 도 동형)로 쪼갠다 — §7 에서 `Settings`/`SettingsPatch` 가
  피해간 것과 같은 specta 유니온 분할 현상이 여기서는 실제로 발생해 있다(중첩된
  `SnippetStringOrList` 의 `#[serde(untagged)]` 이 원인으로 보인다). 두 절반의 필드 구성은 현재
  동일해 기능상 문제는 없지만, `ipc-contract.md` 의 snippet 절(`SnippetEntry = { prefix, body,
  description?, scope? }`)은 이 분할을 반영하지 않은 단순화된 표기다.

### 9.2 AI 프롬프트 오버라이드 — `prompts/<id>.json` (Wave G)

```rust
struct AiPromptTemplate            { version: u32, fim: AiFimPromptTemplate, chat: AiChatPromptTemplate }  // 기존(auto-tab)
struct AiInlineEditPromptTemplate  { version: u32, system: String, user: String }  // Wave G 신설
struct AiCommitMessagePromptTemplate { version: u32, system: String, user: String }  // Wave G 신설
```

- `id` 는 `app::types::PromptTemplateId`(`auto-tab-default` | `inline-edit-default` |
  `commit-message-default`, kebab-case 직렬화)와 1:1. `ai::prompt::load_*` 가 `prompts_dir()/{id}.json`
  을 읽어 실패(부재·파싱 실패) 시 내장 기본값(`bundled_*_template`)으로 폴백한다.
- Inline Edit·AI 커밋 메시지가 별도 템플릿 파일로 분리된 이유: 기존 `AiPromptTemplate`(auto-tab, FIM
  구조)과 셰이프가 달라 재사용하면 사용자 오버라이드가 서로 충돌한다 —
  `2026-08-16-wave-g-ai-contract.md` §2-3.
- `TabKind::AppFile { target: AppFileTarget::Prompt { id } }` 로 이 파일들을 일반 파일 탭처럼 여닫을
  수 있다(§8). hot-exit 미러 대상에서는 제외.

### 9.3 `SearchQuery` — `TabKind::SearchEditor` 에 임베드되어 `layout.json` 에 영속 (Wave D)

```rust
struct SearchQuery {
    text: String,
    case_sensitive: bool,           // 기본 false
    whole_word: bool,                // 기본 false
    regex: bool,                     // 기본 false
    include_glob: Option<String>,
    exclude_glob: Option<String>,
    context_lines: u32,              // 기본 0 — Search Editor 만 opt-in, 기존 퀵서치 패널은 컨텍스트 없음 그대로
    respect_gitignore: bool,         // 기본 true — VS Code `search.useIgnoreFiles` 기본값과 동일
}
```

- Search Editor 탭은 **쿼리만** 영속하고 결과(`SearchMatch[]`)는 영속하지 않는다 — 재시작 시
  탭 복원 직후 `search_run` 을 다시 실행한다(대량 결과를 매 세션 저장 파일에 태우지 않기 위함,
  `2026-08-15-wave-d-search-nav-contract.md` §3.4). `SearchMatch` 자체는 §10.3 참조.
- `context_lines`/`respect_gitignore` 모두 `#[serde(default = ...)]` 라 이 필드가 없던 이전 호출부
  (퀵서치 패널)도 그대로 역직렬화된다.

## 10. Wave A~I 신규 도메인 타입 — 비영속 IPC 타입 (참고 인벤토리)

> 아래는 디스크에 쓰이지 않는 요청/응답 DTO다. 커맨드 시그니처·이벤트의 정본은
> `docs/ipc-contract.md`(도메인별 §3) · 각 `docs/features/*.md` 이며, 여기서는 캠페인 문서 정합화
> 범위에 맞춰 타입 셰이프만 실측 대조해 둔다.

### 10.1 태스크 러너 — `Task`/`TaskSource` (Wave E)

```rust
enum TaskSource { Npm, Make, Cargo }               // TS: "npm" | "make" | "cargo"
struct Task { label: String, command: String, source: TaskSource, cwd: String }
```

- `task::commands::detect_tasks(projectId)` 가 매 호출 `package.json`/`Makefile`/`Cargo.toml` 을 훑어
  즉석에서 만들어내는 순수 파생값이다. 저장·캐시하지 않는다 — 재시작 후에도 다시 감지된다.

### 10.2 git 신규 타입 (Wave C 3-way·hunk·revert·tag·파일히스토리, Wave G/H AI 커밋 메시지)

```rust
struct ConflictSides { base: Option<String>, ours: Option<String>, theirs: Option<String>, workdir: String }  // Wave C
struct CommitFile    { path: String, abs_path: String, orig_path: Option<String>, orig_abs_path: Option<String>, kind: GitChangeKind }  // Wave C, abs_path/orig_abs_path 는 T0 감사 #18(§13)
struct TagInfo        { name: String, target: String, message: Option<String>, annotated: bool }              // Wave C
struct TagCreateOptions { message: Option<String>, annotated: bool }                                            // Wave C
struct RevertOutcome  { conflicted: bool, conflicted_paths: Vec<String>, conflicted_abs_paths: Vec<String> }    // Wave C, conflicted_abs_paths 는 T0 감사 어드버서리얼 검증(§13)
struct StagedDiffText { diff_text: String, truncated: bool, skipped_files: Vec<String>, used_fallback: bool }  // Wave G, used_fallback 은 Wave H
```

- `ConflictSides` 는 index stage 1/2/3(ancestor/ours/theirs) blob + workdir 현재 내용을 함께 준다
  (`git_conflict_sides`). add/add 충돌처럼 base 가 없는 경우는 `None`.
- `RevertOutcome.conflicted_paths` 는 revert 가 충돌로 끝났을 때 다음 상태 새로고침을 기다리지 않고
  바로 첫 충돌 파일로 안내하기 위한 필드(repo-relative, `git2` 인덱스 경로 그대로). `conflicted_abs_paths`
  는 그 절대경로 동봉본(workdir 루트 join) — `CommitFile`/`StatusRow` 의 `abs_path` 와 동일한 이유로,
  파일 도메인 진입점(`file_open` 경유 `onOpenFile`)에 상대경로를 그대로 넘기면 안 되기 때문이다.
- `StagedDiffText.used_fallback` 은 staged 델타가 0건이라 HEAD↔워킹트리(untracked 포함) 전체 diff 로
  대체됐다는 표시(Wave H, `2026-08-16-wave-h-keymap-contract.md` §3.4). AI 커밋 메시지 생성
  (`ai_commit_message`, §10.4) 의 유일한 소비처다.

### 10.3 `SearchMatch` (Wave D) — §9.3 의 영속 `SearchQuery` 와 대비되는 비영속 결과

```rust
struct SearchMatch {
    path: String, line: u32, column: u32, preview: String, match_start: u32, match_end: u32,
    before: Vec<String>,  // SearchQuery.context_lines 만큼, 기본 0 이면 항상 빈 배열
    after: Vec<String>,
}
```

- `column`(1-based)·`match_start`/`match_end`(0-based, `preview` 안 오프셋)은 T0 감사 #23(§13)
  이후 UTF-16 코드유닛 단위다(이전엔 Rust UTF-8 바이트 오프셋을 그대로 흘려보내 한글 등 비 ASCII
  텍스트 뒤 매치의 좌표가 실제보다 커졌다).

### 10.4 AI 요청/응답 (Wave G — Inline Edit·AI 커밋 메시지, `AiInlineComplete*`(auto-tab)는 캠페인 이전부터 존재해 제외)

```rust
struct AiInlineEditRequest  { request_id, owner: String, provider: Option<AiProviderId>, model: Option<String>,
                               selection, instruction, language, file_path, prefix, suffix: String }
struct AiCommitMessageRequest  { request_id, owner: String, provider: Option<AiProviderId>, model: Option<String>,
                                  diff_text, recent_commits: String }
struct AiTextResponse { request_id: String, text: Option<String> }  // d-37 — 세 커맨드(auto-tab 포함) 공용
```

- `provider`/`model` 이 없으면 `Settings.ai_provider`/`ai_model`(§7)을 기본값으로 쓴다.
- 프롬프트 렌더링에는 §9.2 의 `AiInlineEditPromptTemplate`/`AiCommitMessagePromptTemplate` 을 쓴다.

### 10.5 `AuxiliaryWindowInfo` (Wave I) — `open_auxiliary_window` 의 반환값, §8 의 영속 `AuxWindowLayout` 과는 다른 타입

> X-A 배치(2026-08-19)에서 이 타입을 직접 노출하던 `window_open_auxiliary` 커맨드가 중복으로
> 제거됐다(`docs/ipc-contract.md` §"Wave I 계약 확정 추가"). 코어 함수 `open_auxiliary_window` 는
> `layout_move_tab_to_window` 의 `newAuxiliary` 경로와 부팅 시 보조 창 복원에서 내부적으로 계속
> 쓰이지만, 그 반환값을 커맨드 응답으로 그대로 넘기는 IPC 표면이 없어져 이 타입은 이제
> `bindings.ts` 에 생성되지 않는 순수 Rust 내부 타입이다.

```rust
struct AuxiliaryWindowInfo { label: String, project_id: ProjectId, window_slot: u32 }
```

- `label`(Rust 가 발급하는 `editor-<n>` OS 창 라벨)은 이 반환값에만 있고 §8 의 `AuxWindowLayout` 에는
  없다 — 레이아웃은 `slot`(프로젝트 범위 논리 id)만 기억하고, OS 창 라벨과의 매핑은 런타임
  `WindowStore` 가 들고 있다가 창이 닫히면 버려진다(영속되지 않음).

## 11. 설정 변경 브로드캐스트 — `SettingsChanged` 이벤트 (Wave I 신설 배선)

- `event::SettingsChanged { settings: Settings }` 는 이벤트 자체는 Wave I 이전부터 `events.rs` 에
  등재돼 있었으나 실제로 발신된 적이 없었다. Wave I 에서 `settings::commands::apply_and_broadcast` 가
  `settings_update`(patch)·`sync_download`(gist)·`app_file_write`(Settings 타깃, §9.2/§8) 세
  진입점의 공용 코어가 되면서 셋 다 이 이벤트를 전 창(+원격)에 발신하기 시작했다.
- 페이로드는 sanitize 를 마친 **전체** `Settings`(§3) 다 — 리스너가 왕복 없이 즉시 반영할 수 있게
  하려는 설계이며, 프론트 관례는 이 페이로드로 `SETTINGS.CURRENT` 쿼리를 직접 `setQueryData` 하는
  것이다(무효화 후 재조회가 아니다). 이벤트/커맨드 전체 카탈로그의 정본은
  `docs/ipc-contract.md`(§3 "Wave I 계약 확정 추가")이며, 이 절은 §2 의 `settings.json` 영속
  스키마가 변경 시 실제로 어떻게 전파되는지를 잇기 위한 최소 교차 참조다.

## 12. 손 QA 1차 발견 6건 수정 — 영속 스키마 영향 (2026-08-18)

> 계약: `docs/acknowledge/2026-08-18-hand-qa-fix-contract.md`. 이벤트/커맨드 전체 카탈로그는
> `docs/ipc-contract.md`(§3 "손 QA 1차 발견 6건 수정") — 이 절은 §2/§3 의 영속 스키마가 실제로 어떻게
> 바뀌었는지에 한정한다.

- **`TabKind::Diff` 확장**(§3): `rev`/`parentRev`/`beforePath` 옵션 필드 3종 — `rev` 가 있으면 그
  탭은 워킹트리/스테이지 diff 가 아니라 특정 커밋의 파일 diff 를 뜻한다. 전부
  `#[serde(default)]` 라 구 `layout.json`(이 필드 세트가 생기기 전에 저장된 파일)도 그대로
  역직렬화된다 — `compare_with` 가 이미 쓰던 것과 같은 하위 호환 패턴. `ProjectLayout.version` 은
  그대로 2 다(스키마 버전을 올릴 필요가 없는 순수 옵션 필드 추가).
- **`settings.json` — `remoteAllowedHosts` 항목 형태 확장**(§2): 항목 문자열에 `*.` 접두를 허용하는
  것만 바뀌었고, 필드 자체의 타입(`Vec<String>`)·저장 위치는 그대로다. 저장되는 값은 sanitize 를
  통과한 소문자 문자열이므로(대소문자 무시 매칭), 예전에 저장된 비-와일드카드 항목과 섞여 있어도
  파싱/마이그레이션이 필요 없다.
- **`settings.json` — `themeId` 변경 시 `followSystemTheme` 자동 false**(§2): 저장 포맷 변경은
  없다(두 필드 모두 기존 필드) — `settings_set_theme` 핸들러가 쓰는 값이 하나 더 늘었을 뿐이다.
  기존에 저장된 `{ themeId, followSystemTheme: true }` 조합은 다음 `settings_set_theme` 호출까지는
  그대로 유지된다(마이그레이션 없음, 읽기 시점 강제 정정 없음).
- **LSP 세션 dispose 유예**: `LSP_SESSION_DISPOSE_GRACE_MS` 유예·`lsp-session-flush-registry.ts`
  경유 확정 정리는 전부 **런타임 상태**(§1 표의 두 번째 층위, `sessionsByKey` — 프론트 메모리)다.
  영속 스키마에는 아무 영향이 없다.

## 13. T0 감사 데이터·기능 수정 — 영속 스키마·타입 영향 (2026-08-18)

> 계약: `docs/acknowledge/2026-08-18-audit-t0-fix-contract.md` §2.2~§2.4. 이벤트/커맨드 전체
> 카탈로그는 `docs/ipc-contract.md`(§3 "T0 감사 데이터·기능 수정") — 이 절은 §2/§3/§9/§10 의
> 영속 스키마·타입이 실제로 어떻게 바뀌었는지에 한정한다.

- **`SettingsPatch` — 클리어 가능 문자열 6필드에 3상태 규약 일반화**(#22, 사용자 결정 7): §7 이 이미
  `aiProvider`/`aiModel` 리네임을 다뤘던 그 두 필드에 더해 `shellOverride`·`editorFontFamily`·
  `terminalFontFamily`·`uiFontFamily` 까지 총 6필드가, `aiOmlxBaseUrl`(기능 확장 1차부터 있던
  선례)과 같은 **"필드 생략=건드리지 않음, `Some(\"\")`=`None` 으로 해제, 그 외=설정"** 규약을
  따른다. `Settings`/`SettingsPatch` 의 필드 타입 자체는 그대로 `Option<String>` 이라
  `Option<Option<T>>` 로 인한 `bindings.ts` 타입 분기는 없다 — 온전히 `apply_patch` 의 병합
  로직(`merge_clearable_string`) 쪽 의미 변경이다. 디스크(`settings.json`)에 저장되는
  `Settings` 자체의 형태는 바뀌지 않는다(값이 있으면 `Some(value)`, 없으면 필드 자체가 `null` —
  patch 의 "생략" 규약과 무관). 구 클라이언트(이 규약을 모르는)가 만든 `SettingsPatch` 는 클리어가
  필요한 필드를 계속 `null` 로 보내 "건드리지 않음"으로 해석되므로 하위호환은 유지되지만, 그
  클라이언트에서는 해당 필드를 시스템 기본값으로 되돌리는 조작 자체가 여전히 불가능하다(신 UI 로
  전환해야 해소).
- **`StatusRow`/`CommitFile` — 절대경로 필드 추가**(#18): `docs/features/git.md` §1,
  `docs/ipc-contract.md`(§3 "T0 감사 데이터·기능 수정")·본 문서 §10.2 참조. 두 타입 모두 신규
  필드만 추가된 순수 가산 변경이라 별도 마이그레이션은 없다(둘 다 비영속 IPC 응답 타입 — 디스크에
  쓰이지 않는다).
- **`SearchMatch` — 좌표 단위를 UTF-8 바이트에서 UTF-16 코드유닛으로 정정**(#23): 본 문서 §10.3
  참조. `SearchMatch` 자체가 비영속(§10.3 표제)이라 스키마 영향은 없다 — 값의 **의미**만 바뀐다.
- **`LspSessionStatus` — 백그라운드 재시작 후에도 `crashed` 유지**(#24): 새 variant 는 없다(기존
  `"running" | "crashed" | ...` union 그대로). `docs/ipc-contract.md`(§3 "T0 감사 데이터·기능
  수정")·`features/lsp.md` §5 참조.
- **레이아웃·터미널·프로젝트 — 타입 변경 없음**(#2·#15·#21): `project_close`(레이아웃 동기 flush)와
  터미널 탭 닫기(pty 세션 회수)는 전부 기존 커맨드의 내부 부수효과 확장이다 — `ProjectLayout`·
  `TabKind::Terminal` 등 어떤 영속 타입도 필드가 늘거나 줄지 않았다. `asset_protocol_scope` 회수(#15)는
  이후 add-only API 라는 근본 한계로 **롤백됐다** — 현재 상태·후속 계획은 §14 참조.

## 14. T1 정비 1차 배치 — Settings 타입 좁히기·프로젝트 종료 자원 회수 (2026-08-18)

> 계약: `docs/acknowledge/2026-08-18-audit-t1-batch1-contract.md` §1(T1-B·T1-J). 감사 근거:
> `docs/quality-assurance/2026-08-18-architecture-audit.md`(R5#3~#5·R5#9·R5#11·X1#7·X1#18·R4#7).
> IPC 시그니처·이벤트 카탈로그 영향은 `docs/ipc-contract.md`, 자원 회수 전체 목록의 정본은
> `docs/architecture.md` §6.3 — 이 절은 **영속 스키마·타입**에 한정한다.

- **`Settings`/`SettingsPatch` 열거형 5필드를 `Option<String>` → specta union 으로 좁힘**(T1-B
  핵심): `editorRenderWhitespace`(`"none" | "boundary" | "selection" | "all"`)·
  `editorCursorStyle`(`"line" | "block" | "underline"`)·`editorCursorBlinking`(`"blink" | "smooth" |
  "phase" | "expand" | "solid"`)·`terminalCursorStyle`(`"bar" | "block" | "underline"`) 4필드는 새
  Rust enum(`domain::settings::types::Editor{RenderWhitespace,CursorStyle,CursorBlinking}`·
  `TerminalCursorStyle`, 전부 `#[default]` variant 보유 — `Settings` 필드는 `#[serde(default)]`
  라 optional). `aiProvider`(`AiProviderId | null`, 기본값 없음)는 새 enum 을 만들지 않고 `ai_*`
  커맨드가 이미 쓰던 `domain::ai::types::AiProviderId`(`ollamaCloud | codex | omlx`)를 재사용한다.
  프론트 소비처(에디터·터미널·설정 화면) 9곳의 `as EditorRenderWhitespace` 류 캐스트가 전부
  제거됐다 — monaco/xterm 옵션 prop 타입과 값 집합이 완전히 동일해 매핑 함수 없이 구조적으로
  호환된다.
- **하위호환 — 디스크/동기화 페이로드의 out-of-range 값**: 타입이 문자열에서 enum 으로 좁혀지며
  "허용 밖 문자열이면 기본값으로 보정"이라는 기존 런타임 검증(`sanitize_enum`/
  `sanitize_optional_enum`)이 애초에 표현 불가능한 값을 막아 무의미해졌다. 대신
  `settings::service::sanitize_legacy_settings_values` 가 **타입 파싱 이전** raw JSON 단계에서
  이 5필드의 out-of-range/빈 문자열 값을 정규화한다(4개 열거형은 각자의 기본값, `aiProvider` 는
  `null`) — `migrate_legacy_ai_provider_keys`(§7)와 같은 경계(`read_settings_file`·
  `parse_settings_json`·`sync::service::parse_synced_payload`)에 나란히 적용돼, 이 narrowing
  이전에 저장된 `settings.json`이나 다른 빌드가 쓴 동기화 gist 도 그대로 로드된다. 회귀 테스트:
  `settings/service.rs`
  `손으로_편집된_settings_파일의_잘못된_값은_로드시_보정되어_재저장된다`.
- **`SettingsPatch.aiProvider` 의 병합 규약이 달라짐**: `Option<String>` 이던 시절엔 다른 5개
  clearable 문자열 필드(§13 #22)와 같은 "생략=유지·`Some(\"\")`=해제" 3상태 규약 후보였지만,
  `Option<AiProviderId>` enum 에는 "빈 문자열" 이 없다 — `apply_patch` 는 단순
  `patch.ai_provider.or(existing)` 로 병합하고, 한 번 설정된 provider 를 patch 로 명시적으로
  "설정 안 됨" 으로 되돌리는 경로는 여전히 없다(narrowing 이전과 동일한 갭, R5#1 문서화 그대로).
- **폰트 크기 범위 통일**(R5#4): `editorFontSize`/`terminalFontSize` 는 Rust 가 그동안 보정을
  전혀 하지 않았고 프론트는 `settings-view.tsx` 로컬 8–32 와 `shared/constants/code-font-size.ts`
  6–48 두 범위가 따로 있었다. `settings::service::sanitize` 가 이제 둘 다 `FONT_SIZE_MIN=6`/
  `FONT_SIZE_MAX=48`(zoom 스테퍼가 실제로 import 하던 쪽)로 clamp 하고, `settings-view.tsx` 는
  로컬 8–32 상수를 지우고 같은 `code-font-size.ts` 상수를 가져다 쓴다 — 세 벌이 한 벌로 수렴.
- **`agentStatusBadgeEnabled`/`ideAutoOpenDiff` 기본값 드리프트 제거**(R5#5): 두 필드 모두 Rust
  `Default`(`default_true()`)는 `true` 인데 `settings-view.tsx` 의 `??` 폴백은 `false` 였다(설정을
  한 번도 저장하지 않은 상태에서 Rust 기본값과 화면 표시가 반대였던 결함). 폴백을 `?? true` 로
  정정해 Rust `Default` 를 유일한 정본으로 삼는다 — 디스크에 저장된 `Settings` 자체는 항상 명시
  값을 갖고 있으므로(`#[serde(default)]`) 이 드리프트는 "아직 한 번도 저장 안 된 세션의 화면
  표시"에만 영향이 있었다.
- **`Settings` 필드 집합·테마 토큰 목록 파리티 테스트 신설**(R5#3·R5#9, 회귀 방지 — 새 필드/스키마
  없음): `settings/types.rs`
  `settings_필드_집합은_rust와_bindings_ts에서_일치한다` 가 `Settings::default()` 의 실제
  직렬화 키 집합과 생성된 `bindings.ts` 의 `Settings` 타입 필드 집합을 비교하고, `theme/service.rs`
  `테마_토큰_목록은_rust와_theme_tokens_ts에서_일치한다` 가 `COLOR_NAMESPACES`/`SYNTAX_TOKENS`/
  `TERMINAL_TOKENS`(약 200개 시맨틱 키)를 `src/entities/theme/theme-tokens.ts` 와 비교한다. 둘 다
  현재 상태에서 통과 — 미러 드리프트를 사후 감지하는 강제 장치가 이번에 처음 생겼다는 것이 변경의
  본질이다.
- **X1#18 — specta `_Serialize`/`_Deserialize` 분할 실제 인벤토리**: §9.1(위)이 이 분할을
  `SnippetEntry`/`SnippetFile` 2종만 기록하고 "두 절반은 현재 동일"이라 단정했던 것을 실측으로
  정정한다. 현재 `bindings.ts` 에서 실제로 쪼개지는 타입은 **7종**:
  `ResolvedLocale`·`ResolvedTheme`·`SnippetEntry`·`SnippetFile`·`Theme`·`TokenColorRule`·
  `TokenColorSettings`(전부 중첩된 `#[serde(untagged)]`/`Option<Vec<...>>` 필드가 원인으로 보임 —
  §9.1 이 짚은 것과 같은 현상). 실측(`bindings.ts` 직독)으로 둘로 나뉜다 — **5종**(`ResolvedLocale`·
  `SnippetEntry`·`SnippetFile`·`TokenColorRule`·`TokenColorSettings`)은 두 절반의 필드 구성이
  완전히 동일하지만, **`Theme`·`ResolvedTheme` 2종은 감사가 짚은 것보다 넓다** — `_Serialize`
  절반에서는 필수(`?` 없음), `_Deserialize` 절반에서는 옵셔널(`?`)인 필드가 `ResolvedTheme` 은
  5개(`syntaxOverrides`·`warnings`·`author`·`license`·`source`), **`Theme` 은 8개**
  (`extends`·`palette`·`colors`·`syntax`·`terminal`·`author`·`license`·`source` — 감사 X1#18 은
  `ResolvedTheme` 만 지목했으나 같은 비대칭이 `Theme` 에도 그대로 있다)로 갈린다. 프론트가 이
  타입들을 소비할 때(`theme_resolve`/`theme_get`/`theme_save` 등) 이 필드에 옵셔널 체이닝 없이
  접근하면 유니온의 `_Serialize` 절반만 만족해 타입 에러가 나거나, 반대로 `_Deserialize` 절반을
  가정하면 런타임엔 항상 채워져 있는 필드를 불필요하게 옵셔널 취급하게 된다 — 기능상 결함은
  아니지만 `ipc-contract.md`(§7.10-W7 `Theme`/`ResolvedTheme` 언급 포함)의 단순화된 표기가 이
  비대칭을 가려 왔다.
- **`GitStore`/`TreeStore` 프로젝트 종료 회수**(T1-J, R4#7): 두 스토어 모두 영속 스키마가 아니라
  런타임 캐시(`AppState` 밖의 별도 Tauri managed state)라 이 문서의 타입 표에는 애초에 없었다 —
  `project_close` 가 `GitStore::remove`/`TreeStore::remove` 로 캐시 엔트리만 지운다(재오픈 시
  다시 채워짐). 전체 회수 목록의 정본은 `architecture.md` §6.3 — **asset 프로토콜 접근이
  회수되지 않는다는 이 시점의 설명은 T1 2차(§15)에서 뒤집혔다**: 이제 매 요청마다 `AppState::
  projects` 를 직접 조회해 판정하므로 별도 회수 단계 자체가 필요 없다.

## 15. T1 정비 2차 배치 — 영속 스키마 영향 없음 확인 (2026-08-19)

> 계약: `docs/acknowledge/2026-08-18-audit-t1-batch2-contract.md`(T1-G·asset scope 재등록·T1-F).
> 감사 근거: `docs/quality-assurance/2026-08-18-architecture-audit.md`(C1·C10). **이 배치는
> 영속 스키마(`settings.json`·`layout.json`·미러 등, §2 디스크 레이아웃)를 전혀 건드리지 않는다**
> — 아래는 그 확인 근거를 남기는 절이다.

- **T1-G(remote 보안 헤더·http 클라이언트·shell 임시 파일 권한·lsp manifest 캐시·`ai_inline_complete`
  바이트 상한)**: 전부 응답 헤더·프로세스 내부 캐시·파일시스템 권한 비트·요청 크기 검증이다.
  디스크에 쓰는 새 파일도, `Settings`/`Layout` 등 기존 영속 타입의 필드 변경도 없다. 상세는
  `ipc-contract.md` "T1 정비 2차 배치" 절.
- **asset scope 재등록(X1#7)**: 판정 기준을 append-only Tauri 스코프에서 `AppState::projects`
  (이미 존재하는 런타임 상태, §1 "상태의 세 층위" 기준 **세션 스코프**)로 옮겼을 뿐, 새 저장
  구조를 추가하지 않았다. 프로젝트 목록 자체(`Project { id, root, name, capabilities }`)의 영속
  스키마(§3)는 무변경.
- **T1-F(레이어 이동)**: 프론트 컴포넌트 파일 12개의 디렉터리 이동 + import 경로 갱신이다.
  런타임 상태·영속 데이터와 무관.

## 16. T1 정비 3차 배치 — 세션 스코프(§1) 타입에 `owner`/`generation` 필드 추가, 영속 스키마 무영향 (2026-08-19)

> 계약: `docs/acknowledge/2026-08-19-audit-t1-batch3-contract.md`(T1-C·T1-D·T1-A 잔여). 감사 근거:
> `docs/quality-assurance/2026-08-18-architecture-audit.md`(C3·C4·C5). **이 배치도 영속 스키마
> (`settings.json`·`layout.json`·미러 등, §2)를 전혀 건드리지 않는다** — 변경 전부가 §1 "상태의 세
> 층위" 기준 **세션 스코프**(프로세스 재시작 시 사라지는 런타임 상태)의 IPC 타입/필드다.

- **§10.4 AI 요청 타입에 `owner: String` 추가(R6#20)**: `AiInlineEditRequest`/`AiCommitMessageRequest`
  (그리고 §10.4 표에서 제외됐던 `AiInlineCompleteRequest`, auto-tab)에 각각 `owner: String` 필드가
  붙는다 — Rust `AiRequestStore` 의 키가 `requestId` 단독에서 `(owner, requestId)` 복합으로 바뀌었기
  때문이다. `AiInlineEditResponse`/`AiCommitMessageResponse`(응답 쪽, 2026-08-19 시점)는 무변경 —
  이후 d-37(2026-08-25)로 이 둘과 `AiInlineCompleteResponse`가 `AiTextResponse` 하나로 통합됐다
  (§10.4). 세션 스코프
  (`AiRequestStore` 는 진행 중 요청만 잠깐 들고 있는 인메모리 맵)라 §3 핵심 타입에는 등재하지 않는다.
- **IDE 선택영역 타입(§10 인벤토리엔 없었음)에 `owner: String` 추가(R6#12)**: `IdeSelectionInput`
  (`domain::ide::types`)에 `owner: String` 이 붙는다. `IdeStore.current_selection` 자체의 저장
  형태(창별 슬롯이 아니라 여전히 전역 단일 슬롯)는 무변경 — `owner` 는 그 슬롯에 쓸지 말지를 가르는
  게이트(원격 고정 라벨이면 no-op)로만 쓰인다. 상세는 `ipc-contract.md` "T1 정비 3차 배치" 절.
- **검색 세션(§10.3 `SearchMatch` 의 상위 세션 개념, 영속 타입 아님)에 `owner` 스코프 추가(R7#8)**:
  Rust `SearchStore` 의 키가 `sessionId` 단독에서 `(owner, sessionId)` 복합으로 바뀐다 — 새 영속/IPC
  *타입* 필드 추가가 아니라 `search_run`/`search_cancel` 커맨드 인자에 `owner` 가 하나 늘어난 것.
- **LSP 세션 상태 이벤트에 `generation: u32` 추가(R7#1)**: `LspSessionStatusChanged` 이벤트와
  `lsp_sessions` 폴링 응답 타입(`LspSessionInfo`, §10 인벤토리엔 없었음) 모두 `generation: u32` 를
  얻는다 — 어느 쪽도 디스크에 쓰이지 않는 순수 런타임 신호다(자동 크래시 재시작 횟수 카운터, 세션
  자체와 함께 사라진다).
- **LSP 세션의 워크스페이스 root 집합(R7#7)**: Rust `SessionEntry.roots: Vec<(String, u32)>` (root →
  참조 카운트)는 배치 이전부터 있던 필드고, 이번 배치는 **렌더러 세션 키에 root 를 추가**해 이
  기구를 실제로 쓰게 만들었다. 어느 쪽이든 `LspStore`(세션 맵) 안의 런타임 상태일 뿐, 프로젝트가 실제로
  여는 워크스페이스 root 자체는 기존과 같은 방식(프로젝트 열기 시점의 파일시스템 스캔)으로 결정되어
  어떤 영속 스키마에도 새로 등재되지 않는다.

## 17. X-A 배선 + 소규모 잔여 청소 배치 — 영속 스키마 무영향 (2026-08-19)

> 계약: `docs/acknowledge/2026-08-19-xa-wiring-cleanup-contract.md`. 전체 커맨드/이벤트 카탈로그는
> `docs/ipc-contract.md`(§"X-A 배선 + 소규모 잔여 청소 배치" 절)가 정본이다. 이 배치도 §2 의 영속
> 스키마(`settings.json`·`layout.json`·미러 등)를 전혀 건드리지 않는다 — 전부 세션 스코프 런타임
> 상태 또는 커맨드 인자 형태 변경이다.

- **`AppState::self_writes`(신설, `infra::self_write::SelfWriteTracker`)**: `HashMap<PathBuf, Instant>`
  기반 인메모리 TTL 캐시 — 어떤 파일에도 영속되지 않고 앱 재시작 시 그냥 비어서 시작한다. `FsChange`
  의 `from_app: bool` 필드 자체는 신설이 아니다(기존 타입 그대로) — 이전엔 이 필드가 항상 `false` 로
  고정된 상수였고, 이번 배치가 실제 값을 채우기 시작했을 뿐이라 타입 모양은 무변화다.
- **`TerminalCwdChanged{ sessionId, cwd }`**: 타입 모양 무변화(§10 인벤토리 기준) — 이전엔 발행 지점이
  없어 죽은 타입이었고, 이번 배치가 실제로 값을 채워 발행하기 시작했다. `SessionEntry.cwd`(런타임,
  세션과 함께 사라짐)가 그 값의 보관처다.
- **`lsp_report_reinitialize_failure(sessionId, generation) → Result<null, AppError>`(신규 커맨드)**:
  새 영속/세션 타입을 추가하지 않는다 — 응답이 성공하면 §16 에서 이미 기술한 기존
  `LspSessionStatusChanged{ sessionId, status, lastError, generation }` 이벤트가 그대로 재사용되어
  `status: Crashed`/새 `lastError` 문구로 재발행된다.
- **`AuxiliaryWindowInfo`**: 이 타입을 직접 반환하던 `window_open_auxiliary` 커맨드가 중복으로
  제거되며(§10.5) 이제 어떤 커맨드 응답에서도 도달하지 않는다 — 타입 정의 자체(Rust 쪽)는 남아
  있지만 `bindings.ts` 에는 더 이상 생성되지 않는다. 영속 스키마(§8 `AuxWindowLayout`)와는 원래도
  별개 타입이라 그쪽엔 영향이 없다.
- **`tree_rows` 의 `limit: u32 → Option<u32>`**: 커맨드 인자 타입 변경(§1.3(7)) — 반환 타입
  `TreeRowPage` 는 무변화. 영속 스키마 없음(트리 캐시는 `TreeStore` 인메모리).

## 18. d-27 Welcome 확충 배치 — `Project.last_opened_at` 필드 추가 (영속 스키마 영향, 2026-08-20)

> 계약: `docs/acknowledge/2026-08-20-welcome-page-contract.md` §1.1. 커맨드/이벤트 카탈로그는
> `docs/ipc-contract.md`(project 절·"원격 dispatch 정책" 절)가 정본이다. §15~§17 과 달리 **이
> 배치는 §2 의 영속 스키마를 실제로 건드린다** — `projects/<id>/project.json` 에 필드가 하나
> 늘었다.

- **`Project`(`domain/project/types.rs`)에 `last_opened_at: f64` 추가** — 밀리초 epoch(§6 의 f64
  epoch-ms 규칙), `#[serde(default)]` 로 기존(pre-d-27) `project.json` 은 이 필드 없이도 `0.0` 으로
  파싱된다(무마이그레이션, §5). `project_open`(재열기 분기 포함)·`project_activate` 가 대상
  프로젝트의 이 필드를 갱신하고 `save_project` 로 즉시 영속한다.
- **`Project` 가 `derive(Eq)` 를 잃었다** — `f64` 는 `PartialEq` 만 구현하므로(`OpenedFile`/
  `MirrorEntry` 등 기존 f64 보유 타입과 동일 패턴), `Project` 를 `HashMap`/`HashSet` 키로 쓰는
  코드나 `Eq` 를 파생하며 `Project` 를 품는 타입이 있었다면 컴파일이 깨졌을 변경이지만 그런 지점은
  없었다(`PartialEq` 는 유지).
- **신규 조회 커맨드 `project_list_recent() → Vec<Project>`**: 새 영속 타입을 추가하지 않는다 —
  `projects/` 디렉터리 전수를 읽어 기존 `Project` 를 그대로 반환할 뿐이다. 원격 dispatch 는 거부
  (`RemoteDenialPolicy::LocalProjectHistoryExposure`, `docs/ipc-contract.md` 참조).
- **§2 디스크 레이아웃**의 `project.json` 설명에 `lastOpenedAt` 을 반영했다(이 절 갱신에 맞춰).
