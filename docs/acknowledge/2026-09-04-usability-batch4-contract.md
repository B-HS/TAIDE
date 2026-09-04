# 사용성 배치 4 계약 — OS 알림 · Welcome · 성능 극한 · 프로젝트 목록 · git 탭 UX · 터미널 메뉴 · 탭 바 메뉴 (2026-09-04)

> 사용자 요청 7건. 결정 정본 `2026-09-04-usability-batch4-user-decisions.md`. 조사 원문
> `docs/research/2026-09-04-batch4-*.md`. 진행은 역할표(`docs/agent-operations.md` §1) —
> 구현 wf opus·xhigh → 리뷰 wf sonnet·xhigh → 테스트 wf fable·medium. 상태 추적은 `docs/PROCESS.md`
> "사용성 배치 4" 절. 배치 3(`2026-09-04-usability-batch3-contract.md`)의 산출물 위에서 작업한다.

## 0. 공통 규칙 (구현·리뷰·테스트 전 에이전트)

- 배치 3 계약 §0 과 동일: 컨벤션 전부 준수(arrow fn · 코드 주석 금지(JSDoc 영어만) · any/enum 금지 ·
  매직넘버 금지 · useCallback/useMemo 금지 · FSD 위→아래 · barrel 금지 · 타입은 원본에서 유도 ·
  2회 이상일 때만 공통화 · minimal diff). 로케일은 `src-tauri/resources/locales/{ko,en,ja}.json` 3언어
  + `src-tauri/src/domain/locale/service.rs` `MESSAGE_NAMESPACES` 동시 등재(누락 시 Rust 테스트 실패).
- 신규 Rust 커맨드는 `lib.rs` specta 등록 · `remote/dispatch.rs` `REMOTE_ALLOWED_COMMANDS`/`DENIED`
  등재(완전분할 테스트) · `docs/ipc-contract.md` 갱신 · bindings 는 `cargo test` 재생성.
- 검증 사다리: `bun run typecheck` → `lint` → `format:check` → `bun test` → Rust 변경 시 `rust:fmt`·
  `rust:lint`·`rust:test`. cargo PATH:
  `export CARGO_HOME="$HOME/development/rust/cargo"; export RUSTUP_HOME="$HOME/development/rust/rustup"; export PATH="$CARGO_HOME/bin:$PATH"`
- 커밋·푸시 금지(메인이 수행). 설명은 코드 주석이 아니라 `docs/` 에.
- **파일 수정은 Edit/Write 도구로만** 한다 — 셸 heredoc(`python3 -`·`cat >`)·`sed -i` 편집 금지(승인 프롬프트
  폭주 원인, 2026-09-04 사용자 지적). 읽기는 `grep`·`sed -n`·`cat` 허용(허용 목록 `.claude/settings.json`).
- 조건 불충족 메뉴 항목 정책: 탭 메뉴는 "숨김"(`tabs.md` §3.1), 터미널 분할 방향은 "비활성 표시"
  (사용자 확정 전제 — 방향이 원래 존재함을 알려야 하므로).

## A. OS 네이티브 알림 (비활성 창 + 완료성 이벤트만, Rust 소유 게이트)

### A.1 현행 사실 (조사 완료 — 원문 `docs/research/2026-09-04-batch4-topics1-5-research.md` 주제 1)

- 토스트: sonner 153건/55파일, `AppToaster` 는 `AppProviders` 안에서 **모든 창(main·auxiliary·remote)** 이 렌더.
  sonner 에 전역 구독 API 가 없고 title/description 이 ReactNode 라 인터셉트로는 카테고리·텍스트를 얻을 수 없다.
- 완료성 이벤트 발생 지점: 에이전트 `agent:state-changed`(`AgentActivity Working/Idle/AwaitingInput`, 완료 토스트
  현재 없음, `AgentStateSyncProvider` 는 main·auxiliary 양쪽 마운트) · 태스크 러너(완료 신호 없음 — OSC 133 `D`
  exitCode 트래커 `terminal-osc133.ts` 재사용 가능) · git push/pull(`usePushGit`/`usePullGit`, 성공 토스트 없음) ·
  검색 치환(`search-panel-container.tsx:93` 성공 토스트) · LSP 설치(`lsp:install-progress`, 설정 탭 열린 창만 구독).
- 창 포커스: `query-client.ts` 가 `focusManager.setEventListener` 로 이 창의 포커스를 이미 추적하나 창 단위이고
  부팅 직후 `undefined→true` 폴백. **앱 전체 비포커스 판정은 Rust `webview_windows().values().any(is_focused)` 만 정확**
  (`is_focused` 는 core default 권한).
- `tauri-plugin-notification` 2.4.0: 데스크톱 `permission_state()`/`request_permission()` 이 **항상 Granted 스텁**,
  `show()` 실패도 `let _ =` 로 삼킴 → **권한 거부 감지 불가**. macOS dev(`tauri::is_dev()`)에서는
  `set_application("com.apple.Terminal")` 스푸핑으로 알림이 **Terminal 이름·아이콘**으로 뜬다(프로덕션 번들만 정상).
  JS 게스트 패키지는 주입된 `window.Notification` shim 에 의존(원격 미러 불가). 플러그인 init 스크립트가 부팅마다
  `is_permission_granted` 를 `.catch` 없이 invoke → capability 미개방 시 unhandled rejection 이 파일 로그에 남는다.
  신규 크레이트 3(`tauri-plugin-notification`·`notify-rust`·`mac-notification-sys`), 하위는 기존 Cargo.lock 에 존재.
  `notify-rust` 4.18 rust-version 1.89 → `src-tauri/Cargo.toml` `rust-version` 상향 필요. `mac-notification-sys` 는
  ObjC 정적 컴파일 + AppKit 만 링크(릴리스 자립성 게이트 통과 전망, `otool -L` 확인 필요).
- 시스템 설정 알림 창 URL: `x-apple.systempreferences:com.apple.Notifications-Settings.extension`(이 기기 Info.plist
  확증, OS 버전 의존). 기존 `validate_external_url` 은 http(s) 만 허용 → 별도 커맨드 필요.

### A.2 설계 (A안 — Rust 게이트 + 얇은 FE 파사드, 토스트 153곳 무수정)

1. **의존성**: `src-tauri/Cargo.toml` `tauri-plugin-notification = "2"`(최신 2.4.x), `rust-version` 을 실제값(≥1.89)으로
   상향, `lib.rs` `.plugin(tauri_plugin_notification::init())`, `capabilities/main.json` 에
   `notification:allow-is-permission-granted` **1개만**(init 스크립트 rejection 방지). npm 패키지 추가 없음.
   릴리스 자립성: `docs/deployment.md` §8 절차대로 `otool -L` 로 외부 dylib 0 확인(로컬 릴리스 빌드는 메인 2차에서).
2. **Rust `domain/notification/{mod,types,service,commands}.rs`** 신설(architecture.md 도메인 수 갱신):
   - `NotificationCategory { AgentCompleted, TaskCompleted, GitRemote, SearchReplace, LspInstall, Error }`(specta → TS union).
   - `service::decide_delivery(settings, category, any_window_focused) -> NotificationDecision { Deliver | Suppressed(reason) }`
     순수 함수 + 전 조합 테스트.
   - `notification_notify(category, title, body) -> NotificationDelivery`: settings 게이트(카테고리 토글·
     `notifications_only_when_unfocused` && `webview_windows().values().any(|w| w.is_focused().unwrap_or(false))`) →
     통과 시 `app.notification().builder().title(..).body(..).show()`; 결과 enum 을 반환(플러그인이 실패를 삼키므로
     Deliver 는 "시도했다" 의미 — doc 에 명기).
   - `notification_open_system_settings()`: `tauri_plugin_opener::open_url("x-apple.systempreferences:com.apple.Notifications-Settings.extension")`
     (macOS 외 타깃은 NotSupported 에러). URL 상수는 `constants.rs`.
   - 둘 다 `REMOTE_DENIED_COMMANDS` + `UnreachableDesktopWindow`. specta 등록·bindings·`ipc-contract.md`.
3. **설정**(d-53 패턴, 플랫 bool 8): `notifications_enabled`(true) · `notifications_only_when_unfocused`(true) ·
   `notify_agent_completed`·`notify_task_completed`·`notify_git_remote`·`notify_search_replace`·`notify_lsp_install`·
   `notify_error`(전부 true). `SettingsPatch`·`Default`·`apply_patch`·`settings_to_sync_patch`·`emptySettingsPatch`.
4. **FE 파사드** `src/entities/notification/notify.ts`(IPC 소유이므로 entities): `notifyNative({ category, title, body })` —
   (1) `getWindowContext().kind !== 'main' || isRemoteMirrorRuntime()` 이면 즉시 반환(중복·미러 차단),
   (2) `focusManager.isFocused()` 로 1차 조기 컷(설정 onlyWhenUnfocused 를 FE 가 모르므로 settings 쿼리 캐시를 읽거나
   컷을 Rust 에 위임 — 구현자는 settings 캐시가 있을 때만 조기 컷, 없으면 IPC), (3) `notificationNotify` IPC.
   판정 부분은 순수 함수 `shouldForwardNativeNotification({ windowKind, isRemoteMirror })` 로 분리 + 테스트.
5. **발생지 배선**(8~12곳): `src/app/providers/native-notification-provider.tsx`(main 창 전용 마운트 — `app.tsx` 분기)
   가 `agent:state-changed` 의 `Working→Idle|AwaitingInput` 전이를 감시하되 **Working 지속 ≥ 10초
   (`AGENT_COMPLETION_NOTIFY_MIN_WORKING_MS` 상수)** 일 때만 발화(에이전트별 시작 시각 Map — 판정은 순수 함수 +
   테스트), `lsp:install-progress` 종료 시 발화. 태스크: `terminal-osc133.ts` 트래커에 `onCommandFinished({ exitCode,
   durationMs })` 콜백을 추가하고 `terminal-view.tsx` → `terminal-session.tsx` 에서 **`durationMs ≥ 10초` 인 명령만**
   TaskCompleted(제목에 exitCode 성공/실패). git 원격: `usePushGit`/`usePullGit`(+ fetch/sync 경로) onSuccess/onError.
   검색 치환: `search-panel-container.tsx` 성공 지점. Error: `describeIpcError` 토스트 중 위 5카테고리 실패 지점만
   (전역 미러는 하지 않음 — C안 기각).
