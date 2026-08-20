# Welcome 페이지 확충 계약 (2026-08-20, d-27)

> 사용자 요청(2026-08-20): "Welcome page 에 file 열기, folder 열기, 최근에 열었던 프로젝트들
> (열려있다면 그 프로젝트로 탭변경) 기타 넣을만한것들 해서 꾸며서 넣어놔."
> 착수 전 메인 실사: `features/welcome/welcome-screen.tsx`(54줄) 기존재 — 폴더 열기(⌘O)+최근
> 목록 UI 는 있으나 **app-shell 이 `recentProjects={[]}` 하드코딩**(배선 부재). TabKind 에
> `{kind:"welcome"}` 탭 실재(사용자 스크린샷의 "Welcome ×"). 영속 프로젝트 기록
> (`~/Library/Application Support/dev.taide.app/projects/*/project.json`)은 id·root·name·
> capabilities·rootMissing 만 보유 — **최근 정렬용 타임스탬프 부재**(Rust 스키마 확장 필요).
> `project_list` 는 세션 프로젝트 대상(전 기록 열람 커맨드 부재 정황 — 구현 실사).

## 1. 범위

### 1.1 Rust — 최근 프로젝트 데이터 (표면 추가)

- 프로젝트 영속 기록에 `last_opened_at`(밀리초 epoch — IPC 시간 f64 규칙) 필드 추가(serde
  default 로 기존 파일 무해 호환). `project_open`(기존 재열기 포함)·`project_activate` 시 갱신.
- 신규 커맨드 `project_list_recent`(영속 기록 전수 — 최근순·rootMissing 포함) — **표면 추가
  절차 전수**: 배선 3곳 + 파리티 + T1-K ALLOWED/DENIED 등재(**DENIED 추천** — 로컬 웰컴 UI
  전용·원격 불요) + bindings 재생성(cargo test) + ipc-contract 문서. 기존 `project_list`
  의미 무변경.
- 도메인 경계: project 도메인 내 완결(화이트리스트 무변경 기대).

### 1.2 TS — Welcome 화면 확충 ("꾸며서")

- **폴더 열기**: 기존 유지(⌘O 라벨).
- **파일 열기(신규)**: 추천안 — 활성 프로젝트가 있으면 그 루트 기준 파일 선택 dialog →
  에디터 탭으로 열기(root guard 정합 — 프로젝트 밖 파일은 기존 가드 의미 유지 실사). 열린
  프로젝트가 없으면 비활성+힌트(단일 파일 편집은 프로젝트 모델 밖 — 임의 확장 금지).
- **최근 프로젝트 목록(배선 완성)**: `project_list_recent` 소비 — **이미 열려 있으면
  `project_activate` 로 전환**(사용자 명시 요구), 닫혀 있으면 `project_open(root)`.
  rootMissing 기록은 비활성 표시+사유. 표시는 이름 주 + 루트 경로 부제(d-26 2단 선례).
