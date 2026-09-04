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