6. **설정 UI**: `settings-notification-section.tsx` 신설(SETTINGS_SECTION_ID/TOC 추가) — SwitchField 8 + "알림 설정
   열기" 버튼(`notification_open_system_settings`, 실패 시 설정 경로 텍스트 폴백 토스트) + "테스트 알림 보내기" 버튼
   (Suppressed 사유를 토스트로 표시) + dev 실행 시 'Terminal' 이름으로 뜨는 제약 힌트 문구. 첫 네이티브 발송 세션당
   1회 안내 토스트(sonner `action` 으로 설정 창 열기 — `use-explorer-entry-crud.ts` 선례) — "이미 안내함" 은 모듈
   스코프가 아니라 `useRef`/세션 state 로.
7. **로케일**: `settings.notifications*`(섹션 제목·8 스위치·설명·버튼 2·dev 힌트·안내 토스트), 알림 제목/본문 키
   `notification.agentCompleted`·`taskCompleted{Succeeded,Failed}`·`gitRemote{Push,Pull}{Succeeded,Failed}`·
   `searchReplaceDone`·`lspInstall{Succeeded,Failed}` 3언어 + 네임스페이스 등재. 알림 텍스트는 FE 가 `t()` 로 만들어
   Rust 에 전달(Rust 는 텍스트를 모른다).
8. **문서**: `docs/features/settings-ui.md`(섹션·제약), `docs/architecture.md`(도메인 추가·게이트 소유권),
   `docs/ipc-contract.md`, `docs/data-model.md`(설정 8필드), `docs/tech-stack.md`(플러그인), 백로그(UNUserNotificationCenter
   전환·권한 상태 조회 크레이트).

## B. Welcome 커맨드 + 탭 0 자동 표시

### B.1 현행 사실 (주제 2)

- Welcome 은 `TabKind::Welcome` 탭(`default_layout()` 이 제목 리터럴 "Welcome" 으로 생성)과 프로젝트 0개 화면이 같은
  `WelcomeContainer`(lazy 청크)를 쓴다. `open_tab` 은 kind 중복 시 기존 탭을 활성화하되 **title 을 갱신하지 않는다**.
- 빈 pane 렌더: `pane-node-view.tsx` `{!activeTab && <div>{t('editor.noFileOpen')}</div>}` — `!activeTab` 이 "이 창의 탭 0"
  과 동치. 보조창은 트리가 비면 스스로 닫힌다(`auxiliary-window-shell.tsx isPaneTreeEmpty`, Wave I §3.2) → 메인 창 게이팅 필수.
- 커맨드 선례 `settings.open`(활성 프로젝트 없으면 `app.openProjectFirst` 토스트). `CommandContext` 확장 시
  `command-catalog.test.ts`·`command-registry.test.ts` 의 `dummyContext` 동반 수정. 로케일 `app.welcome` 은 현재 없음.

### B.2 설계

1. `view.welcome` 커맨드: `command-registry.ts` `CommandContext.openWelcomeTab`, `command-catalog.ts` 에
   `{ id: 'view.welcome', titleKey: 'app.welcome', categoryKey: KEYMAP_CATEGORY.VIEW, run: (c) => c.openWelcomeTab() }`
   (`view.git` 뒤), `command-palette.tsx` 에서 `openSettingsTab` 선례를 복제해 `openTab({ kind: { kind: 'welcome' },
   title: WELCOME_TAB_TITLE, target: currentWindowFocusedPane(layout), preview: false })`. `WELCOME_TAB_TITLE = 'Welcome'`
   상수는 `shared/constants/tab.ts`(신설) — Rust `default_layout()` 과 동일 리터럴. 기본 단축키 없음. 테스트 2곳 갱신.
   로케일 `app.welcome`(en Welcome / ko 시작 화면 / ja ようこそ画面) + 네임스페이스. G 절 탭 바 여백 메뉴의 "Welcome 열기"
   항목은 같은 `openWelcomeTab` 경로를 쓴다(pane-tab-bar 에서 `useOpenTab` 으로 직접 호출).
2. 탭 0 자동 표시(A안): `pane-node-view.tsx` 의 빈 상태를 `getWindowContext().kind === 'main' && settings.welcomeOnEmptyEditor`
   일 때 `<Suspense><WelcomeContainerLazy projectId={projectId} /></Suspense>` 로 교체(보조창·설정 off 는 기존 문구).
   레이아웃·Rust 무변경.
3. 설정 `welcome_on_empty_editor: bool`(기본 true, d-53 패턴 전 파일 + `settings-interface-section.tsx` SwitchField +
   로케일 `settings.welcomeOnEmptyEditor(+Description)`).
4. 문서: `command-palette.md` §2.1(24종), `tabs.md` §2·`PRD.md` FR-B2 의 `File(웰컴)` → `TabKind::Welcome` 정정,
   `layout-shell.md` 에 "빈 에디터 영역 = Welcome(메인 창·설정)" 절. QA 체크 항목: 마지막 탭 닫힘 시 Welcome 즉시 표시.

## C. 성능 극한 최적화 — 계측 8지표 · 안전 수정 · 2단계 개선 (FE + Rust)

### C.1 현행 사실 (주제 3a·3b — 원문 참조; 착수 전 실물 재확인)

- FE: 계측 코드 없음(부팅 마크 없음). 실측된 순수 함수 비용 — 팔레트 fuzzy 5k=3.96ms / 20k=12.68ms / 50k=31.23ms(H3),
  키맵 디스패치 O(n) 0.002ms(기각), 검색 flatten 0.33ms(기각). 남은 후보: M3 `FILE.CONTENT/RAW` 캐시 회수(다른 창·스플릿·
  diff 고려 — `collectOpenFilePathsOutsideProject` 선례 재사용), L2 키바인딩 충돌 인덱스화, 가상화 미적용 목록(git 변경
  목록·아웃라인·팔레트 결과), H2 마커 구독(전량 스냅샷 — `useSyncExternalStore` 불변 참조 계약 주의), L3 오버레이
  스크롤바 관찰. monaco 지연 로딩(안 C, −3.76MB)은 **별도 배치**.
- Rust: 계측 없음. 저위험 4건 — `list_themes` 재파싱(설정 토글마다 1.1MB → OnceLock 요약), `flush_dirty_layouts` fsync
  를 async 워커/spawn_blocking 으로, `tree read_children` `metadata()→file_type()`(심링크 픽스처 테스트 선행), 파일
  sniff 이어읽기 공유 리더. 대형 3건 — `project_open` 락 후절화(attach build/register 2단, `attach_all` 등록 순서 계약
  `architecture.md` §3·§6.3 + lib.rs 소스 스캔 테스트 유지, attach 완료 시 git status 1회 refresh 보정), 워처 IdCache
  필터(`ScopedIdCache: FileIdCache`, 무시 디렉토리 경계 rename 이 From/To 2건이 되는 점 — `layout_apply_path_change`
  영향 선행 확인), git status 결과 캐시(워처 무효화 + **2초 TTL**, `fs:changed` 축 연결 필수, `update_index` 미채택).
  제외: 트리 응답 재설계(ipc-contract "불변")·`ps` fork 제거·전역 guard 분할.
- 테스트 하네스: bun 은 테스트 파일 간 모듈 미격리, `mock.module` 프로세스 전역(현재 16파일·37회, restore 0).

### C.2 설계 — 1단계(계측 + 안전 수정) → 2단계(대형 3건 + 가상화 묶음), 같은 웨이브 직렬

1. **Rust 계측** `src-tauri/src/infra/perf.rs`: `PerfRegistry`(`&'static str` 이름 고정 슬롯 + `AtomicU64` 누적/횟수/최대,
   전역 Mutex 금지) · `perf::span(name)` RAII(게이트 off 면 `AtomicBool::load` 1회) · 게이트 = 환경변수 `TAIDE_PERF=1`
   (릴리스 포함, 기본 off; `cfg!(debug_assertions)` 빌드는 기본 on). span 위치: 커맨드 디스패치 클로저(이름별 카운터),
   setup 4단계, `project_open`·`project_activate`·`file_open`·`tree_toggle/reveal`·`git_status`·`search_*`·`search_list_files`·
   `pty` 는 **바이트/횟수 카운터만**(고빈도 span 금지)·`lsp_send` 카운터. `perf_snapshot()`/`perf_reset()` 커맨드
   (app 도메인, `REMOTE_DENIED`). `[profile.profiling]`(strip=false, debug=1) 워크스페이스 `Cargo.toml` 에 추가 — CI·
   `tauri build` 미참조를 문서에 명기.
2. **FE 계측** `src/shared/lib/perf-mark.ts`: `perfMark(name)`/`perfMeasure(name, from)` — `performance.mark/measure` 래퍼,
   게이트는 `import.meta.env.DEV || perfEnabled`(Rust `perf_snapshot` 로 게이트 상태 조회 or `TAIDE_PERF` 를 부팅 시
   `app_info` 류 기존 IPC 로 전달 — 구현자가 기존 부팅 정보 IPC 를 재사용), off 면 no-op. 발행 상한(세션당 마크 수
   `PERF_MARK_LIMIT`)과 로그 포워딩은 dev 에서만(40KB 회전 로그 보호). 마크 지점: 부팅(`main.tsx` 시작·React 마운트·
   `use-reveal-window` 첫 페인트 게이트), 프로젝트 전환, 파일 열기(탭 클릭→에디터 ready), 팔레트 열기·입력 응답,
   트리 펼침, 검색 결과 렌더, 탭 전환, 터미널 출력 처리량(바이트/초 카운터). 개발용 스냅샷 표시는 커맨드
   `app.showPerfSnapshot`(콘솔 테이블 출력)로 최소화.
3. **벤치·예산 테스트**: `scripts/bench-frontend.ts`(순수 함수 시간 벤치, 수동 실행, `package.json` `bench` 스크립트)
   + `bun test` 에는 **연산 횟수 카운터 기반 예산 테스트만**(벽시계 단언 금지). Rust 는 `cargo test` 에 카운터 예산
   테스트(예: `list_themes` 재파싱 0회).
4. **FE 안전 수정 3건**: H3 fuzzy(소문자 사전계산·조기 종료 — `indices` UTF-16 계약·서로게이트/İ 테스트 선행 보강),
   M3 캐시 회수(탭 전부 닫힌 뒤 한 지점, 다른 창·스플릿·diff·프리뷰 열림 판정 재사용, d-43 클로버 계약 유지),
   L2 키바인딩 충돌 인덱스화(`keybinding-catalog.ts` Map 기반, 기존 테스트 유지).
5. **Rust 안전 수정 4건**: `list_themes` 요약 OnceLock/캐시(테마 파일 변경 시 무효화 경로 확인), `flush_dirty_layouts`
   fsync 비동기(순서·유실 없음 테스트), `read_children` `file_type()`(DT_UNKNOWN 폴백·심링크 픽스처 테스트),
   sniff 이어읽기 공유 리더.