- **기타("넣을만한것들")** — 추천 구성: 주요 단축키 안내 카드(⌘P·⌘⇧F·⌃` 등 실제 APP_KEYMAP
  에서 유도 — 하드코딩 금지), 버전 표기는 실소비 부재 정황(app_get_info 프론트 소비 0) 실사
  후 필요 시만. 과설계 금지 — 문서 링크·뉴스 피드 류 외부 의존 금지.
- **적용 면 2곳 통일**: 프로젝트 0개 화면(app-shell)과 welcome 탭(kind:"welcome" 렌더러 —
  구현 실사로 위치 특정) 이 같은 컴포넌트를 쓰도록 정리(중복 금지). features 레이어 순수성
  유지 — 데이터·콜백은 상위(widgets/app-shell)에서 주입(FSD).
- i18n 신규 키 4곳 동기·3언어 실번역. 접근성(button·label) 기본 준수.

### 1.3 범위 외

- 원격 표면 확대(신규 커맨드는 DENIED)·프로젝트 삭제/고정 등 관리 기능·뉴스/업데이트 피드.
- 기존 프로젝트 기록 마이그레이션(장식 필드라 default 로 충분).

## 2. 실행·검증

- **선행: d-26b(테마)·d-26 검토 반영 완결 후 착수**(동일 트리 순차). Rust 포함 —
  tauri dev 재빌드로 앱 재시작 고지.
- 구현 Workflow(sonnet+xhigh 단독 — Rust 선행 후 TS 순차) → 메인 2차 → Phase E 4렌즈
  (정확성: 타임스탬프 갱신 지점·activate 전환 / 표면: 신규 커맨드 절차 전수·DENIED·파리티 /
  설계: FSD·컴포넌트 통일·과설계 / 계약: i18n·문서) → 적대적(major 이상) → 수정 → 커밋 →
  병합.
- 실기 확증(사용자): 웰컴에서 최근 프로젝트 클릭 전환·파일/폴더 열기·재시작 후 최근순 유지.

---

## 3. 구현 완료 기록 (Phase E 검토 전)

### 3.1 Rust — 표면 델타

- `Project`(`domain/project/types.rs`)에 `last_opened_at: f64`(`#[serde(default)]`, 밀리초 epoch)
  추가. `Project`가 `f64` 필드를 얻으며 `derive(Eq)` 를 제거(`f64` 는 `PartialEq` 만 구현 —
  `OpenedFile`/`MirrorEntry` 등 기존 f64 보유 타입과 동일 패턴)했고, `PartialEq` 는 유지했다.
- 갱신 지점 2곳: `service::open_project`(재열기 분기 포함 — 기존엔 `session` 만 저장했으나 이제
  `existing.last_opened_at` 갱신 후 `save_project` 도 호출)·`service::activate_project`(신규
  `projects: &mut HashMap<ProjectId, Project>` 매개변수 추가 — 시그니처 변경, 해당 project 를
  찾으면 갱신+저장, 없으면 스킵). `commands::project_activate` 도 `state.projects` 를
  읽기/쓰기하도록 배선(기존엔 `session` 만 다뤘음).
- 신규 커맨드 `project_list_recent() -> Vec<Project>`: `service::list_recent_projects` 가
  `projects/` 디렉터리 전수(`find_existing_project_id` 와 스캔 로직을 `iter_project_ids` 로 공통화)
  를 읽어 `root_missing` 재계산 후 `last_opened_at` 내림차순(`f64::total_cmp`) 정렬. 반환 타입은
  기존 `Project` 재사용(신규 타입 없음).
- 표면 추가 절차 전수: `lib.rs` `collect_commands!` 등록 → `dispatch.rs`
  `IMPLEMENTED_JSON_COMMANDS` 등재 → **`REMOTE_DENIED_COMMANDS` 로 등재**(신규
  `RemoteDenialPolicy::LocalProjectHistoryExposure` variant — `REMOTE_ALLOWED_COMMANDS` 에는
  올리지 않았고 `dispatch()` 의 `match` arm 도 추가하지 않음, DENIED 는 match 도달 전 단락) →
  `cargo test` 로 `bindings.ts` 재생성(신규 command 1개 + `Project.lastOpenedAt` 필드) →
  `docs/ipc-contract.md` command 표(176→177, 원격 ALLOWED 160 그대로/DENIED 19→20) 갱신.
  `tests/domain_boundaries.rs` 그린 확인(신규 도메인 간 참조 없음 — project 도메인 내 완결).
- 테스트: 정렬(`list_recent_projects은_last_opened_at_내림차순으로_정렬한다`)·세션 밖 기록 포함+
  rootMissing 표시·재열기/activate 갱신 지점 2건·구버전(필드 없는) `project.json` default 파싱
  — 총 5개 신규 + 기존 `activate_project` 시그니처 변경에 따른 테스트 갱신.

### 3.2 TS — Welcome UI 통일·확충

- **적용 면 통일**: 신규 위젯 `widgets/welcome/welcome-container.tsx`(`WelcomeContainer`) 가
  데이터(recent/open 프로젝트 목록)·뮤테이션(openProject/activateProject/openTab)을 전부 소유하고
  순수 `features/welcome/welcome-screen.tsx`(`WelcomeScreen`)에 props 로 주입. `app-shell.tsx`
  (프로젝트 0개 화면)와 `pane-node-view.tsx`(`TabKind.welcome` 탭 렌더러 — 이전엔 렌더러가 없어
  탭 제목만 보이는 범용 폴백으로 떨어졌음, 이번에 명시 케이스 추가)가 같은 `WelcomeContainer` 를
  `projectId` prop(0개 화면은 `null`, 탭 렌더러는 그 프로젝트의 id)만 다르게 써서 공유 —
  app-shell 의 기존 `handleOpenProject`/`openProject`/`activateProject` 바인딩은 컨테이너로
  이관되어 제거(중복 제거, minimal diff).
- **파일 열기(신규)**: `projectId` 가 있을 때만 활성(`canOpenFile`), tauri dialog(`defaultPath`=
  해당 프로젝트 root)로 선택 후 `layout_open_tab` 으로 `{kind:'file'}` 탭 오픈. 프로젝트 밖 경로
  선택 시 root guard 는 기존 `file_open`(`root_guard::resolve_owning_project`) 경로가 그대로
  적용(신규 특례 없음 — 계약 §1.2 그대로).
- **최근 프로젝트 목록(배선 완성)**: `project_list_recent` 소비. 현재 열린 세션 목록(`project_list`)
  과 대조해 열려 있으면 `project_activate`, 아니면 `project_open(root)`. `rootMissing` 은 버튼
  `disabled`+사유 텍스트로 표시.
- **단축키 안내 카드**: `APP_KEYMAP`(`shared/lib/keymap.ts`) 에서 6개 id(quick-open·
  command-palette·search·toggle-terminal·toggle-sidebar·save) 를 골라 `formatKeymapShortcut` 로
  렌더 — 라벨·단축키 문자열을 하드코딩하지 않음.
- **버전 표기**: 실사 결과 `app_get_info` 프론트 소비처가 0(계약 §1.2 사전 실사와 일치) — 추가하지
  않음(과설계 금지).
- `QUERY_KEY.PROJECT.RECENT`(`['project','recent']`) 신설 — `PROJECT.ALL`(`['project']`) 접두사라
  기존 `useOpenProject`/`useActivateProject` 의 `invalidateQueries({queryKey: PROJECT.ALL})` 가
  그대로 커버(추가 무효화 배선 불요). `query-key.test.ts` 분류표에 `scopedByProject: false` 로 등재.
- i18n 신규 키 4개(`app.openFile`·`app.openFileHint`·`app.recentProjectRootMissing`·
  `app.keyboardShortcutsTitle`) — en/ko/ja 3언어 실번역 + `locale/service.rs` 의
  `MESSAGE_NAMESPACES["app"]` 동기.

### 3.3 검증

- Rust: `cargo build --lib`·`cargo test --lib`(1060 passed)·`cargo test --test
  domain_boundaries`(3 passed)·`cargo fmt`·`cargo clippy --workspace --all-targets -- -D
  warnings`(0 경고).
- TS: `bun run typecheck`·`bun run lint`(사전 존재 경고만, 신규 에러 0)·`bun run format:check`·
  `bun test`(1424 passed)·`bunx vite build`(성공, 청크 크기 경고는 기존 monaco 번들 사유로 무관).
- 통합: `bun run verify` 전체 파이프라인(typecheck→lint→format→test→rust:fmt→rust:lint→
  rust:test) exit 0.
- 미실행: tauri dev 실기 확인(원격 X-Ops 정책상 앱 실행 금지 — 사용자 실기 확증 필요, §2 "실행 Workflow" 절 참조).