6. **2단계 Rust 대형 3건**(순서 고정): ① `project_open` 후절화 → ② IdCache 필터 → ③ git status 캐시(TTL 2초 +
   `fs:changed`/`git/watch.rs` 무효화 + `architecture.md` §6.3 회수 목록 등재 + `project::capability` 회수). 각 건마다
   전후 `perf_snapshot` 수치를 계약 §3 에 기록(cargo test 픽스처 기준).
7. **2단계 FE 가상화 묶음**(접근성 동반 필수): git 변경 목록 가상화 + 컨텍스트 메뉴 단일 인스턴스 + 키보드 로빙을
   `file-tree.tsx` 컨테이너 로빙(인덱스 기반 + `scrollToIndex`)으로 통일(E 절의 섹션 헤더 로빙과 **같은 구현**이어야
   한다 — E 절 구현자가 먼저 만든 `git-sections`/로빙 위에 얹는다), 아웃라인 평탄화·가상화·접기(`outline-rows.ts`
   순수 함수 + 테스트), 팔레트 결과 가상화(cmdk 와 공존 확인 — 불가하면 상한 유지 + 사유 기록), H2 마커 severity
   카운트 2단 셀렉터(캐시된 동일 참조 반환), L3 스크롤바 관찰 축소.
8. **문서**: `docs/quality-assurance/2026-09-04-perf-baseline.md`(8지표·측정 절차·체크박스 — 실기 측정은 사용자),
   `docs/architecture.md`(perf 인프라·게이트), `docs/ipc-contract.md`(perf 커맨드), `docs/debugging.md`(TAIDE_PERF 사용법),
   백로그(monaco 지연·트리 응답 재설계·ps fork·update_index·감사 §1 이월분 등재).

## D. 프로젝트 목록 아이콘 · 라벨 · 색 (우클릭 → 표시 설정 다이얼로그)

### D.1 현행 사실 (주제 4)

- `app-sidebar.tsx` → `sortable-project-icon.tsx`(dnd-kit, 컨텍스트 메뉴 존재) → `features/project/project-icon-button.tsx`
  (`<Folder/>` 고정, `initialsOf` dead code, 40px 버튼에 overflow-hidden 없음, aria-label = 프로젝트 이름, 툴팁 이름+루트).
- `Project`(project.json) 와 `ProjectRef`(session.json) 가 root/name 을 `upsert_project_ref` 한 곳에서 미러. 신규 필드는
  `#[serde(default)]` 로 마이그레이션 0(d-27 `last_opened_at` 선례). `ProjectListChanged{projects: ProjectRef[]}` 이벤트가
  이미 fanout 되고 `ipc-sync-provider` 가 `PROJECT.LIST` 를 무효화한다.
- 색 토큰: `graph.lane1..12`(`var(--taide-graph-laneN)`, `commit-graph.tsx` 단독 소비). 아이콘 레지스트리 선례
  `shared/icons/file-icon-registry.ts`(`Record<name, ComponentType<LucideProps>>`). `lucide-react/dynamic` 전량은 청크 2007개 → 기각.
- PRD FR-A4("focus 된 content 타입에 따라 아이콘 변경")는 구현 0 → 보류 각주.

### D.2 설계 (A' 저장 + U2 다이얼로그 + C1 카탈로그)

1. **Rust**: `domain/project/types.rs` `ProjectDisplay { icon: Option<String>, label: Option<String>, color: Option<String> }`
   (`#[serde(default)]`, `Default`), `Project.display`·`ProjectRef.display` 양쪽 `#[serde(default)]`, `upsert_project_ref` 가
   미러. `service::set_project_display(id, patch)` — sanitize: icon `[a-z0-9-]{1,64}`, label trim·제어문자 제거·1~4
   코드포인트, color `lane1..lane12`; 해제 규약은 `settings/service.rs merge_clearable_*`("빈 문자열 = 해제 / 미지정 = 유지")
   재사용. `save_project` + `save_session` 후 `ProjectListChanged` emit(신규 이벤트 0). 커맨드 `project_set_display`
   (`REMOTE_ALLOWED`, `project_reorder` 와 같은 등급) + 테스트(양쪽 파일 재로드 동일값·sanitize·해제).
2. **FE**: `shared/icons/project-icon-registry.ts`(큐레이션 약 56종 — 코드·웹·모바일·DB·인프라·디자인·문서·테스트·AI·
   게임·금융·기타, `ProjectIconName` union, 미지 이름 `folder` 폴백), `shared/lib/project-display.ts`
   `resolveProjectDisplay(ref) => { mode: 'label' | 'icon' | 'default', label, icon, colorVar }` **단일 폴백 지점** + 테스트,
   `shared/constants/project-display.ts`(`PROJECT_LABEL_MAX_CODEPOINTS = 4`, 길이별 타이포 클래스 사다리
   `PROJECT_LABEL_CLASS_BY_LENGTH`, 색 목록). `entities/project/project.ipc.ts`·`project.query.ts` `useSetProjectDisplay`
   (onSuccess `PROJECT.LIST` 무효화 — 이벤트가 오지만 즉시성). `project-icon-button.tsx`: 폴백 사다리(label → icon →
   `<Folder/>`), `overflow-hidden`, 글리프/텍스트 색만 `style={{ color: colorVar }}`, aria-label 은 프로젝트 이름 유지.
   `features/project/project-display-dialog.tsx`(표시 모드 라디오 3택 배타 + `shared/ui/command` 검색 아이콘 그리드 +
   maxLength 라벨 입력(코드포인트 기준) + 12색 스와치 + 실시간 미리보기; `create-tag-dialog.tsx` 열림 전이 리셋 패턴).
   `sortable-project-icon.tsx` 컨텍스트 메뉴에 "프로젝트 표시…" + "기본값으로 되돌리기"(커스터마이즈 시만 활성).
3. **로케일**: `project.display*`(메뉴 2·다이얼로그 제목·모드 3·라벨 필드·색·저장·취소·미리보기) 3언어 + 네임스페이스.
4. **문서**: `layout-shell.md`(사이드바 표시 규칙), `data-model.md`(ProjectDisplay), `ipc-contract.md`, `PRD.md` FR-A4 보류
   각주, 백로그(Welcome 최근 목록·보조창 타이틀바 확장 시 `ProjectDisplayChanged` 이벤트).

## E. git 탭 stash/changes 구분 UX (VS Code SCM 파리티)

### E.1 현행 사실 (주제 5)

- 모호함의 원인은 색이 아니라 배치: `git-panel-container.tsx canStash = rows.length > 0` 때문에 **스태시 0건이어도 변경이
  있으면 stash 섹션이 목록 최상단에** 렌더된다(`git-panel.tsx`). 섹션 헤더는 `resource-group-header.tsx`(hover 액션만,
  키보드 도달 불가), Graph 헤더만 인라인 마크업(이중 스타일). 접이식 헤더 선례 `shared/ui/file-group-header.tsx`, sticky
  선례 `settings-view.tsx` + `ScrollContainer`. i18n 키 6종 기존재(`git.stash`·`git.noChanges` 등), 신규 키 0.
- 키보드 로빙 `change-row-navigation.ts` 는 "그룹 헤더 포커스 = activeIndex -1" 정의 → 헤더 포커스화 시 셀렉터·doc·
  테스트 동반 수정 필수(`features/git.md` §2 계약).

### E.2 설계 (A안)

1. 섹션 순서 **Merge Changes → Staged Changes → Changes → Stashes → Graph**. 변경 그룹은 0건이면 미렌더(현행), 3그룹 모두
   비면 `git.noChanges` 한 줄. Stashes 섹션은 `stashes.length > 0` 일 때만 렌더, Stash 실행 버튼은 상단 헤더 바(Sync 옆,
   `disabled={!canStash}`)로 이동.
2. 공용 접이식 헤더 `features/git/git-section-header.tsx`(`file-group-header` 선례 확장): `sticky top-0 z-10
   bg-explorer-background border-t border-app-border` + chevron(`rotate-90`) + 개수 뱃지(접혀도 표시) + hover/focus-within
   액션 + `data-git-section-header` + 포커스 가능. 변경 그룹·Stashes·Graph 전부 이 컴포넌트로 통일(`resource-group-header.tsx`
   대체·삭제). 배경 톤 차 없음(임포트 테마 충돌 회피). 행은 `pl-4` 들여쓰기.
3. 접힘 상태: `entities/git/git-section-collapse-memory.ts` 모듈 스코프 메모리(`commit-message-memory.ts` 선례, 재시작 시
   초기화, 기본 Stashes 만 접힘) + 테스트. `architecture.md` §6.4 소유권 표 등재.
4. 순수 함수 `widgets/git-panel/git-sections.ts` `buildGitSections({ rows, stashes, conflicts, collapsed })`(가시성·순서·
   카운트·접힘 해석) + 테스트. 로빙 셀렉터를 `'[data-git-change-row], [data-git-section-header]'` 로 확장, 헤더에서
   ←/→ 로 접기/펼치기, doc comment·`change-row-navigation.test.ts` 갱신. C.2-7 의 가상화는 이 구현 위에 얹는다.
5. 문서 `features/git.md` §2·§(패널 구성) 갱신 + QA 체크리스트(sticky·chevron·포커스 이동 손 QA).

## H. 테스트 대폭 보강 · 하네스 도입 · CI 게이트

### H.1 현행 사실 (주제 3c)

- bun 1817·cargo 1248·e2e 13. 미로드 LOC 47.3%(`.ts` 70파일·`.query.ts` 10종 저커버·`.tsx` 172 미커버). RTL 미도입.
  `mock.module` 프로세스 전역·restore 0. 테스트는 릴리스 태그 push 에서만 실행(dev·main push·PR 게이트 없음).
  e2e 14스펙 이상 확장 시 파일럿 C8(스위트 후반 webkit 열화 — Vite HMR WS 재시도 원인) 재발 → 원격 페이지 HMR
  클라이언트 비활성(dev 전용) 선결. e2e 신규 스펙은 `editorFontFamily` 등 `Option` 설정을 건드리면 원복 불가(하네스 §7).

### H.2 설계

1. **하네스**: devDependencies `happy-dom`·`@testing-library/react`·`@testing-library/dom`(각 최신, 정확한 버전은 `bun add -d`
   결과), `bunfig.toml` `[test] preload = ["./src/shared/testing/dom-preload.ts"]`(GlobalRegistrator + `afterEach(cleanup)` +
   `beforeEach(mock.restore)` 규약), `src/shared/testing/render.ts`(providers 래퍼 — QueryClient·i18n 최소). 커버 범위 한계
   (WKWebView 고유 결함은 e2e 전용)를 `docs/memory/test-conventions.md`(신설)에 명기. `tsconfig.json` types 갱신.
2. **단위 테스트 P0+P1**: 순수 모듈 ~10파일(`callback-registry`·`remote-json`·`keymap-capture`·`xterm-theme`·
   `system-appearance`·`window-appearance`·`inline-completion`·`file-icon-registry`·`explorer-path` 등 — 모듈 전역 상태
   리셋 훅 필수) + `.query.test.ts` 10종(`file`·`settings`·`project`·`search`·`theme`·`task`·`plugin`·`tree`·`lsp`·`layout`·`git`:
   `queryOptions().queryFn`·`QUERY_KEY`·mutation 무효화 대상 검증). 배치 3·4 신규 코드의 순수 함수·훅·컴포넌트 테스트
   (하네스로 `useExplorerAutoReveal`·`useOpenFileTab`·`TerminalContextMenu`·`TabBarContextMenu`·`ProjectDisplayDialog`·
   git 섹션 헤더·Welcome 빈 pane 분기).
3. **Rust 테스트**: `infra/archive.rs`(압축률 높은 반복 데이터로 예산 초과 케이스)·`error.rs`·`events.rs`·
   `domain/window/commands.rs`(State 의존 → 순수 부분 `service` 로 밀어내기)·`terminal/tree/locale/vsix/file` 확장·
   `tests/capability_symmetry.rs`. 배치 3·4 신규 Rust(navigation_guard·open_tab_in_split·notification decide·
   project display sanitize·perf registry) 전수.
4. **e2e**: 선결 — 원격 페이지 HMR 클라이언트 비활성(dev 전용, `vite.config.ts`/원격 index 주입 지점 확인). 신규 스펙
   14~20: explorer CRUD 충돌·탭 rename 추종·터미널 재부착·스플릿 동일 파일 저장·탭 pin 가드 + 배치 3(퀵오픈 삭제 파일
   NotFound 토스트·autoReveal 선택) + 배치 4(Welcome 커맨드·탭 0 Welcome·탭 바 여백 메뉴·터미널 분할 새 터미널·git
   섹션 접기). `e2e/lib/explorer.ts` 헬퍼. `Option` 설정은 건드리지 않는다.
5. **CI**: `.github/workflows/ci.yml` 신설 — `push`(dev·main) + `pull_request`: `frontend`(ubuntu: bun install·typecheck·lint·
   format:check·bun test·typecheck:e2e), `rust`(macOS: fmt·clippy·test, `cache-warm.yml` 캐시 키 재사용). `actionlint`
   통과. `docs/deployment.md` 에 CI 절 추가. 커버리지 임계 미도입.
6. 문서: `docs/quality-assurance/2026-09-04-test-gap-map.md`, `docs/tech-stack.md`(하네스), `docs/memory/test-conventions.md`.

## F. 터미널 우클릭 컨텍스트 메뉴 (분할 = 그 방향에 새 터미널 생성)

### F.1 현행 사실 (조사 완료 — 구현 전 실물 재확인 필수, 원문 `docs/research/2026-09-04-batch4-terminal-tabbar-context-menu-research.md`)

- 터미널에 컨텍스트 메뉴 없음. `terminal-view.tsx` 마크업은 `<div ref={containerRef} className='h-full w-full' />`
  한 줄. 우클릭은 xterm 이 이미 처리한다: `Clipboard.ts rightClickHandler`(숨은 textarea 를 커서 아래로 옮겨
  focus·select, `preventDefault` 없음) + **`rightClickSelectsWord` 기본값이 macOS 에서 true** 라 우클릭이 커서
  아래 단어를 선택한다.
- 네이티브 메뉴 억제는 메인 창 `app-shell.tsx handleNativeContextMenu`(document 버블) 뿐 — 보조 창은 네이티브
  메뉴가 그대로 뜬다. Radix Trigger 는 자체 `preventDefault` 를 하므로 Trigger 를 붙이면 보조 창도 닫힌다.
- `layout_split(target_pane, edge, tab_id)` 은 **기존 탭 이동**이다(`service.rs split` — `extract_tab` 후 새 Leaf).
  터미널 탭은 탭 우클릭 메뉴(`tab-context-menu.tsx` Split 서브메뉴)·`⌘\` 로 이미 4방향 이동 가능.
- "새 터미널을 그 방향에 생성"하는 단일 커맨드는 없다. `open_tab` → `split` 2회 조합은 (a) `open_tab` 이
  새 탭을 즉시 활성화해 기존 터미널 view 를 unmount(ring buffer 전량 replay) (b) 새 탭이 마운트 즉시 스폰을
  시작한 뒤 split 재마운트로 **두 번째 셸 스폰**(감사 §4-B A6/C14 고아 셸 패턴 재발) (c) `open_tab` 의 kind
  동등 dedupe 로 session_id 미기록 터미널 탭이 있으면 새 탭이 안 생김 — 세 결함을 동시에 만든다.
- 비활성 판정 근거는 최소 pane 크기뿐: `pane-node-view.tsx MIN_PANEL_SIZE_PX = 120`(react-resizable-panels 는
  퍼센트 환산 후 정규화로 뭉개므로 앱이 사전에 걸러야 한다). Left/Right 는 폭, Top/Bottom 은 높이 축.
  pane 크기를 아는 컴포넌트가 없고 `TerminalSession` 은 `paneId` 도 모른다.
- xterm 액션 API 전부 존재: `hasSelection/getSelection/selectAll/clear/paste`. `paste()` 가 bracketed paste 를
  보존하므로 `writePty` 직접 호출보다 옳다. 노출 핸들 `TerminalAttachHandle` 은 `write/jumpToPrevious/Next` 뿐.
- 로케일: `editorArea.splitTop/Bottom/Left/Right`·`tab.split`·`tab.newTerminal` 재사용 가능. 신규 5키.
- 원격 미러: `layout_*`·`pty_*` 는 허용 목록 → 분할·새 터미널·종료 동작. `navigator.clipboard` 는 비보안
  컨텍스트에서 부재 가능.
- cwd 상속: `pty_default_options` 가 `ensure_within_root` 로 cwd 를 검증 → 라이브 cwd 가 루트 밖이면 Forbidden.
- `docs/features/terminal.md` §1·§7·§8 은 clipboard 애드온·`⌘+/-`·검색 바를 기술하나 실제 미구현/미설치.
  메뉴에 검색·이름 바꾸기·폰트 크기는 넣지 않는다(각각 미구현/커맨드 부재/상태바·설정 존재).

### F.2 설계 (채택안 A — 신규 Rust 커맨드 1개, 결정은 전부 조사 추천안)

1. **Rust `layout_open_tab_in_split`**: `service.rs split` 의 "새 Leaf 를 target 위치에 끼우는" 부분
   (`is_root_target` 분기 + `insert_split_at`/`wrap_leaf_in_split` + focused pane 설정 + normalize)을
   `fn insert_new_leaf(layout, target_pane, edge, new_leaf)` 로 추출해 기존 `split`(이동)과 신규
   `open_tab_in_split(project_id, target_pane, edge, kind, title)`(신규 탭을 새 Leaf 에 생성 + 활성)이 공유한다.
   `begin_mutation` 1회·`layout:changed` 1회·기존 터미널 unmount 0 — 이중 스폰이 구조적으로 불가.
   commands.rs 커맨드 + specta 등록 + `REMOTE_ALLOWED_COMMANDS` 등재 + `IMPLEMENTED_JSON_COMMANDS`/dispatch arm +
   Rust 단위 테스트(루트 Leaf·같은 방향 부모·다른 방향 부모·NotFound pane) + `ipc-contract.md`.
   배치 3 A.2-1 의 File kind 선검증은 이 커맨드에도 동일 적용(공유 헬퍼로).
2. **FE `entities/layout`**: `layout.ipc.ts openTabInSplit` + `layout.query.ts useOpenTabInSplit`(`useLayoutMutation`).
3. **순수 판정** `widgets/terminal-pane/terminal-split-availability.ts`(형제 선례 `terminal-flow-control.ts`):
   `resolveSplitAvailability({ paneWidthPx, paneHeightPx, minPaneSizePx, resizerThicknessPx }) => Record<SplitEdge, boolean>`
   — `required = minPaneSizePx * 2 + resizerThicknessPx`, left/right 는 폭, top/bottom 은 높이. `bun:test` 경계값
   (239/240/241)·구분자 0~8·0 크기(측정 실패 → 전부 false 가 아니라 전부 true 로 두지 말고 false) 케이스.
   `MIN_PANEL_SIZE_PX` 는 `pane-node-view.tsx` 에서 `shared/constants/layout.ts` 로 승격해 두 곳이 공유.
4. **측정값 공급**: `PaneNodeView` leaf 루트 div 에 ref 를 달아 `paneId`·`paneElementRef` 를
   `TerminalSession → TerminalPane` 으로 내린다(`autoFocus` 와 같은 자리). 메뉴 `onOpenChange(true)` 시점에
   `getBoundingClientRect()` 1회만 읽는다(ResizeObserver·리렌더 0). 높이는 탭 바(36px, Zen 이면 0)를 뺀다.
5. **`TerminalAttachHandle` 확장**(`terminal-view.tsx`): `focus/hasSelection/getSelection/selectAll/clear/paste`.
   Terminal 옵션에 `rightClickSelectsWord: false` 명시(복사 대상을 사용자 선택으로 고정).
6. **메뉴 컴포넌트** `features/terminal/terminal-context-menu.tsx`(props 전용, `file-tree-context-menu.tsx` 스타일):
   ```
   복사(disabled=!hasSelection) · 붙여넣기(disabled=!canReadClipboard) · 모두 선택
   ── 지우기
   ── 터미널 분할 ▸ 위/아래/왼쪽/오른쪽 (각 disabled=!availability[edge], 라벨 editorArea.split*) · 새 터미널
   ── 터미널 종료
   ```
   `TerminalPane` 이 `<ContextMenuTrigger asChild><div className='h-full w-full'>…</div></ContextMenuTrigger>` 로 감싸고,
   `ContextMenuContent onCloseAutoFocus={(e) => { e.preventDefault(); attachRef.current?.focus() }}` 로 포커스를
   xterm 에 되돌린다. 메뉴 열림 시 `hasSelection` 스냅샷·pane rect 측정·클립보드 가능 여부(`navigator.clipboard`
   존재 + `isRemoteMirrorRuntime` 비보안 컨텍스트 판정을 순수 함수로 분리)를 `useState` 로 잡는다.
   마우스 리포팅 TUI 안에서도 항상 메뉴를 연다(VS Code 동일).
7. **액션 배선**(`terminal-session.tsx` 가 mutation 소유): 복사 `navigator.clipboard.writeText(getSelection())`,
   붙여넣기 `readText()` → `attach.paste(text)`, 모두 선택/지우기 → attach, 분할 → `openTabInSplit({ projectId,
   targetPane: paneId, edge, kind: { kind: 'terminal', sessionId: '', cwd }, title })` — cwd 는 라이브 cwd
   (`cwd ?? persistedSession?.cwd ?? tabCwd`)를 쓰되 프로젝트 루트 밖이면 null(루트) 폴백(순수 판정 함수 +
   테스트, 기존 경로 유틸 재사용), 새 터미널 → 기존 `openTab({ kind: terminal, target: paneId })` 경로 재사용,
   종료 → 기존 `closeTab` (확인 다이얼로그 없음 — 기존 탭 닫기와 동일).
8. **로케일** 신규 `terminal.copy`·`terminal.paste`·`terminal.selectAll`·`terminal.clear`·`terminal.kill`
   (ko "복사"/"붙여넣기"/"모두 선택"/"지우기"/"터미널 종료", en "Copy"/"Paste"/"Select All"/"Clear"/"Kill Terminal",
   ja "コピー"/"貼り付け"/"すべて選択"/"クリア"/"ターミナルを終了") + `MESSAGE_NAMESPACES` terminal 배열.
9. **문서**: `docs/features/terminal.md` 에 §6.2 "컨텍스트 메뉴" 절 + §1·§7·§8 의 현행 불일치(clipboard 애드온
   미설치·`⌘+/-` 미구현·검색 바 미구현) 정정 + 백로그 등재(검색 UI·탭 이름 바꾸기·종료 확인 다이얼로그·
   "이 터미널 옮기기"), `docs/ipc-contract.md`, PROCESS 기준선(커맨드 수·로케일 키 수) 갱신.

## G. 탭 바 우클릭 컨텍스트 메뉴 (여백 메뉴 신설 + 기존 탭 메뉴에 "이름 바꾸기")

### G.1 현행 사실 (조사 완료 — 원문 위 research 문서 주제 7)

- `pane-tab-bar.tsx`: 탭 목록(dnd-kit) + 여백 filler div(`min-w-8 flex-1`, 더블클릭 = 새 untitled) + 우측 `+`
  액션 영역(스크롤 컨테이너 밖, `tab-bar-add-menu.tsx` 드롭다운: 새 파일·새 터미널). 여백에 컨텍스트 메뉴 없음.
- Radix `ContextMenuTrigger` 는 `stopPropagation` 을 하지 않는다 — 탭 바 전체를 감싸면 탭 위 우클릭에 탭 메뉴와
  여백 메뉴가 동시에 열린다. 따라서 여백(filler)과 `+` 영역에만 Trigger 를 둔다.
- 핸들러 전부 기존: `openUntitledTab`·`openTab(terminal)`·`handleCloseSaved`·`handleCloseAll`·`splitPane`·
  `useReopenClosedTab`. 신규 IPC 0. 탭 0 pane 에서 분할은 `layout_split` 이 tabId 를 요구해 NotFound → 숨김 필수.
  untitled 탭은 `closed_tabs` 에 쌓이지 않는다(재열기 불가 — 기존 동작).
- 탭 이름 바꾸기 경로 전무. 탐색기 인라인 rename(`use-explorer-entry-crud.ts startRename`)이 있고, 브리지 선례
  `explorer-reveal-bridge.ts`(`createFireAndForgetBridge`) 로 탐색기에 요청을 보낼 수 있다.
- 로케일: `keymap.reopenClosedTab`·`editorArea.split*`·`tab.closeSaved`·`tab.closeAll`·`tab.newTerminal` 재사용.
  testing-library 미도입이라 메뉴 컴포넌트 직접 테스트 불가 → 표시 조건은 순수 빌더로 테스트.

### G.2 설계 (A안 골격 + C안 순수 빌더 + 탭 메뉴 "이름 바꾸기")

1. `features/tab/tab-bar-menu-items.ts`: `buildTabBarMenuItems({ hasTabs, hasActiveTab, hasClosedTabs })` 순수 빌더
   (항목 id·라벨 키·표시 여부·분할 서브항목) + `bun:test`(탭 0 → 분할 숨김, closedTabs 0 → 재열기 숨김, 탭 0 →
   닫기류 숨김). `tab-bar-add-menu.tsx`(`+` 드롭다운)도 같은 빌더를 소비하되 `+` 는 현행 2항목(새 파일·새 터미널)만
   노출하도록 필터 인자를 둔다(파괴적 항목은 `+` 에 노출하지 않음).
2. `features/tab/tab-bar-context-menu.tsx`(props 전용 ContextMenu): 새 파일(untitled 버퍼 — 현행 `+`·더블클릭과
   동일) · 새 터미널 · 닫은 탭 다시 열기 · 저장된 탭 닫기 · 모든 탭 닫기 · 분할 ▸ 위/아래/왼쪽/오른쪽(활성 탭
   이동 — 탭 메뉴 Split 과 동일 의미) · Welcome 열기(B 절 커맨드 — `requestShowWelcome` 류 진입점을 B 절이 정하면
   빌더에 1줄 추가; B 절 구현 전이면 항목 자리만 비움). 새 탭은 우클릭한 pane(`target: paneId`)에 연다.
3. `pane-tab-bar.tsx`: filler div 와 우측 `+` 액션 영역 **둘 다** `ContextMenuTrigger asChild` 로 같은 메뉴 래핑
   (asChild 필수 — Slot 이 dnd droppable ref 와 합성), `useReopenClosedTab` 1줄 추가.
4. 기존 탭 메뉴 "이름 바꾸기"(file 탭 한정): `shared/lib/bridge/explorer-rename-bridge.ts` 신설(reveal 브리지와
   동일 패턴 + 테스트) → `tab-context-menu.tsx` 항목 → `pane-tab-bar.tsx` 에서 `requestRenameInExplorer(path)` →
   `explorer-panel.tsx` 구독(`onViewChange('files')` + reveal 후 `startRename(row)`) — 배치 3 C 절이 만든
   `onRevealInExplorerRequest`/autoReveal 인프라 재사용. 사이드바가 접혀 있으면 배치 3 C.2-6 과 같이 펼친다.
   로케일 `tab.rename`(ko "이름 바꾸기" / en "Rename" / ja "名前を変更") + `tab` 네임스페이스.
5. 문서: `docs/features/tabs.md` §3.2 "탭 바 여백 메뉴"·§3.1 "이름 바꾸기" 추가. 단축키 힌트(ContextMenuShortcut)는
   미도입.

### G.3 기각·보류

- 탭 바 전체 단일 ContextMenu + 히트테스트(B안): 이중 메뉴 위험·4파일 구조 변경 → 기각.
- 기존 탭 메뉴에 저장/되돌리기·다른 이름으로 저장·Copy into New Window: 브리지 계약 확장 필요/백로그 확정 항목 → 보류.

## 3. 구현 기록 (웨이브 1)

> 2026-09-04, 구현 wf `wf_c7cc982b`. **A·B·D·E·F·G 6절**을 Rust 4 · TS 5 에이전트 + 통합 검증 1로
> 나눠 구현했다(C 성능·H 테스트·CI 는 웨이브 2). 아래는 통합 단계가 실물 대조로 확인한 결과다.
> 계약 각 절의 설계 문언과 실제 코드가 다른 곳은 "이탈" 에 사유와 함께 적었다.

### 3.1 절별 반영 결과 (실물 대조)

| 절 | 반영 | 실물 확인 지점 |
|----|------|----------------|
| **A.2-1** 의존성 | O | `src-tauri/Cargo.toml` `tauri-plugin-notification = "2"`·`rust-version = "1.89"`, `lib.rs:478` 플러그인 init, `capabilities/main.json` `notification:allow-is-permission-granted` **1개만**. Cargo.lock 순증 4(예측 3 + Windows cfg 게이트 `tauri-winrt-notification`) |
| **A.2-2** notification 도메인 | O | `domain/notification/{mod,types,service,commands}.rs`, `NotificationCategory` 6종, `decide_delivery` 순수 함수 + 조합 테스트, `notification_notify`·`notification_open_system_settings` 2종 `REMOTE_DENIED`(`UnreachableDesktopWindow`) |
| **A.2-3** 설정 8필드 | O | `settings/types.rs:202-238`(Settings)·`:393-400`(Patch)·`:508-515`(Default), `apply_patch`·`settings_to_sync_patch`·`emptySettingsPatch` 전부 |
| **A.2-4** FE 파사드 | O | `entities/notification/{notification.ipc.ts,notify.ts}`, 순수 판정 `shared/lib/native-notification-gate.ts` + 테스트 18케이스 |
| **A.2-5** 발생지 배선 | O | 에이전트·LSP = `app/providers/native-notification-provider.tsx`, 태스크 = `terminal-osc133.ts onCommandFinished` → `terminal-view` → `terminal-pane` → `terminal-session.tsx`, git = `git.query.ts:213/218`, 검색 = `search-panel-container.tsx:96` |
| **A.2-6** 설정 UI | O | `settings-notification-section.tsx` SwitchField **8** + 시스템 설정 버튼 + 테스트 알림 버튼 + `import.meta.env.DEV` 힌트, 첫 전달 1회 안내는 `useRef` + sonner `action` |
| **A.2-7·8** 로케일·문서 | O | 로케일 순증 48키(3언어 각 1016), `architecture.md`(도메인 25)·`tech-stack.md`·`data-model.md` §19·`deployment.md` §8·`ipc-contract.md`·`settings-ui.md` §2.1 |
| **B.2-1** `view.welcome` | O | `command-catalog.ts:93`, `command-registry.ts:10`, `command-palette.tsx:174`, `shared/constants/tab.ts WELCOME_TAB_TITLE` |
| **B.2-2** 탭 0 자동 표시 | O | `pane-node-view.tsx:127` `getWindowContext().kind === 'main' && (settings?.welcomeOnEmptyEditor ?? true)` → `:229` `WelcomeContainerLazy`, off·보조 창은 `editor.noFileOpen` 유지 |
| **B.2-3·4** 설정·문서 | O | `settings-interface-section.tsx:95-98`, `command-palette.md`·`tabs.md`·`layout-shell.md` §1.1·`PRD.md` FR-B2 |
| **D.2-1** Rust | O | `ProjectDisplay`/`ProjectDisplayPatch`, `Project.display`·`ProjectRef.display`(`#[serde(default)]`), `set_project_display` sanitize 3축 + `lane1..lane12` 허용목록, `project_set_display`(`REMOTE_ALLOWED`), 신규 이벤트 0 |
| **D.2-2** FE | O | `project-icon-registry.ts`(**56종**)·`project-display.ts`(단일 폴백)·`project-display.test.ts` 20케이스·`project-display-dialog.tsx`·`sortable-project-icon.tsx:97-99` 메뉴 2항목 |
| **D.2-3·4** 로케일·문서 | O | `project.display*` 11 + `error.project.displayInvalid`, `layout-shell.md` §2.2·§2.3.1·`data-model.md` §20·`ipc-contract.md`·`PRD.md` FR-A4 각주 |
| **E.2-1~4** git 섹션 | O | 순서 Merge→Staged→Changes→Stashes→Graph, `git-sections.ts buildGitSections`, `features/git/git-section-header.tsx`(`resource-group-header.tsx` 삭제), `entities/git/git-section-collapse-memory.ts`, `GIT_SECTION_ROVING_SELECTOR` |
| **E.2-5** 문서 | O | `features/git.md`·`architecture.md` §6.4 + 손 QA `quality-assurance/2026-09-04-git-section-ux-hand-qa.md` |
| **F.2-1** `layout_open_tab_in_split` | O | `layout/service.rs` `insert_new_leaf`(추출)·`open_tab_in_split`, `commands.rs:169`, specta·dispatch 3표·테스트 7건 |
| **F.2-2~7** FE | O | `entities/layout` 2줄, `terminal-split-availability.ts`·`terminal-clipboard-availability.ts`(+테스트), `features/terminal/terminal-context-menu.tsx`, `TerminalAttachHandle` 6메서드 + `rightClickSelectsWord: false`, `MIN_PANEL_SIZE_PX` → `shared/constants/layout.ts` 승격 |
| **F.2-8·9** 로케일·문서 | O | `terminal.{copy,paste,selectAll,clear,kill}`, `terminal.md` §6.2 + §1·§7·§8·§11 정정, `backlog.md` 이월 5건 |
| **G.2-1~3** 여백 메뉴 | O | `tab-bar-menu-items.ts` 순수 빌더 + 테스트 8건, `tab-bar-context-menu.tsx`, `pane-tab-bar.tsx` filler·`+` 양쪽 Trigger, `+` 드롭다운도 같은 빌더(`surface: 'addMenu'`) |
| **G.2-4·5** 이름 바꾸기·문서 | O | `shared/lib/bridge/explorer-rename-bridge.ts`(+테스트 3건) → `app-shell`·`explorer-panel`·`explorer-container`, `tab-context-menu.tsx:139`, `tabs.md` §3.1·§3.2 |

### 3.2 통합 단계가 직접 보완한 것

계약 항목이지만 어느 에이전트의 소유 파일도 아니어서 비어 있던 4건을 통합에서 채웠다.

1. `docs/ipc-contract.md` 최상단 **실측 배너** — 배치 4 웨이브 1 행 신설(command 179→**183**, raw 포함
   182→**186**, `REMOTE_ALLOWED` 158→**160**, `REMOTE_DENIED` 24→**26**, event 23 불변). 각 에이전트가
   자기 절에만 증분을 적어 헤더가 4개 배치만큼 뒤처져 있었다. (본문 §원격 dispatch 정책의 서술형
   개수 "157종/24종" 은 d-42·d-50 때도 갱신하지 않은 이력 서술이라 선례대로 두었다 — 정본은 배너다.)
2. `docs/backlog.md` — **A.2-8 이 지정한 알림 이월 3건**(권한 상태 조회·`UNUserNotificationCenter`
   전환·알림 클릭 딥링크)과 **D.2-4 가 지정한 프로젝트 표시 이월 3건**(Welcome 최근 목록·보조창
   타이틀바 확장 시 `ProjectDisplayChanged`, PRD FR-A4 슬롯 충돌, 타이포·색 대비 실측) 절 신설.
   D 담당은 이 내용을 `layout-shell.md` 본문에만 적어 두었었다.
3. `docs/features/settings-ui.md` §1 — Interface 섹션 설명에 `welcomeOnEmptyEditor` 스위치 1항목
   추가(`explorerAutoReveal` 옆). Rust·FE 양쪽이 서로 상대 소유로 보고 비워 둔 자리다.
4. `src/widgets/welcome/welcome-container-lazy.ts` JSDoc — 렌더 지점이 2곳에서 **3곳**(빈 편집 영역
   추가)이 되어 서술을 정정.

### 3.3 이탈 (계약과 다르게 구현한 것 — 중복 제거)

| # | 이탈 | 사유 |
|---|------|------|
| 1 | `layout_open_tab_in_split` 인자를 위치 8개 → 구조체 1개(`OpenTabInSplitRequest`) | clippy `too_many_arguments`(상한 7)에 걸려 `-D warnings` 빌드 실패. `#[allow]` 은 컨벤션 금지라 같은 린트의 기존 선례(`LspSpawnRequest`·`PtySpawnOptions`)를 따랐다. 필드 이름·형태는 계약과 동일 |
| 2 | `NotificationDecision`/`NotificationDelivery` 를 타입 1개로 합침 | 두 지점 사이에 관측 가능한 상태 변화가 없어 형태가 완전히 같다. 동형 타입 2개는 "타입은 원본에서 유도" 에 어긋남 |
| 3 | `notification_open_system_settings` 에서 `AppHandle` 인자 제거 | `tauri_plugin_opener::open_url` 은 자유 함수이고(기존 `system_open_external_url` 도 안 받는다), 비-macOS 분기에서 미사용 인자가 되어 clippy 실패 |
| 4 | 알림 FE 파사드가 `focusManager.isFocused()` 조기 컷을 하지 않음 | `onlyWhenUnfocused` 설정을 모른 채 포커스만으로 컷하면 그 설정을 끈 사용자의 알림을 삼킨다. 설정 캐시를 읽으려면 entities → app 역참조(FSD 위반·eslint 규칙 위반). 계약이 허용한 "컷을 Rust 에 위임" 채택 |
| 5 | `set_project_display` 시그니처가 `(state, id, patch)` 가 아니라 `(paths, session, projects, id, patch)` | 같은 파일의 `open_project`/`close_project` 가 전부 개별 조각을 받는다. `AppState` 를 받으면 서비스 단위 테스트가 불가능 |
| 6 | `merge_clearable_string` 재사용 대신 project 도메인에 `merge_display_axis` 신설 | 원본이 private 이고 `settings/service.rs` 는 다른 에이전트 소유였다. 해제 규약("생략=유지/빈문자열=해제")은 동일하게 따랐고 출처를 JSDoc·문서에 명시 |
| 7 | `buildGitSections` 인자가 `{rows, stashes, conflicts, collapsed}` → `{rows, stashCount, graphCount, collapsed}` | conflicts 는 `rows[].isConflicted` 에서 유도되므로 따로 받으면 이중 출처, stash·graph 는 개수만 필요 |
| 8 | 터미널 pane rect 를 leaf 루트가 아니라 **콘텐츠 박스**에서 측정 | 계약의 "높이에서 탭 바 36px(Zen 0) 제외" 가 매직넘버(컨벤션 금지) + zen 플래그 배선을 요구한다. 콘텐츠 박스는 같은 수치를 배선 없이 준다 |
| 9 | 클립보드 판정 입력에서 `isSecureContext` 제외 | async clipboard API 는 secure context 에만 존재해 `writeText`/`readText` 존재 확인에 완전히 포함된다. AND 로 두면 Tauri 커스텀 스킴에서 상시 비활성이 되는 오검출만 남는다 |
| 10 | `tab.newFile` 신규 키 미추가 → 기존 `tab.newUntitledFile` 재사용 | 실물 확인 결과 같은 의미의 키가 3언어에 이미 있었다 |
| 11 | 여백 메뉴 빌더가 분할 4방향 표를 소유하지 않고 `SPLIT_EDGE_OPTIONS` 재사용(export 1단어 추가) | 표를 새로 만들면 `tab-context-menu.tsx` 와 통째로 중복. 빌더는 split 항목의 "표시 여부" 만 소유 |
| 12 | 아이콘 그리드 빈 결과에 기존 `palette.noResults` 재사용, 색 스와치는 재클릭 해제(전용 "색 없음" 항목 없음) | 신규 로케일 키를 늘리지 않기 위해. `toast-position-picker` 의 `aria-pressed` 토글 선례 |
| 13 | 소유 파일 밖 다수를 1줄씩 수정 | `Project` 에 필드가 늘어 `#[cfg(test)]` 스텁 9파일이 컴파일 불가(`display: Default::default()` 1줄씩), `CommandContext` 확장으로 `*.commands.test.ts` 3곳 dummyContext 파손(`openWelcomeTab: () => {}` 1줄씩), `TerminalView → TerminalSession` 사이의 실제 경로인 `terminal-pane.tsx` 콜백 통과 4곳, `commit-detail-panel.tsx` 가 삭제 대상 `ResourceGroupHeader` 의 마지막 소비처. 전부 해당 변경이 직접 유발한 파손 |
| 14 | 계약에 없는 `open_project` 수정 1건 | 신규 분기가 이력에서 id 만 재사용하고 `Project` 를 새로 만들어, 프로젝트를 닫았다 열면 표시 설정이 매번 초기화됐다. `find_existing_project_id` → `find_existing_project_record` + 회귀 테스트 |
| 15 | 계약에 없는 파일 2개 신설(`features/project/project-display-glyph.tsx`, `features/tab/tab-bar-menu-items.ts` 의 `group` 필드) | 전자는 사이드바 버튼과 다이얼로그 미리보기가 같은 폴백 사다리를 그려야 해서(2회 이상 룰), 후자는 항목이 필터로 사라져도 구분자가 어긋나지 않게 하기 위해 |
| 16 | Graph 섹션 커밋 행에 `pl-4` 미적용 | 가상화 절대배치 폭 계산에 변수를 더한다. 리소스 행(변경 그룹·스태시)에는 적용 |
| 17 | `docs/PROCESS.md` 기준선(커맨드 수·로케일 키 수) 미갱신 | 오케스트레이터가 PROCESS 를 건드리지 말라고 지시. 실측치는 이 절 §3.4 에 남긴다 |

### 3.4 미결 (사용자·후속 결정)

1. ~~**`git.stashEmpty` 키 처리**~~ — **해소(§3.6 L4-2)**: 3언어 + `MESSAGE_NAMESPACES` 에서 제거했다.
2. ~~**보조 에디터 창 터미널의 태스크 완료 알림 부재**~~ — **해소(§3.6 F-1)**: 발화가 Rust 이벤트
   (`terminal:command-finished`)로 올라가면서 메인 창이 모든 창의 터미널을 대신 알린다.
3. **알림 본문(body) 로케일** — `notification.*` 10종은 보간 없는 **제목**만이고 본문은 FE 가
   비번역 데이터(에이전트명·명령·브랜치·치환 건수·서버 id)로 조립한다. 본문에도 번역 문장이
   필요하면(예: "exit {{code}} 로 종료") 키를 더 만들어야 한다.
4. **테스트 알림 결과 토스트 문구** — 억제 사유를 그 스위치의 라벨 키로 대신 보여준다. 전용 키
   3종(`test.sent`·억제 사유 문장)을 추가하면 명확해진다.
5. **접힘 메모리 범위** — 창(프로세스) 단위 1벌이라 프로젝트 A 에서 접은 섹션이 B 에도 적용된다.
   프로젝트별 분리를 원하면 `commit-message-memory.ts` 처럼 ProjectId 키 + 상한이 필요하다.
6. **탭 바 여백 메뉴의 키보드 접근** — filler·`+` 영역에 `tabIndex` 가 없어 Shift+F10 으로 열 수
   없다. 주면 Tab 순회에 빈 항목이 끼는 트레이드오프.
7. **여백 메뉴 "Welcome 열기" 의 pane 지정** — **정정(§3.6 G-1)**: `open_tab` 의 kind dedupe 는
   `find_leaf_mut` 로 찾은 **그 leaf 자신의 `tabs` 만** 훑는다(`layout/service.rs:428-444` —
   `command-palette.md` 의 "`open_tab` 은 같은 leaf 에 동일 kind 탭이 있으면 재사용" 서술과 일치).
   여백 메뉴는 항상 `target: paneId`(우클릭한 pane)를
   넘기므로(`pane-tab-bar.tsx:167`) 다른 pane 의 Welcome 탭이 활성화되는 일은 없고, 대신 **pane 마다
   별도의 Welcome 탭이 생길 수 있다**. 전역 dedupe 가 필요한지는 §3.4-9 실기 확인 후 결정한다.
8. **워크스페이스 `resolver = "2"`** — MSRV 비인지 resolver 라 상향한 1.89 가 의존 해석에 반영되지
   않는다. resolver 3 전환은 전 의존 해석에 영향을 주므로 별도 배치 권장.
9. **실기 확인 대기(앱 실행 필요)** — 마지막 탭 닫힘 시 Welcome 즉시 표시 / 보조 창 터미널
   우클릭 시 네이티브 메뉴 억제 / 유휴 터미널 연속 우클릭 시 xterm 히든 textarea 로 인한
   `app-shell` 예외 분기 부작용 / 원격 미러에서 복사·붙여넣기 비활성 표시 / 릴리스 빌드
   `otool -L` 자립성 / CJK 4자 라벨 클립·lane 색 대비 36테마.
10. **기존 문서 결함(범위 밖·보고만)** — `docs/features/tabs.md` 에 리터럴 `\n## 4. DND` ·
    `\n## 5. 스플릿 리사이즈` 2곳이 있어 두 헤딩이 렌더되지 않는다. HEAD 에도 동일하게 존재한다.

### 3.5 검증 실측 (통합 단계, 전 항목 그린)

| 명령 | 결과 |
|------|------|
| `bun run typecheck` | 통과 — 오류 0 |
| `bun run lint` | 통과 — **0 errors / 9 warnings**(9건 전부 기존 파일: TanStack Virtual `incompatible-library` 4·`exhaustive-deps` 5) |
| `bun run format:check` | 통과 — "All matched files use Prettier code style!" |
| `bun test` | 통과 — **1960 pass / 0 fail / 3268 expect, 201 파일**(조사 시점 기준선 1817) |
| `cargo fmt --all --check` | 통과 |
| `cargo clippy --workspace --all-targets -- -D warnings` | 통과 — 경고 0 |
| `cargo test --workspace` | 통과 — lib **1267 pass / 0 fail**(기준선 1248), 통합 3·6·17 pass, doc-test 0 |
| `bunx vite build` | 통과 — 6.99s, 청크 경고는 기존(monaco·pdf) |
| `bun run typecheck:e2e` | 통과 — 오류 0 |

**표면 실측**: `IMPLEMENTED_JSON_COMMANDS` **183** = `bindings.ts` 의 `TAURI_INVOKE` 전수(raw 3종
제외), `REMOTE_ALLOWED_COMMANDS` **160** ⊎ `REMOTE_DENIED_COMMANDS` **26** 완전분할, raw 3종 포함
총 **186**. event **23종 불변**. 로케일 `{ko,en,ja}.json` 각 **1016키**(순증 48, 3언어 키집합 일치 ·
`MESSAGE_NAMESPACES` 등재 완료 — `cargo test` 가 강제). 신규 TS 로케일 참조 173키 전수가 3언어에
존재함을 별도 대조로 확인했고, 미참조 신규 키는 Rust 전용 2종(`error.notification.settingsUnsupported`·
`error.project.displayInvalid`)뿐이다.

**금지 패턴 전수 확인**(변경된 `src/**` 68파일, 생성물 `bindings.ts` 제외): `useCallback`/`useMemo` 0 ·
`any` 0 · `enum` 0 · `function` 키워드 0 · 코드 주석(`//`) 0 · `@ts-ignore`/`@ts-expect-error`/
`eslint-disable` 0. Rust 신규 코드의 `#[allow]` 0, 비테스트 `unwrap()` 0.

> §3.6 이 이 표의 두 수치를 바꿨다: **event 23 → 24**, **로케일 1016 → 1015키×3**.

### 3.6 리뷰 수정 (웨이브 1 코드 리뷰 반영)

웨이브 1 diff 리뷰에서 확증된 major 1건을 전건 수정하고, minor 4건 중 근본 수정이 작고 회귀 위험이
낮은 4건을 함께 처리했다. info 3건은 수정하지 않았다.

| id | 등급 | 처리 | 사유·내용 |
|----|------|------|-----------|
| **F-1** 배경 터미널의 태스크 완료 알림 소실 | major | **수정** | 아래 상세 |
| **A-1** `workingSinceByProjectRef` 무한 누적 | minor | 수정 | `native-notification-provider.tsx` 가 `project:closed` 를 구독해 그 projectId 키를 제거한다. 닫힌 프로젝트는 더 이상 `agent:state-changed` 를 내지 않으므로 그 항목을 지울 주체가 달리 없었다 |
| **notif-body-unbounded-length** 알림 본문 길이 무제한 | minor | 수정 | `notifyNative` 진입점에서 제목·본문을 `NOTIFICATION_TEXT_MAX_CODE_POINTS`(200)로 자른다. 순수 함수 `truncateNotificationText`(코드포인트 단위 — 서로게이트 반쪽이 `NSString` 에 닿지 않게) + 테스트 3건 |
| **L4-1** 카운트 뱃지 마크업 중복 | minor | 수정 | `features/git/git-section-count-badge.tsx` 신설 → `git-section-header.tsx`·`commit-detail-panel.tsx` 가 공유. 후자는 chevron·토글·sticky·로빙 정지점이 없는 순수 캡션이라 `GitSectionHeader` 자체를 재사용할 수 없어, 실제로 중복이던 뱃지만 뽑았다 |
| **L4-2** `git.stashEmpty` 미참조 키 | minor | 수정 | 3언어 로케일 + `locale/service.rs` 의 `MESSAGE_NAMESPACES` 에서 제거(§3.4-1 해소). 로케일 **1015키×3**, `cargo test` 통과 |
| **G-1** 계약 §3.4-7 서술 오류 | minor | 수정(문서) | `open_tab` 의 dedupe 범위를 실물 확인해 §3.4-7 을 정정했다. 코드 변경 없음 |
| E-1 스태시 행이 로빙 시퀀스에 없음 | info | 미수정 | 참고만 |
| project-label-format-chars-not-filtered | info | 미수정 | 참고만 |
| L4-3 `sortable-project-icon.tsx` 훅 순서 | info | 미수정 | 참고만 |

**F-1 상세 — 태스크 완료 감지를 pty reader 스레드로 올렸다.**

- **증상**: `pane-node-view.tsx` 는 pane 의 활성 탭만 렌더하므로, 터미널 탭에서 빌드를 걸어두고 다른
  탭으로 옮기면 `TerminalSession` 이 언마운트되고 xterm 인스턴스와 `attachOsc133BlockTracker` 가
  통째로 dispose 된다. 그 사이 셸이 내보내는 OSC 133 `C`/`D` 를 볼 주체가 없어 알림이 발화하지
  않고, 나중에 그 탭으로 돌아가면 `pty_attach` 의 스크롤백 재생이 같은 `C`/`D` 를 수 ms 간격으로 다시
  파싱해 실행 시간이 0 에 수렴 → 10초 임계에 걸려 그때도 억제된다. "빌드 걸어두고 자리를 뜬다" 는
  이 기능의 핵심 시나리오가 통째로 죽어 있었다.
- **수정**: 계약 A.2-5 가 지정한 FE 배선(`terminal-osc133.ts onCommandFinished` → `terminal-view` →
  `terminal-pane` → `terminal-session.tsx`)을 **철거**하고, 감지·계측을 pty reader 스레드로 옮겼다.
  reader 콜백은 이미 `extract_latest_cwd` 로 OSC 7 을 스캔하던 자리이고, 부착 여부와 무관하게 항상
  살아 있는 유일한 지점이다.
  - `infra/shell_integration.rs`: `CommandMarker { OutputStart, Finished { exit_code } }` +
    `extract_command_markers(bytes)`(기존 `earliest_terminator` 재사용, 청크당 전 마커를 순서대로
    반환 — 한 청크에 `C`…`D` 가 함께 실릴 수 있어 "마지막 하나만" 이면 전이를 잃는다). 테스트 5건.
  - `domain/terminal/commands.rs`: `report_command_marker` 가 세션별 `Mutex<Option<Instant>>` 를
    `C` 에서 채우고 `D` 에서 소비해 `TerminalCommandFinished { sessionId, cwd, exitCode, durationMs }`
    를 발행한다. `C` 를 못 본 `D` 는 발행하지 않는다(측정 불가 ≠ 즉시).
  - `events.rs`·`lib.rs`: 이벤트 1종 추가(`collect_events!` + `fanout_remote_events!`),
    `이벤트_타입_목록은…` 테스트 기준선 23 → **24**. `bindings.ts` 는 `cargo test` 의 export 로 재생성.
  - FE: `native-notification-provider.tsx` 가 `terminal:command-finished` 를 구독해
    `shouldNotifyTaskCompletion` 통과 시 알린다(본문 = 세션 cwd). 이제 측정된 명령만 도달하므로
    `shouldNotifyTaskCompletion` 의 `durationMs` 는 `number | null` → `number` 로 좁혔고,
    `resolveCommandDurationMs`·`TerminalCommandFinished`(TS) 와 4파일에 걸친 콜백 배선은 삭제했다.
- **부수 효과**: 발화가 창과 무관한 백엔드 이벤트가 되면서 §3.4-2(보조 창 터미널 무알림)도 해소됐다.
  본문이 탭 제목 → 세션 cwd 로 바뀌었다(탭 제목은 사실상 상시 "터미널" 이라 정보량이 늘었다).
- **남는 한계**: 청크 경계에 걸려 잘린 `C`/`D` 마커는 이번 청크에서 감지되지 않는다(`extract_latest_cwd`
  와 같은 의도적 한계). cwd 와 달리 그 명령의 알림은 회복되지 않고 그냥 뜨지 않는다.

**재검증**(전 항목 그린, `bun run verify` exit 0): typecheck 0 · lint **0 errors / 9 warnings**(기존
9건 그대로) · format:check 통과 · `bun test` **1958 pass / 0 fail**(1960 에서 삭제된 배선 테스트 8건 −,
신규 6건 +) · `cargo fmt` · `cargo clippy -D warnings` 경고 0 · `cargo test` **lib 1272 pass**(1267 +
`extract_command_markers` 5건) · `bunx vite build` exit 0.

### 3.7 테스트 단계

> 2026-09-04, §3.6 직후. 단위 보강 1 · e2e 스펙 작성 1 · 최종 검증 1 로 나눠 진행했다. 웨이브 1 이
> 만들거나 바꾼 모듈의 **커버리지 공백만** 채우고, 기존 케이스와 겹치는 시나리오는 작성하지 않았다.
> 프로덕션 코드는 건드리지 않았다(테스트·e2e·문서만).

**단위 테스트 — `bun test` 1958 → 2008(순증 50) / `cargo test` lib 1272 → 1288(순증 16)**

TS 는 기존 테스트 파일 9개를 보강하고 2개를 신설했다. 괄호는 실측 총 케이스 수다.

| 파일 | 케이스 | 보강 요지 |
|------|:-----:|-----------|
| `src/shared/lib/native-notification-gate.test.ts` | 28 | 3스냅샷 연속 최초 시각 유지, 한 에이전트 완료 시 타 에이전트 기록 보존, 완료 후 재작업 새 시각, 빈 로스터, duration·임계 0, `truncate` 상한 경계(서로게이트)·상한 1·빈 문자열 |
| `src/shared/lib/project-display.test.ts` | 27 | 3축 동시 입력 시 label 모드 배타(icon 버림·색 유지), 공백 라벨 → icon 모드, 아이콘 trim, 색 토큰 대소문자·CSS 변수 전체명 거부, 이모지 4개 타이포 단계 |
| `src/widgets/terminal-pane/terminal-split-availability.test.ts` | 23 | 음수 측정, 두꺼운 구분자 축별 반영, 최소 0·구분자 0, cwd 후보 캐스케이드 없음·접두사 공유 형제 디렉터리·루트 끝 슬래시 |
| `src/widgets/git-panel/git-sections.test.ts` | 16 | 5섹션 전부 비가시 + `showNoChanges`, `sections` 키 순서 = 렌더 순서, 같은 그룹 다건 순서·집계, 접힘 5축, 접힘이 가시성에 무영향 |
| `src/features/tab/tab-bar-menu-items.test.ts` | 14 | `hasTabs` 만 참인 정확 목록, `group` 연속성·필터 후 그룹 순서 유지, 아이콘 부재 항목, `+` 드롭다운이 조건 인자를 무시하고 파괴적 항목 미노출 |
| `src/entities/git/git-section-collapse-memory.test.ts` | 9 | 재기록 last-wins, 스냅샷 복사본(변형 미반영)·호출마다 새 객체, reset 후 기본값 복귀 |
| `src/shared/lib/command-catalog.test.ts` | 7 | `view.welcome` 이 `openWelcomeTab` 호출·`app.welcome`·기본 단축키 없음, `view.git` 바로 뒤 위치 |
| `src/shared/lib/bridge/explorer-rename-bridge.test.ts` | 5 | 구독자 없을 때 요청 폐기(재생 없음), 요청마다 1회 호출 |
| `src/widgets/terminal-pane/terminal-clipboard-availability.test.ts` | 5 | 런타임 프로브가 bun(clipboard 부재)에서 예외 없이 둘 다 false |
| **신규** `src/entities/notification/notify.test.ts` | 5 | 파사드가 게이트 결과 반환·IPC 인자 전달, 제목·본문 200 코드포인트 절단, 상한 이하 무변경, `delivered` 일 때만 첫 전달 안내 구독자 호출, IPC 실패 시 null |
| **신규** `src/entities/project/project.query.test.ts` | 2 | `useSetProjectDisplay` 성공 시 `PROJECT.LIST` 만 무효화(RECENT·ACTIVE 무효화 없음), projectId·patch IPC 전달 |

Rust 는 3파일 순증 16건: `infra/shell_integration.rs` 5(한 청크 2명령 C/D/C/D 순서 보존, OSC 7 과
혼재해도 133 만 읽고 cwd 정상, A/B 만이면 빈 벡터, 음수·공백 종료 상태 파싱, 마커 없는 출력) ·
`domain/project/service.rs` 7(아이콘 64바이트 경계, 라벨 길이 코드포인트 기준, 제어문자만인 라벨 =
해제, 3축 동시 해제, 미열림 프로젝트 NotFound + 무기록, 해제 패치 양쪽 파일 미러, root/name 미러 불변) ·
`domain/layout/service.rs` 4(Left/Top 은 새 페인이 앞·Right/Bottom 은 뒤, 중첩 대상에서도 새 페인 포커스 ·
revision +1, 기존 페인 활성 탭 불변 = 터미널 언마운트 방지 계약).

**e2e 스펙 17~20 (전부 미실행)**

| 스펙 | 대상 절 | 검증 |
|------|--------|------|
| `e2e/specs/17-welcome-command-and-empty-pane.e2e.ts` | B | 탭 0 상태의 빈 편집 영역 Welcome → 팔레트 `View: Welcome` → Welcome 탭 활성 + `layout_get` welcome kind 0 → 1 |
| `e2e/specs/18-tab-bar-context-menu.e2e.ts` | G | 여백 메뉴 `New Terminal` → terminal 탭 +1, 탭 0 상태에서 `Split`·`Close All Tabs` 숨김(탭 있을 때 표시로 양성 대조) |
| `e2e/specs/19-git-sections-collapse.e2e.ts` | E | `Changes` 헤더 `role=button`·`position: sticky`, 클릭으로 `aria-expanded` true→false→true 및 행 표시/0건/표시, Stash 섹션 헤더 0건 |
| `e2e/specs/20-terminal-context-menu-split.e2e.ts` | F | `.xterm` 우클릭 → `Split` ▸ `Split Right` → `.xterm` 2개 + `layout_get` root split/horizontal, leaf 2, terminal 탭 +1 |

공용 헬퍼 2개 신설(2회 이상 사용): `e2e/lib/layout.ts`(`layout_get` 오라클 — leaf·탭 수집, kind 별
카운트) · `e2e/lib/tab-bar.ts`(여백 filler 로케이터 + 여백 메뉴 열기). 하네스 문서
`docs/quality-assurance/2026-08-18-e2e-harness.md` 는 스펙 16종 → **20종**, §5 표 17~20 행 + 미실행·
전제 주석, §8 디렉토리 구조를 갱신했다.

> **17~20 은 작성 시점에 미실행이다.** 하네스가 앱을 기동하지 않으므로(스펙 14~16 과 같은 이유)
> 사용자가 `bun run tauri dev` + REMOTE 준비 후 `bun run e2e` 를 직접 돌려야 한다.

**e2e 미작성 → 수동 QA 로 남긴 절**: §A 알림(`notification_notify` 가 `REMOTE_DENIED` 이고 앱 전체
포커스 게이트를 Playwright 가 제어할 수 없다) · §D 프로젝트 표시(글리프·색 대비·CJK 클립이 시각
오라클). 두 절의 체크 항목은 하네스 문서 §5.1 "수동 QA 로 남긴 항목" 에 체크박스로 두었다.

**테스트 단계가 미작성으로 남긴 것**

1. `domain/notification/service.rs decide_delivery` — 기존 6건이 마스터·카테고리 6종·포커스·
   비포커스전용 off·사유 우선순위 전 조합을 이미 덮어 중복이라 추가하지 않았다.
2. `domain/terminal/commands.rs report_command_marker` 의 "`C` 없이 `D` 는 미발행"(§3.6 F-1 계약) —
   `AppHandle`·`TerminalStore` state 를 요구하는데 이 저장소에 `tauri::test` mock-app 하네스가 없다
   (`commands.rs` doc 에 명시). 자동 검증하려면 `started_at` 소비부를 `AppHandle` 없는 순수 함수로
   빼는 소규모 프로덕션 리팩토링이 필요해, 테스트 단계의 minimal diff 원칙상 손대지 않았다.
3. `resolveSplitTerminalCwd` 의 `liveCwd === ''` 경로 — `??` 가 폴백하지 않고 `!candidate` 로 루트
   폴백된다. OSC 7 이 빈 경로를 보고할 일은 없어 보이나 의도된 계약인지 불명해 고정하지 않았다.

**최종 검증 실측**(저장소 루트, 2026-09-04)

| 명령 | 결과 |
|------|------|
| `bun run verify` | **exit 0** — typecheck 오류 0 · lint 0 errors / 9 warnings(기존 9건 그대로) · format:check 통과 · `bun test` **2008 pass / 0 fail / 3340 expect, 203 파일** · `cargo fmt` 통과 · `cargo clippy -D warnings` 경고 0 · `cargo test` lib **1288 pass / 0 fail**, 통합 3·6·17 pass, doc-test 0 |
| `bunx vite build` | **exit 0** — 청크 경고는 기존(monaco·pdf) |
| `bun run typecheck:e2e` | **exit 0** |

`cargo test` 의 bindings export 후에도 `git status --short` 에 신규 dirty 파일이 생기지 않았다
(`bindings.ts` 는 §3.6 에서 이미 재생성된 상태 그대로).
