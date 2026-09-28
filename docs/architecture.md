# TAIDE 아키텍처

> 시스템 구조의 정본. 상태 소유권 원칙은 ADR-0004, IPC 상세 계약은 `docs/ipc-contract.md`,
> 영속화는 `docs/data-model.md`, 각 기능 내부는 `docs/features/*.md`.

## 1. 전체 구조

계층은 세 개다. 의존은 아래 방향(문서 표기상 왼쪽→오른쪽)으로만 흐른다.

```
React view (표시 전용)  →  IPC 경계 (typed commands / events)  →  Rust 코어 (상태·자원 소유)
```

- **Rust 코어**: 모든 도메인 상태의 단일 출처. pty·git·LSP 프로세스·파일 와처 등 시스템 자원 소유.
- **IPC 경계**: query / mutation / event 3종으로 고정된 타입 계약. Rust 타입에서 TS 타입을 자동 생성해
  드리프트를 차단한다(ADR-0011).
- **React view**: 조회(캐시)·표시·입력 전달. 도메인 상태를 만들지 않는다. reload 하면 Rust 상태에서
  전체 화면이 복원된다.

## 2. Rust 코어 구조

아래는 주요 경로 발췌이며 전체 workspace 멤버는 `Cargo.toml`이 정본이다.

```
TAIDE/                       (Cargo workspace — members: src-tauri, crates/taide-*)
├── Cargo.toml               워크스페이스 루트. release 프로파일도 여기 (멤버에 두면 무시된다)
├── crates/taide-cli/        `taide` CLI (--wait 마커 방식 — agent-integration.md §2)
│                            **bin 이름은 `taide-cli`** — `taide` 로 두면 앱 바이너리와 출력이 충돌한다
├── crates/taide-model/      Tauri 미의존 공통 ID·AppError(기존 facade)·AppEvent(30종)
├── crates/taide-runtime/    Tauri 미의존 AppServices 조립·AppState·검색/AI 요청·tree cache·flush handshake·EventSink port·TaskSupervisor
├── crates/taide-system/     Tauri 미의존 프로세스 정책·독립 CPU 샘플 저장소
├── crates/taide-ide/        Tauri 미의존 IDE 서비스·MCP JSON-RPC wire·lockfile 자원 정책
├── crates/taide-lsp/        Tauri 미의존 LSP 정책·세션 저장소·실행 파일 해석/프로세스 기동
└── src-tauri/
    ├── src/
    │   ├── main.rs          진입점 (lib.rs 의 run() 호출만)
    │   ├── lib.rs           부트스트랩: 커맨드 등록·플러그인·AppState·복원·폴링 태스크·종료 정리
    │   ├── state.rs         taide-runtime::AppState·FlushTicket 타입 재수출 facade
    │   ├── error.rs         taide-model::error 재수출 facade — 모든 command 의 Result 에러 타입
    │   ├── events.rs        이벤트 payload 타입 (ipc-contract 의 Rust 측 정본)
    │   ├── ids.rs           taide-model::ids 재수출 facade (ProjectId / PaneId / TabId 등)
    │   ├── paths.rs         앱 데이터 디렉토리 경로 규칙 (data-model.md §2)
    │   ├── constants.rs     무시 목록·파일 크기 4단계 임계값 (워처·트리·검색이 공유)
    │   ├── domain/          도메인 로직 (한 도메인 = 한 모듈, 총 25개 — commands/capability 일부는 Tauri 결합)
    │   │   ├── agent/       에이전트 감지, wait 마커, CLI 설치 상태
    │   │   ├── ai/          AI 기능 — 자동완성·Inline Edit·커밋 메시지 (provider 3종: Codex·Ollama Cloud·OMLX)
    │   │   ├── app/         앱 정보 (버전·플랫폼)
    │   │   ├── file/        파일 열기/저장/생성/이동/삭제, 크기 정책, dirty 미러
    │   │   ├── font/        시스템 폰트 열거 (fontdb) (7.5-D)
    │   │   ├── git/         status/diff/blame/log/stage/commit/push + watch.rs(무효화 분류)
    │   │   ├── ide/         IDE MCP WebSocket·인가·도구 실행 (wire는 taide-ide — agent-integration.md §3·§7.4)
    │   │   ├── layout/      탭·스플릿·포커스 (PaneNode 트리), 멀티 윈도우 탭 이동
    │   │   ├── locale/      번역 메시지 로드/병합 + 사용자 언어팩 (7.5-H)
    │   │   ├── lsp/         LSP 세션 IPC·AppHandle 콜백·상태 이벤트, 서버 설치 조립
    │   │   ├── notification/ OS 네이티브 알림 게이트 (설정·카테고리·앱 전체 포커스 판정)
    │   │   ├── plugin/      플러그인 매니페스트 로드·검증
    │   │   ├── project/     프로젝트 열기/닫기/목록, capability 확장점(trait·레지스트리 — §3)
    │   │   ├── remote/      원격 접속 서버 (axum WS·비밀번호 인증·허용/거부 정책·이벤트 팬아웃)
    │   │   ├── search/      프로젝트 전역 텍스트 검색 (자체 병렬 스캔 + regex)
    │   │   ├── settings/    앱 설정
    │   │   ├── snippet/     스니펫 저장·열거
    │   │   ├── sync/        GitHub 연동 (설정·테마·언어팩 동기화)
    │   │   ├── system/      시스템 사용량 (CPU·메모리)
    │   │   ├── task/        작업 러너 (package.json scripts·Makefile·Cargo.toml 감지)
    │   │   ├── terminal/    pty 세션, 링버퍼, 셸 프로필, 터미널 경로 해석
    │   │   ├── theme/       테마 로드/해석/번들 38종 + 사용자 테마 저장·삭제 (theme-system.md)
    │   │   ├── tree/        파일 트리 (Rust 소유 + flat rows 페이지네이션)
    │   │   ├── vsix/        VSIX 추출 (테마·grammar 임포트)
    │   │   └── window/      보조 윈도우 생성·닫힘·복원, 기존 WindowStore 공개 facade
    │   ├── platform/        Tauri platform adapter와 레지스트리 공개 facade
    │   │   ├── asset_protocol.rs   열린 프로젝트 asset URI 응답
    │   │   ├── event_sink.rs       AppEvent를 기존 Tauri 이벤트로 발행
    │   │   ├── navigation_guard.rs 웹뷰 탐색·새 창 URL 가드
    │   │   └── window_registry.rs runtime 레지스트리 공개 경로 facade
    │   └── infra/           외부 자원 어댑터 (22파일)
    │       ├── pty.rs       portable-pty 래퍼 (배칭·flow control·링버퍼)
    │       ├── lsp_proc.rs  LSP 자식 프로세스 + JSON-RPC 프레이밍
    │       ├── lsp_install.rs  LSP 서버 다운로드·설치
    │       ├── watcher.rs   notify + debouncer (무시 목록 필터 — 이벤트·파일ID 캐시 양쪽, §2.3)
    │       ├── persist.rs   원자적 쓰기 (temp → fsync → rename)
    │       ├── archive.rs   tar/zip/xz 해제 (LSP 설치·VSIX)
    │       ├── asset_protocol.rs  platform asset 프로토콜 공개 경로 facade
    │       ├── navigation_guard.rs  platform 웹뷰 가드 공개 경로 facade
    │       ├── perf.rs      성능 계측 레지스트리 (게이트 뒤 고정 슬롯 + 원자 카운터 — §2.2)
    │       ├── crypto.rs    constant_time_eq 등 (ide·agent 가 재사용)
    │       ├── secret.rs    OS keyring (SecretStore trait)
    │       ├── http.rs      reqwest 래퍼
    │       ├── clock.rs / external_url.rs / language.rs / range_file.rs / redact.rs / root_guard.rs /
    │       │   self_write.rs / shell_integration.rs / shell_quote.rs  (보조 유틸)
    ├── tests/               도메인 경계를 넘는 통합 테스트 (session_restore.rs) +
    │                        도메인 경계 아키텍처 테스트 (domain_boundaries.rs — 화이트리스트 기계 강제)
    └── capabilities/        Tauri 권한 정의 (최소 권한 — NFR-7)
```

> **초안 대비 실제 구현에서 달라진 것**
> - `infra/repo.rs`(git2 래퍼)는 만들지 않았다 — git2 호출이 `domain/git/service.rs` 안에 있다.
> - `infra/proc.rs` 대신 용도별로 `infra/pty.rs` 와 `infra/lsp_proc.rs` 로 나뉘었다.
> - `src/cli/` 가 아니라 **별도 크레이트 `crates/taide-cli`** 다 (워크스페이스 구성).
> - 초안에 없던 도메인이 순차 추가되어 25개가 됐다: `app`·`tree`·`agent`(초기), Phase 7.5 의
>   `locale`·`font`, 이후 웨이브에서 `ai`·`ide`·`remote`·`snippet`·`sync`·`system`·`task`·`vsix`·
>   `window`, 사용성 배치 4 의 `notification`. `locale` 은 **테마와 완전히 같은 구조**다(내장 정의
>   + 사용자 파일 열거 + `extends` 부분 병합) — 같은 문제를 두 번 푸는 대신 검증된 구조를 재사용했다.
> - `notification` 은 **상태를 갖지 않는 게이트 도메인**이다(스토어 없음, `app.manage` 없음).
>   "OS 알림을 보낼지" 의 판정을 Rust 가 소유하는 이유는 두 가지다. (1) "앱이 비포커스" 는
>   `webview_windows().values().any(is_focused)` 로만 정확하다 — 창마다 JS realm 이 분리돼 있어
>   프론트가 자기 창만 보면 보조 창을 보고 있는 사용자에게도 알림이 간다. (2) 알림 트리거가
>   전 창 브로드캐스트 이벤트(`agent:state-changed`·`lsp:install-progress`)라 판정을 프론트에
>   두면 열린 창 수만큼 중복 발화한다. 텍스트(제목·본문)는 반대로 **프론트가 소유**한다 —
>   `t()` 카탈로그와 이벤트 데이터를 가진 쪽이 프론트이고, Rust 는 문자열을 해석하지 않는다.
> - 도메인별 저장소는 소유 crate를 기준으로 분리한다. `TreeStore`·`SearchStore`는 taide-runtime,
>   `LspStore`는 taide-lsp, `TerminalStore`는 taide-terminal, `PluginStore`는 taide-plugin에 있다.
>   `AgentStore`·`AgentHooksStore`는 taide-agent, `GitStore`는 taide-git, `RemoteStore`는 taide-remote, `IdeStore`는 taide-ide에 있다. setup은 필요한 저장소를
>   `app.manage()`로 등록하며, AppServices에 조립된 저장소는 같은 내부 상태를 공유하는 clone을 등록한다.

- 각 domain 모듈은 `commands.rs`(IPC 노출) / `service.rs`(로직) / `types.rs`(직렬화 타입)로 나눈다.
  command 는 얇게: 파라미터 검증 → service 호출 → 이벤트 발행. 로직은 service 에만 둔다.
- domain 은 infra 를 trait 경유로 사용한다(테스트에서 인라인 구현으로 대체 — 백엔드 컨벤션의
  ServiceDb 격리와 같은 취지). **infra → domain 은 타입을 포함해 전면 금지**다(계층 역방향 —
  감사 R4#6). infra 가 도메인 데이터를 필요로 하면 infra 측에 경량 타입을 정의하고 도메인이
  변환해 전달한다(`infra::language::LanguageOverlay` 선례). 기존 4건의 `types` 역참조만 사유와
  함께 화이트리스트로 남아 있다(아래 기계 강제 참조 — 전부 향후 반전 후보).
- **도메인 간 직접 호출은 금지한다. 판정 기준 (T1-I, 2026-08-19 명문화)**:
  - 금지 — 도메인 간 함수 호출(`commands::`·`service::`·`hooks::` 등 실행 경로)과 타 도메인
    Store 타입 직접 참조(`app.state::<XxxStore>()` 포함).
  - 허용 — 직렬화 데이터 타입(`types.rs`) 참조(entities 성격)와 `project::capability` 확장점(§3)
    구현.
  - 필요한 도메인 간 연동은 **상위 조립부(lib.rs setup)가 배선한다**: 프로젝트 수명주기는
    `ProjectCapabilities`(§3), 설정 토글 반응은 `settings::commands::SettingsToggleObservers`,
    탭 닫힘 뒤 IDE pending diff 해소·PTY 회수는 `layout::service::LayoutTabClosedObservers`,
    IDE MCP의 탭 open/close 수명주기는 `ide::server::IdeLayoutActions`,
    IDE diff 결과의 보호된 파일 저장은 `ide::commands::IdeSaveFile`,
    app 파일·sync 다운로드의 설정 적용은 루트 `settings_port::SettingsApplyPort`,
    파일·Git·IDE의 플러그인 언어 조회와 VSIX 설치 확정은 루트 `plugin_port::PluginRuntimePort`,
    복원 프로젝트의 파일·Git 워처 build/register는 루트 `project_restore_port::ProjectRestoreWatchers`,
    원격 WebSocket의 JSON/raw command 호출은 `remote::dispatch::RemoteDispatchPort`,
    에이전트 감지의 터미널 전경 PID 조회는 `agent::commands::AgentForegroundPids`,
    네이티브 메뉴의 최근 프로젝트·번역 조회는 `window::menu::MenuSources`,
    시스템 사용량 프로세스 라벨은 `system::commands::SystemUsageLabelProviders`, 터미널 spawn
    추가 env 는 `terminal::commands::PtySpawnEnvProvider` — 전부 lib.rs 가 구현/클로저를 정적
    등록하고 도메인은 등록된 것을 소비만 한다. (초안이 언급한 "이벤트 버스(내부 broadcast
    channel)"는 실현되지 않았다 — `events.rs` 는 프론트행 IPC 이벤트를 정의하고, 내부 연동이
    필요하면 조립부가 그 이벤트를 `app.listen_any` 로 **구독**한다: 원격 세션 팬아웃
    `fanout_remote_events!`, 네이티브 메뉴 갱신 `listen_for_app_menu_refresh`(d-58 —
    `features/window-chrome.md` §7.3). 반응이 단방향이고 실패해도 커맨드 결과에 영향이 없는
    경우에 한한다.)
    OS 보조 창 생성과 layout 탭 이동처럼 결과·rollback을 같은 mutation guard에서 다루는
    응답 의존 연동은 `lib.rs`의 `layout_move_tab_to_window` command가 직접 조립한다.
    보조 창 닫힘의 flush·등록 해제는 window command가 담당하고, `CloseRequested`·`Destroyed`
    양쪽에서 회수한 결과의 mirror 조회·layout 탭 복귀는 `lib.rs`가 조립한다.
    `taide-runtime::WindowRegistry`는 보조 창 label→project/slot 매핑과 역조회만 소유하며
    두 종료 경로 중 먼저 해제한 쪽만 결과를 받는다. AppServices가 생성한 공유 복제본을
    `lib.rs`가 Tauri 관리 상태로 등록하고 `platform::window_registry::WindowRegistry`와
    기존 `domain::window::commands::WindowStore` 경로는 타입 재수출로 유지한다.
    layout mutation 완료와 보조 창 탭 복귀의 `LayoutChanged`, Git 명령·워처의 status/refs 이벤트,
    terminal 세션의 spawned/exited/cwd/command-finished 이벤트, 설정·테마 변경 이벤트와
    sync connect/disconnect/upload/download 완료 상태 이벤트, 원격 서버 시작·중지 상태 이벤트,
    창 chrome 변경 이벤트와 프로젝트 목록·그룹·셸 슬롯 snapshot 이벤트,
    프로젝트 열기·닫기·활성화·최근 목록 정리 이벤트, 파일 변경·재스캔 이벤트,
    LSP 세션 상태·설치 진행 이벤트, IDE status/diff/save/close-tab 이벤트와
    agent 상태·외부 열기 이벤트와 hot-exit flush 요청은
    `taide-runtime::EventSink::publish(taide-model::app_event::AppEvent)`를 거쳐
    `platform::event_sink::TauriEventSink`가 기존 Tauri 이벤트로 변환한다. 어댑터는 관리 상태에
    AppHandle을 보관하지 않고 발행 호출 동안 빌리며, `finish_mutation`은 port를 인자로 받는다. Git은 기존
    cache 무효화 뒤 status→refs 발행 순서를 유지한다. terminal은 세션 등록 뒤 spawned,
    종료 metadata 갱신 뒤 exited, 실제 cwd 변경과 측정된 command marker에만 각각 발행한다.
    설정은 영속화·상태 갱신·integration observer 완료 뒤 SettingsChanged를 발행하고,
    테마 변경은 그 다음 ThemeChanged를 발행한다. sync는 성공한 네 경로의 상태 반영 뒤
    SyncStateChanged를 발행한다. 원격 서버는 시작 상태 등록 뒤, 중지 신호 뒤 각각
    RemoteStateChanged를 발행한다. 창 chrome은 세션 상태 저장과 mutation guard 해제 뒤
    WindowChromeChanged를 발행한다. 프로젝트 snapshot helper는 session read lock을
    발행 전에 해제한다. 프로젝트 열기는 capability attach 성공 뒤 opened를,
    닫기는 detach 뒤 closed→activated를, 최근 목록 정리는 목록/그룹 갱신 뒤 결과를 발행한다.
    파일 watcher는 self-write 해소 뒤 변경을 발행하며 overflow는 재스캔 이벤트로 분리한다.
    프로젝트 watcher attach·복원의 GitStatusChanged도 같은 port를 사용한다. LSP helper는
    lifecycle snapshot의 generation·last_error와 설치 bytes의 f64 변환·phase·message를
    보존한다. IDE diff/save는 pending 요청 등록 뒤, close-tab은 닫기 성공 뒤 발행하고,
    status는 시작·중지·연결 상태를 반영한다. agent 상태는 변경 diff가 있을 때만 발행하며,
    외부 열기는 대기열 등록 뒤 single-instance 경로에서만 발행하고 원격 fanout에서 제외한다.
    hot-exit 요청은 All/Window/Project handshake 시작 뒤 앱 전체에 발행하며 원격 fanout에서 제외한다.
    기존 `collect_events!` 등록·원격 `listen_any` fanout은 변경하지 않는다.
    `taide-runtime::TaskSupervisor`는 Tauri가 setup에서 주입한 Tokio handle로 IDE reconcile·agent poll·layout flush·프로젝트 watcher 복원
    장기 작업과 agent hook·IDE·remote 자동 시작 작업을 이름별로 중복 없이 실행하고, 완료된 이름은 회수해 재등록을 허용한다.
    반환형 반복 작업 API는 도메인 저장소가 JoinHandle을 보유해 기존 종료 대기를 유지하면서 감독자가 동일 작업의 AbortHandle을 추적할 수 있게 한다.
    공유 `TaskOperationLease`는 등록 worker 이후의 action owner도 마지막 Drop까지 추적한다. 감독자는 ID만 보관하고 lease가 감독자를 강하게 소유하므로 순환 소유가 없다.
    stop_all은 입장을 닫고 task 취소를 요청하지만 operation을 완료로 지우지 않으며 shutdown은 실제 task 완료와 마지막 operation 반납을 함께 기다린다.
    폰트 목록과 시스템 사용량 두 조회는 같은 감독자의 `run_blocking_result`로 시작한 OS 조회 worker를 추적한다. 요청 waiter가 사라져도 실제 완료 전에는 정상 root가 idle이 아니며 시스템 사용량 breakdown은 label 수집부터 최종 응답 조립까지 operation을 보유한다. 실제 OS scan/Direct Exit의 종료 상한은 별도다.
    runtime `agent_actions::agent_list`는 프로젝트 gate 뒤 같은 등록 감독자의 operation을 받아 PID/probe callback·상태 조립·반환까지 보유한다. 빈 PID/cache 경로도 닫힌 입장에서는 조회하지 않는다.
    Native/remote의 공개 wire와 기존 프로젝트 오류 우선순위는 유지한다. `agent_release_marker`는 같은 operation을 mutation lock 대기 전에 받아 삭제/추적 해제까지 보유하고
    AppState 종료 표시 또는 닫힌 감독자에서는 새 요청을 거절한다. Exit cleanup과 이미 입장한 요청의 NotFound 멱등 경쟁은 유지하며 동기 파일 I/O stall·직접 Exit는 미완료다.
    agent hook 서버의 accept/connection 작업은 감독 범위에 있고, taide-agent의 AgentHooksStore는 Tokio accept JoinHandle을 유지한다.
    동시 시작은 첫 서버 정보만 등록하고 뒤늦은 accept 작업을 취소하며 앱 종료에서 저장소 핸들을 명시적으로 중지한다.
    runtime `agent_hook_server`는 cached 응답→uncached operation 입장→native bind 답→accept 등록→shutdown check→store 게시 및 stop의 store 정리/abort 정책을 소유한다.
    Native는 실제 loopback TcpListener·UUID 토큰·accept/connection·AppState shutdown port를 제공한다. uncached bind 대기와 등록/저장 완료는 정상 root가 기다리지만
    cached 경로의 기존 응답 우선순위와 shutdown check 뒤 store 게시 사이의 경쟁은 그대로다. 실제 listener/auth/connection과 OS stall·직접 Exit는 미검증이다.
    remote 서버는 기존 RemoteStore가 shutdown 송신자와 JoinHandle을 유지하면서 TaskSupervisor도 서버 작업을 추적한다.
    중복 bind는 첫 서버를 유지하고 뒤늦은 서버에 종료 신호·취소를 보내며, 일반 중지는 기존 grace wait 뒤 abort 순서를 유지한다.
    IDE 서버 accept 작업도 감독자가 추적하고 taide-ide의 공유 IdeStore가 Tokio JoinHandle을 보유한다. 동시 bind는 첫 서버의 토큰·포트·핸들을 유지하며
    뒤늦은 작업을 취소하고 후보 lockfile을 제거한다. 등록 실패·종료 중 시작의 후보 lockfile도 정리하고, 정상 중지는 기존 연결 취소·pending diff/save 해소를 유지한다.
    accept는 저장소 등록 뒤 시작되고 각 IDE 연결도 감독 범위에서 IdeStore가 핸들을 보유한다. 연결별 writer/알림 전달/요청 작업은 JoinSet이 소유해
    연결 종료·부모 취소 때 함께 중단되며, 서버 종료 뒤 도착한 연결은 저장소가 등록하지 않고 취소한다.
    원격 WebSocket의 writer·이벤트 작업도 감독 범위에 두되 writer의 제한 시간 종료를 유지한다. 원격 요청 작업은 연결별 자식으로 취소하지 않고
    독립 반복 작업으로 추적하므로 이미 받은 요청이 세션 무효화 뒤에도 permit을 기다려 실행될 수 있는 기존 계약은 그대로다.
    원격 서버의 정상 중지 대기도 감독하며 shutdown 신호→grace wait→시간 초과 abort 순서를 유지한다. 앱 종료로 감독자 등록이 닫혔으면 서버를 직접 취소한다.
    메뉴의 최근 프로젝트 작업·보조 창 탭 복귀/flush/복원·전체 hot-exit timeout·프로젝트 attach 시 agent hook 재조정·LSP 종료/재시작 지연은 호출별 ID로 각각 추적하며 `RunEvent::Exit`에서 함께 취소한다.
    메뉴 recent/language listener의 blocking worker도 호출별 key로 직접 추적하고 감독한 async waiter가 결과를 await한다.
    listener는 IO를 수행하지 않으며 언어가 실제로 바뀔 때만 전체 메뉴를 갱신하는 기존 gate를 유지한다.
    stop_all은 queued blocking에 abort를 요청하고 종료 후 등록을 거절하며 worker 진입에서 종료 상태를 확인한다. 이미 시작한 blocking worker는 강제 중단할 수 없어 실제 완료·panic cleanup까지 추적한다.
    async waiter 취소를 worker 완료로 해석하지 않으며 이 API는 시작한 OS 작업의 bounded 종료 대기나 main-thread 메뉴 callback 취소를 보장하지 않는다.
    infra LSP wait worker는 stdout/stderr ReaderTask를 소유한다. child exit flag 뒤 두 reader를 함께 드레인하고 500ms 대기 초과에는 abort 후 실제 완료를 await한 뒤 tail/exit callback을 전달한다.
    LspProcHandle은 wait JoinHandle을 보유하며 mutable await로 대기 취소 뒤에도 재대기할 수 있다. Drop은 종료 요청이고 실제 정상 완료는 child wait·두 reader·exit callback 반환까지다.
    kill과 child wait poll은 같은 gate에서 직렬화하며 회수 poll/child owner Drop 전에 숫자 PID 권한을 닫는다. kill_on_drop의 runtime 취소/오류 fallback은 best effort이며 실제 회수 대기가 아니다. PTY thread·직접 native Exit의 전체 회수는 미완료다.
    이 deadline은 EOF 드레인 대기 한도이며 non-yield callback의 강제 중단·앱/OS의 bounded 종료를 보장하지 않는다.
    PTY의 명시 kill/Drop과 child wait 종료는 같은 PauseGate를 영구 해제하며 늦은 pause 요청은 다시 reader를 가두지 않는다. killer 오류에도 pause를 해제하되 기존 오류를 반환한다.
    Unix native child는 wait owner가 단독 소유하며 blocking WNOWAIT 관찰 뒤 같은 killer mutex에서 시그널 권한을 반납하고 실제 wait로 회수한다. 회수 뒤 kill/Drop은 숫자 PID를 재신호하지 않는다.
    초기 child owner Drop은 권한으로 종료를 요청하고 반납한 뒤 직접 소유한 child를 wait한다. 종료 관찰 오류를 반환할 때도 권한 반납 뒤 소유 child의 wait를 먼저 시도한다. 종료 관찰/OS wait 오류를 성공한 회수로 해석하지 않으며 기존 SIGHUP 정책도 유지한다. Windows 복제 OS handle은 정상 wait 뒤 반납하며 실기 검증은 별도다.
    성공한 PTY spawn은 reader/flusher/wait의 세 std thread handle을 별도 PtyCompletionHandle에 보존한다. 세션의 master/writer를 보유하지 않고 Drop 뒤에도 실제 join할 수 있다.
    join은 대기 시점의 Tokio blocking pool에서 수행하며 mutable await를 mutex 안에 보존해 대기 취소 뒤 재대기한다. panic/오류에도 나머지 worker를 join한 뒤 실패를 반환하며 성공한 실제 join만 완료 플래그를 올린다.
    reader unwind는 flusher stop을 알리고 정상 stop/최종 flush 순서는 유지한다. 이는 blocking Read/callback을 강제 중단하는 계약이 아니다.
    TerminalStore는 spawn lease와 별도 완료 목록을 보유해 제거/교체·미반환 세션도 추적한다. shutdown은 입장을 닫고 종료를 요청하며 wait_for_idle은 마지막 lease·모든 worker·소유한 cleanup task의 실제 완료를 기다린다.
    runtime terminal_actions의 감독 blocking worker는 같은 mutation lock의 owned guard를 보유한다. 요청 취소에도 guard와 lease가 실제 worker에 남고 성공한 결과는 등록까지 guard를 반환한다. 미반환 결과의 Drop도 store에 종료/완료 소유권을 전달한다.
    같은 runtime의 spawn application은 env await→owned guard→shutdown/project gate→inert initial channel 폐기→ID/output/metadata→감독 spawn→삽입→TerminalSpawned를 소유한다.
    명시적 TerminalSpawnPorts로 env future·channel 폐기·UUID·native session factory를 주입하며 Tauri는 기존 scan/exit callback과 실제 native 조립만 남긴다.
    post-await 삽입·이벤트 caller는 같은 감독자의 operation으로 추적하므로 정상 root는 publication의 실제 반환과 guard 반납도 기다린다. guard 이전 env/입장 대기 전체를 감독한 계약은 아니다.
    같은 runtime은 spawn/write를 제외한 공개 action 10개의 resize/pause·kill/attach/detach guard·세션/shell 조회·root 경로 검증·기본 옵션 정책도 소유한다.
    attach sink는 mutation guard 뒤 lazy factory로 생성하고 실제 raw Channel 포장만 Tauri adapter에 남는다. 공개 command signature/Rustdoc와 기존 오류·기본값을 유지한다.
    write action은 input observer→writer 취득→write_all/flush 순서를 runtime에서 유지하며 Tauri는 같은 등록 TaskSupervisor와 native observer callback만 주입한다.
    blocking writer 자체가 감독되므로 요청 waiter Drop/abort 뒤에도 실제 완료까지 정상 root drain에 남는다. shutdown 뒤 신규/아직 시작하지 않은 작업은 거절하며 시작한 write의 강제 중단이나 bounded 종료는 보장하지 않는다.
    native output observer/callback의 실제 조립·나머지 application/root 자원·OS 오류와 강제 bounded 종료는 별도 미완료 경계다.
    생성 중인 PtySpawnOwner는 master/writer·child slot·시작한 thread·integration 경로를 보유하고 실패/언와인드 시 child wait·worker join 뒤 경로를 정리한다. Builder의 thread 시작 오류는 Result로 반환하며 성공한 초기화에서만 세션/완료 handle에 전달한다.
    정상 root drain은 PTY 오류를 성공으로 처리하지 않으며 native 루프 밖에서 기다린다. partial cleanup도 같은 blocking spawn worker에서 실제 완료까지 기다린다. OS 오류·SIGHUP 무시/자손·직접 native Exit·마지막 완료 handle Drop의 전체 회수는 별도 미완료 gate다.
    자동 시작의 설정 조건·오류 처리와 각 서버의 별도 수명주기 소유권은 유지한다. 기존 주기·Tauri runtime도 유지하며
    나머지 서버·세션 lifecycle 작업은 후속 경계다.
    `AppState`와 flush handshake는 model·infra 타입만 참조해 runtime crate에 있고,
    기존 `crate::state` 경로는 같은 타입의 재수출이다. `AppState`는 내부 상태를 Arc로 공유하는 cloneable handle이고
    Tauri 관리 상태 타입과 IPC wire 입력은 유지한다. 터미널 spawn의 내부 Rust State 주입에는 공유 TaskSupervisor가 추가됐다.
    검색 세션 `SearchStore`도 runtime crate가 owner/session별 취소·대체·종료 정리를 소유하고,
    기존 search 명령 경로는 같은 타입을 재수출한다. Tauri 명령의 mutation guard·Channel 경계는 유지한다.
    AI 요청 `AiRequestStore`는 owner/requestId별 중복 시작·취소를 관리하고, 시작별 token이 늦은 완료의
    새 요청 제거를 막는다. token은 순환 소유 없이 store·key·identity를 보유하고 Drop 시 자기 registry/owner만 정리한다.
    취소/수동 finish로 registry에서 제거된 identity도 token Drop까지 live-owner 목록에 남는다. shutdown은 신규 입장을 닫고
    취소를 전달하며 idle 대기는 실제 token owner 회수까지 기다린다. 기존 AI 명령 경로는 같은 타입을 재수출하며 provider·secret·IPC 경계는 유지한다.
    AI 8개 공개 action의 입력 상한·설정 snapshot·provider 해석·prompt 선택·취소 select·finish·응답 조립은
    Tauri 미의존 runtime `ai_actions`가 소유한다. Tauri command는 같은 State/인수/반환 타입으로 위임하며,
    provider HTTP와 secret port는 기존 taide-ai/infra를 사용한다. 정상 root는 같은 AppServices AiRequestStore의 owner 회수를
    ExitDrain 준비 조건과 직접 Exit의 등록 자원 대기에 포함한다. 이 로컬 요청 정리는 외부 provider가 원격 처리를 중단했다거나 강제 OS 종료까지 회수됨을 보장하지 않는다.
    `TreeStore`는 taide-tree의 프로젝트별 캐시를 runtime의 공유 Arc<RwLock>에 보관하고,
    프로젝트 종료 시 기존 capability가 해당 항목을 제거한다. tree 명령·캐시 경합 정책은 유지한다.
    `PluginStore`는 taide-plugin의 기존 read-through 캐시를 공유 Arc<RwLock>에 보관하고,
    plugin 명령·언어 overlay 포트가 같은 목록을 소비한다.
    runtime `plugin_actions`는 공개 action 5개의 read-through/reload·directory/archive stage→mutation guard→commit/cache·
    uninstall/grammar 조회와 VSIX commit/cache 정책을 소유한다. `vsix_actions`는 공개 action 2개의 기존 추출·stage 뒤
    guard와 commit port 호출을 소유한다. 실제 AppHandle commit port adapter만 Tauri 조립에 남는다.
    두 설치 action의 전체 async 작업과 nested blocking stage는 같은 TaskSupervisor에 등록한다. 요청 waiter Drop은 async 작업 abort를 요청하고 반환 staging의 RAII 소유자는 자기 임시 경로만 정리한다.
    반환 전에 요청이 사라지면 blocking worker의 결과 전송 실패가 staging 소유자를 Drop한다. 정상 root는 이미 시작한 stage와 cleanup 시도의 실제 완료를 기다린다.
    Tauri managed State를 추가했으며 실제 생성 IPC payload는 불변이다. stage→guard→기존 최종 중복 검사/atomic commit/cache와 같은 VSIX 함수 포트를 유지한다.
    OS cleanup 실패나 서비스가 staging 경로 반환 전에 panic하는 경우의 완전 회수·강제 bounded 종료를 보장하지 않는다.
    `AgentHooksStore`는 taide-agent에서 서버 정보·accept 핸들·프로젝트별 활동 override를 공유 Arc<Mutex>에 보관한다.
    AppServices와 기존 agent 명령 재수출은 같은 상태를 소비하며, 중복 서버 시작·900초 override 만료·종료 시 override 정리 정책을 유지한다.
    `AgentStore`도 taide-agent의 공유 Arc<Mutex>로 활동 diff·PTY 세션 신호·PID 이름 캐시·wait marker·외부 열기 대기열을 보관한다.
    기존 agent 명령은 같은 타입을 재수출하며 OS PID 조회·PTY 전달·AgentForegroundPids 주입은 Tauri adapter에 남는다.
    runtime `agent_actions`는 공개 list/release-marker/pending-open 3개의 프로젝트 gate·lazy PID/probe 순서·활동 조립·
    marker 검증/삭제/추적·대기열 선소비를 소유한다. 공용 state/detected-agent helper와 종료 marker cleanup도 같은 runtime 정책이다.
    Tauri poll과 root는 같은 helper를 재수출로 소비한다. runtime `agent_actions::poll_agents`는 프로젝트 snapshot→lazy foreground/probe→
    활동/diff→변경 이벤트→signal/PID cache prune을 소유하며 같은 등록 TaskSupervisor의 operation을 마지막 prune까지 보유한다.
    Native poll은 기존 setup tick에서 같은 state/stores/EventSink·OS port를 주입하고 닫힌 감독자는 poll 입장을 거절한다.
    probe 실패 시 해당 diff는 유지하고 전체 valid-session 집합에는 추가하지 않는 기존 정책과 await 중 제거된 프로젝트의 snapshot 처리는 유지한다.
    runtime `agent_hook_actions`는 공개 status/install/uninstall 3개의
    scope·settings/project gate·shape·read/merge/write 정책을 소유하며 lazy home/emitter/CLI availability/server 포트를 받는다.
    공개 3개 action은 동일한 등록 TaskSupervisor operation을 첫 gate부터 최종 status 반환까지 보유한다. 닫힌 입장은 파일/host port 전에 거절한다.
    install의 emitter/server await와 이어지는 JSON 쓰기까지 이 owner에 포함한다. Tauri 명령과 원격 gateway는 같은 감독자를 주입하고 기존 frontend/remote 인자·응답은 유지한다.
    실제 hook server bind/accept/store 자체의 독립 admission·transport 인증은 이 소유권만으로 완료되지 않는다.
    공통 hook 파일의 invalid JSON 거절·기존 권한/사용자 row·비소유 파일 보존과 loopback URL 조립은 taide-agent에 둔다.
    runtime `agent_hook_reconcile`는 비IPC toggle/reconcile/uninstall과 프로젝트·사용자 파일 reconcile을 소유한다.
    Native는 같은 state·등록 TaskSupervisor와 lazy home/emitter/server/stop port를 주입하며 공개 signature·root cleanup 재수출을 유지한다.
    각 application은 단일 operation을 마지막 파일 적용 또는 disable 후 server stop까지 보유한다. disabled/동일 toggle/닫힌 입장은 host port를 실행하지 않는다.
    home→프로젝트 JSON snapshot/read→lazy emitter→프로젝트 rewrite→인밴드 rewrite→server await→HTTP rewrite 순서와 실패 skip/부분 적용을 유지한다.
    emitter await 전 JSON snapshot으로 쓰는 기존 정책도 유지하므로 await 중 외부 변경을 덮을 위험은 남는다.
    runtime `agent_actions::apply_hook_payload`는 decoded payload의 agent/event gate→프로젝트 snapshot/longest cwd match→override 저장→
    현재 agents의 같은 agent 활동/reason 변경→diff→이벤트를 소유하며 단일 operation을 동기 EventSink callback 완료까지 보유한다.
    빈 agents에도 override는 먼저 저장하고 동일 payload나 해당 agent가 없는 cache는 이벤트를 발행하지 않는 기존 정책을 유지한다.
    Native는 기존 private signature에서 같은 state/stores/EventSink/등록 TaskSupervisor를 주입하고 listener/인증/JSON decode/connection transport는 유지한다.
    runtime `agent_probe`는 Unix PID 이름 캐시·
    Windows lazy process-tree port·CLI emitter 판정/OnceLock 캐시를 소유하고 같은 등록 TaskSupervisor에 실제 blocking worker를 추적한다.
    empty/캐시 hit는 OS port를 실행하지 않으며 caller 취소·기존 CLI 3초 timeout 뒤에도 시작한 worker/버려진 결과 회수를 정상 root가 기다린다.
    실제 PID 조회·CLI 실행은 Tauri adapter에 유지하고 timeout을 CLI kill로 처리하지 않는다. static OnceLock의 동시 최초 probe race도 기존처럼 유지한다.
    poll은 caller 취소·post-await 이벤트 callback·실제 probe worker 결합을 메모리로 검증했다. hook reconcile은 자기 UUID 파일과 가짜 home/port로
    emitter/server await의 caller 취소·정상 root 대기·부분 파일 적용 및 소유 파일 보존을 검증했다. payload도 메모리 정책/owner와 동기 이벤트 callback의
    정상 root 대기를 검증했다. 실제 foreground/앱 실기·server admission/transport·
    모든 action의 전체 입장 선형화와 직접 Exit/OS stall/CLI kill의 bounded 회수는 미완료이며 모든 agent action의 취소 완료를 보장하지 않는다.
    `GitStore`는 taide-git에서 repo root·status 캐시와 같은 repo의 push/fetch 락을 공유한다.
    최초 무효화 구독의 1회 실행은 공유 OnceLock으로 제어하고 실제 세 이벤트 등록은 Tauri adapter의 콜백이 맡는다.
    runtime `git_actions`는 공개 action 41개·repo root 해석·cache·mutation/repo lock·함수별 이벤트 순서를 소유한다.
    status의 perf→구독 callback→루트→cache 순서와 diff의 루트→plugin overlay callback 순서를 유지하며,
    실제 AppHandle 구독/plugin 취득·EventSink adapter는 Tauri에 남는다. GitActionContext가 같은 AppState·GitStore·등록 TaskSupervisor를 전달한다.
    41개 blocking action은 공유 operation과 같은 감독 worker를 사용한다. global mutation 23개·push/fetch의 repo lock 2개는 async owned guard로 취득한 뒤
    caller/worker가 단일 owner를 공유하며, 마지막 guard Drop 뒤 operation을 반납한다. guard 없는 조회 16개도 같은 operation으로 추적한다.
    요청 Drop 뒤 시작한 worker가 남아도 guard가 먼저 풀리지 않고 정상 root는 worker와 post-await cache/event action의 마지막 owner 완료를 기다린다.
    취소된 caller의 post-await cache/event 생략과 동기 cold repo discover는 기존 정책이다. 실제 Git/hook/OS stall·직접 Exit·강제 bounded 종료와 전체 M6 gate는 미완료다.
    status 계산은 슬롯 identity·generation을 함께 검증하므로 슬롯 회수 뒤 같은 프로젝트 ID를 다시 조회해도 이전 계산이 새 캐시를 덮지 않는다.
    프로젝트 조회·활성화/정렬/display·그룹 CRUD·shell slot/chrome·open/open_in_slot/close/group_open의 공개 action 25개와 snapshot helper 3개는
    runtime `project_actions`가 소유한다. 기존 저장→state 반영→함수별 guard 수명→이벤트 순서를 유지한다.
    ProjectLifecyclePort가 같은 native capability detect/attach·flush·detach를 주입한다. fresh open의 attach는 guard 밖이며 실패 rollback은 같은 close를 호출한다.
    close는 flush 뒤 guard 안에서 재검사하므로 중복 close의 두 번째 detach·이벤트를 생략한다. group queue는 첫 실제 성공까지 활성화를 이월하며 shutdown 뒤 남은 멤버를 skipped로 보고한다.
    비IPC restore_state와 projects_pending_watcher_restore도 같은 runtime 정책이다. setup은 같은 native wrapper를 통해 동기 session/layout/settings 복원·chrome 승격/dirty 반영·active 우선 watcher 대상 선택을 수행한다.
    capability attach와 watcher restore의 blocking build는 runtime `project_build`가 같은 등록 TaskSupervisor로 추적한다.
    ProjectBuild는 결과 resource를 operation lease보다 먼저 회수하며 partial move 뒤에도 native commit/발행 스코프가 끝날 때까지 lease를 보유한다.
    요청/restore waiter 취소 뒤 시작한 worker·미등록 결과 Drop은 정상 root가 기다린다. 실제 watcher queue/register callback·window flush 정책과 전체 자원 종료는 별도 미완료 gate다.
    `RemoteStore`는 taide-remote에서 서버 Tokio 핸들·종료 신호·세션 digest·nonce·로그인 잠금과 클라이언트 수를 공유 Arc<Mutex>에 보관한다.
    이벤트 broadcast·세션 epoch watch도 같은 채널을 공유하며, 링크/nonce 1회 소모·만료·독립 잠금 축·전체 세션 해제/서버 중지의 epoch 증분을 유지한다.
    HTTP/WS 서버·부팅 secret cache refresh·상태 이벤트 발행·감독 작업 조립은 Tauri adapter에 남는다.
    runtime `remote_actions`는 공개 action 5개의 cache status·일회 link/host snapshot·password trim/hash/secret 성공 뒤
    cache/revoke·명시적 revoke 정책을 소유한다. 실제 keyring 구현은 같은 infra port를 사용하며,
    비IPC 서버/HTTP/WS·부팅 cache refresh·이벤트/감독 조립과 전체 shutdown은 별도 미완료 경계다.
    `IdeStore`는 taide-ide에서 서버/연결 Tokio 핸들·pending diff/save 응답·선택·진단·클라이언트 수를 공유 Arc<Mutex>에 보관한다.
    알림 broadcast도 같은 채널을 공유하며 탭/프로젝트 종료의 응답 해소·현재/최신 선택·진단 준비 상태·원격 owner 차단 정책을 유지한다.
    원격 owner 라벨은 model의 단일 상수이며 remote crate와 기존 Tauri 경로는 재수출한다. 실제 MCP 서버·lockfile·PTY readiness 대기/환경 주입은 Tauri adapter에 남는다.
    runtime `ide_actions`는 공개 action 7개의 selection owner gate·snapshot/notification/store 순서,
    diagnostics·status·at-mention과 pending diff/save 소비·같은 IdeSaveFile port·guard·함수별 오류/응답 정책을 소유한다.
    Tauri command의 공개 입력/반환/문서는 유지하며 비IPC 서버·연결·lockfile·pending reconcile와 전체 종료는 별도 미완료 경계다.
    `LspStore`는 taide-lsp의 공유 내부 상태에 세션 맵과 독립적인 강한 프로세스 목록을 보관한다. 실제 spawn/restart는 입장 gate 안에서 생성·등록하며 shutdown 뒤 factory를 거절한다.
    제거/교체된 세션의 프로세스도 완료까지 보유한다. shutdown 뒤 wait_for_idle은 실제 worker 완료를 기다리며 완료 핸들은 다음 spawn/대기에서 정리한다. 동기 factory는 같은 프로세스 gate에 재진입하지 않는다.
    `LspInstallStore`는 taide-lsp의 서버별 설치 슬롯을 공유 Arc<Mutex>에 보관하고,
    설치 중복·취소·guard 해제 상태를 LSP 설치 명령과 공유한다.
    `TerminalStore`는 taide-terminal의 공유 Arc 상태에 PTY 세션 맵·spawn 입장 수·제거/교체 세션의 완료 목록과 cleanup task를 보관하고,
    터미널 명령·프로젝트 종료·앱 종료 경로가 같은 상태를 소비한다. 완료 목록은 master/writer를 소유하지 않으며 실제 성공 join 뒤에만 정리한다.
    `SystemUsageStore`는 taide-system에서 앱 PID와 전체 프로세스의 CPU 이전 샘플을 독립 sysinfo 인스턴스에 보관한다.
    두 내부 Arc<Mutex>는 clone 간 공유하고, Tauri 명령은 blocking 실행과 도메인별 PID 라벨 조립만 맡는다.
    `RemoteDispatchLimiter`는 taide-runtime의 공유 Arc<Semaphore>로 원격 요청의 동시 실행을 제한한다.
    상한은 기존 remote 정책 상수에서 조립 시 주입하고 초과 요청은 permit을 기다린다.
    `PlatformServices`는 OS 경로 열기·항목 표시·URL 열기·알림 전달을 제공하며, runtime은 Tauri를 모른다.
    `taide-runtime::system_actions`는 열린 프로젝트의 strict owning-root와 외부 URL을 검증한 뒤 platform을 호출하며,
    app-data enum의 디렉터리를 생성한 뒤 reveal한다. CLI 예외나 새로운 경로 실재 gate·mutation guard를 추가하지 않는다.
    `notification_actions`는 시크릿 마스킹→설정 snapshot→주입한 전 창 focus 조회→기존 delivery 판정→조건부 전송을 소유한다.
    focus callback은 설정 read lock을 놓은 뒤 억제 설정에서도 호출한다. 실제 전 창 조회·usage PID/label 공급·OS 설정의 고정 macOS URL/cfg는 Tauri adapter에 남는다.
    실제 Tauri opener·알림 플러그인은 platform adapter가 소유하며 Delivered는 OS 표시 성공이 아니라 플러그인 전달 결과다.
    `SecretStoreState`는 taide-infra의 Arc<dyn SecretStore> 포트로 clone 간 같은 구현을 공유한다.
    OS 키링 구현과 service identifier 선택은 Tauri 조립부에 남고 runtime은 주입된 포트만 보유한다.
    파일의 보호된 저장 action은 taide-runtime의 `save_file_within_open_projects`가 소유하고 기존 file service 경로는 재수출한다.
    루트/CLI 권한→모드 보존 원자 저장→self-write 표시→프로젝트 미러 정리 순서를 유지한다.
    `IdeSaveFile`도 runtime 포트이며 IDE diff는 조립부가 주입한 같은 저장 action을 호출한다.
    `taide-runtime::file_actions`가 창 flush 완료 확인을 제외한 15개 파일 application action을 소유한다.
    루트/CLI·entry 권한, mutation guard·blocking 실행, self-write와 미러 정책을 runtime에서 조립하고 Tauri 명령은 같은 인수·응답 계약으로 위임한다.
    file_open의 plugin overlay callback은 권한 확인과 설정 snapshot 뒤에 호출하며 파일 읽기는 같은 등록 TaskSupervisor의 blocking worker에서 수행한다.
    file_save와 file_copy는 종료 operation을 mutation 대기 전부터 보유하고 owned guard를 실제 worker 완료까지 유지한다. copy의 self-write 표시는 실제 복사 성공 뒤 같은 worker에서 수행한다. dirty mirror의 blocking worker도 같은 감독자가 추적하고 untitled 미러 생성은 기존 동기 경로다. 두 미러 생성은 전역 mutation guard를 취하지 않는다.
    raw action은 바이트를 반환하고 Tauri adapter만 ipc::Response로 감싼다. 실제 창 label 확인과 exit는 file_flush_complete에 남는다.
    `taide-runtime::settings_actions`는 설정 조회·patch·테마 변경과 공통 apply를 소유한다.
    공통 apply는 sanitize→저장→live state 적용→주입한 integration callback await→SettingsChanged 순서를 유지한다.
    callback은 이전/적용 Settings snapshot을 소유하며 실제 IDE→agent→remote observer는 Tauri 조립부의 등록 순서로 완료된다.
    직접 settings_update·settings_set_theme는 TaskSupervisor의 취소되지 않는 operation으로 요청 중단 뒤에도 callback·이벤트 완료까지 실행하고 정상 root가 이를 기다린다. app_file_write·apply_settings_file·sync_download의 공유 SettingsApplyPort 소비 경로는 별도 종료 소유 검증이 필요하다.
    공통 apply는 mutation guard를 재취득하지 않고 patch·테마 action은 snapshot부터 이벤트 완료까지 같은 guard를 유지한다.
    app_file_write·apply_settings_file·sync_download는 기존 SettingsApplyPort를 통해 같은 runtime apply를 소비한다.
    테마 변경만 SettingsChanged 이후 ThemeChanged를 추가 발행하며 저장 실패·없는 테마는 observer와 이벤트에 도달하지 않는다.
    `taide-runtime::sync_actions`는 status/connect/disconnect/upload/download 5개의 secret/state·gist snapshot·재검증·저장/이벤트 조립을 소유한다.
    같은 SecretStore와 lazy SyncGistPort factory를 받으며 GitHub HTTP 요청·응답/오류 마스킹과 API-profile client는 Tauri adapter에 유지한다.
    기존 gist update의 round-trip은 mutation guard 밖이고 최초 create는 같은 guard를 유지해 중복 gist 생성을 막는다.
    download는 fetch 뒤 guard를 취득하고 gist 변경→다른 sync 완료→conflict→parse/schema→settings apply→theme/locale→SyncStateChanged 순서를 유지한다.
    Native sync_download는 fetch 완료 뒤에만 같은 TaskSupervisor의 취소되지 않는 apply operation에 입장한다. fetch 중 요청 취소는 기존처럼 허용하고, apply 입장 뒤에는 guard·SettingsApplyPort·theme/locale·SyncStateChanged 완료까지 정상 root가 기다린다.
    settings apply callback은 기존 SettingsApplyPort를 호출하며 보호 설정 strip·기존 payload와 best-effort 파일 적용을 바꾸지 않는다.
    runtime은 이미 workspace에 있는 taide-sync/serde_json을 직접 참조하고 normal graph에 Tauri는 없다. HTTP/keyring 실기·요청 취소와 정상 root 회수는 미완료다.
    `taide-runtime::search_actions`는 프로젝트 루트 확인, 검색 세션 시작/완료/취소, blocking 검색·목록·치환과 파일별 guard/self-write/skip 집계를 소유한다.
    검색·치환 대상 스캔/파일별 치환·목록의 blocking worker는 등록 TaskSupervisor가 실제 완료까지 추적한다. 검색 세션 finish는 worker 소유로 요청 취소·panic 뒤에도 실행하고, 종료 시 신규 입장을 닫은 뒤 현재 세션의 취소 flag를 설정한다.
    검색 batch는 주입된 Send callback으로 전달하고 Tauri adapter만 기존 Channel 전송을 수행한다.
    치환 worker는 AppHandle 대신 같은 AppState clone을 사용한다. 전체 pass가 아니라 파일마다 mutation guard를 취득하며 기존 root guard·skip 상한을 보존한다.
    검색 목록은 비UTF-8 경로를 lossy 문자열로 바꾸지 않고 제외한다. 기존 synthetic Unix 경로 unit도 runtime에 있다.
    perf span과 완료 debug 로그는 Tauri adapter에 유지하며 목록 debug 시간은 action 진입부터 프로젝트 조회·blocking walk 완료까지 측정한다.
    `taide-runtime::tree_actions`는 트리 action 5개와 cache hit/miss·prefetch·entry 재확인·인플레이스 수정 정책을 소유한다.
    조회는 전역 mutation guard 없이 캐시의 read lock으로 페이지를 반환하고, miss는 로컬 트리를 만든 뒤 프로젝트와 기존 entry를 재확인한다.
    다섯 action은 같은 등록 TaskSupervisor operation을 처음부터 마지막 페이지 조립까지 보유하고, prefetch의 실제 blocking worker도 감독자에 등록한다. 수정 action은 prefetch 완료 뒤 기존 mutation/write lock을 취득한다. prefetch는 힌트이며 빠진 디렉터리는 기존 서비스의 read fallback을 유지한다.
    Tauri tree command는 기존 인수·응답·TreeStore 재수출·collapse 문서와 TreeToggle/TreeReveal perf span만 보존한다.
    `taide-runtime::app_actions`는 앱 파일 읽기의 live settings snapshot과 쓰기/parsed 설정 적용의 mutation guard·검증·적용 await를 소유한다.
    Native·원격의 두 앱 쓰기 entry는 같은 TaskSupervisor의 취소되지 않는 operation에서 action 전체를 실행한다. 요청 중단 뒤에도 guard·SettingsApplyPort observer·이벤트가 완료될 때까지 정상 root가 기다리며 prompt 저장도 같은 action 소유 범위에 있다.
    설정 적용 callback만 기존 SettingsApplyPort/AppHandle adapter에 유지하고 remote gated strip은 gateway에서 먼저 수행한다.
    프롬프트 저장은 설정 포트를 호출하지 않으며 기존 service의 validate→atomic write를 소비한다. 제품 버전·process perf registry는 Tauri metadata/진단 adapter에 남는다.
    `taide-runtime::theme_actions`와 `locale_actions`는 현재 테마·언어의 live 설정 snapshot→system 선택/fallback→service load 조립을 소유한다.
    원래 await가 없는 조회를 동기 selector로 제공하며 Tauri async command 시그니처는 유지한다. mutation guard·설정 쓰기·이벤트는 추가하지 않는다.
    같은 runtime 모듈은 나머지 theme list/get/save/delete 4개·locale list/get 2개의 기존 service 위임도 소유한다. 기존 current selector·공개 async signature·오류/저장 정책을 유지하며 OS system 값 공급은 adapter/소비자의 책임이다.
    locale load/exists의 기존 경로 검증 누락은 자기 UUID 경로 이탈 fixture로 확인됐으며 별도 보안 수정 방향을 질문한 미완료 gate다. action 이전 완료를 보안 합격으로 해석하지 않는다.
    `taide-runtime::snippet_actions`는 공개 list/save/delete 3개의 기존 service 위임을 소유한다. 원래 await 없는 호출은 동기 action으로 제공하고 Tauri async signature는 유지한다.
    filename/JSON 검증·원문 atomic 저장·관용 scan/정렬·삭제 오류는 같은 taide-snippet service를 사용하며 mutation guard·이벤트·새 스키마를 추가하지 않는다.
    `taide-runtime::task_actions`는 project_root 확인 뒤 같은 taide-task scan을 TaskSupervisor의 blocking worker로 실행한다. command 문자열을 반환할 뿐 탐지한 task를 실행하지 않는다.
    Tauri command는 기존 managed TaskSupervisor를 추가 주입하며 원격도 같은 등록 상태를 전달한다. 실제 Specta 생성으로 wire payload 불변을 확인했다.
    요청 waiter가 Drop돼도 이미 시작한 scan은 감독에 남고 정상 root는 실제 완료를 기다린다. 종료 뒤 새 입장은 Forbidden이며 강제 중단·bounded 종료를 보장하지 않는다.
    `taide-runtime::layout_actions`는 dirty flush·finish·공통 탭 open/close와 레이아웃 command 정책 18개, root의 탭 창 이동·보조 창 탭 복귀 정책을 소유한다. 기존 layout service 경로는 재수출 또는 adapter로 유지한다.
    finish의 focus 보정·dirty 표시·LayoutChanged 발행 뒤에 호출자가 state를 기록하는 기존 순서를 보존한다.
    close의 observer는 state 기록 뒤 같은 mutation guard 안에서 주입된 callback으로 실행하며 실제 IDE→PTY 후처리 등록은 Tauri 조립부에 남는다.
    dirty flush는 먼저 drain하고 snapshot을 저장한다. 없는 layout·저장 실패는 기존 로그/생략 정책을 유지한다.
    주기 flush loop는 같은 공유 state·등록 TaskSupervisor·기존 2초 interval을 받는 runtime 정책이다. 즉시 첫 tick·기본 Burst와 실제 blocking worker await를 유지한다.
    ticker를 취소해도 시작한 저장 worker는 감독자가 완료까지 추적하며 정상 root가 기다린다. window/exit의 동기 flush와 전체 종료·저장 오류 정책은 별도 남는다.
    Tauri layout command 19개 중 18개는 runtime action에 위임하고 닫기는 기존 observer adapter를 통해 같은 공통 runtime close를 소비한다.
    mutation helper의 14개 소비자는 tab/pane/project 대상 선정·guard·clone·정책·finish 이벤트→state write를 공유한다.
    파일 열기와 split의 CLI 허용/실재 파일 gate, untitled 변환의 strict root gate와 경로 변경의 성분 단위 containment도 runtime에 있다.
    닫힌 탭 stack만의 개명은 dirty/state를 기록하되 revision이나 이벤트를 추가하지 않는다. 실제 보조 창 생성/close와 IDE→PTY observer는 Tauri adapter에 남는다.
    창 이동은 같은 guard에서 새 OS 창을 먼저 열고 이동 실패 시 close callback을 호출한다. 성공 뒤 빈 layout 슬롯을 제거하고 등록된 창을 닫으며 finish 이벤트→state write를 유지한다.
    창 복귀는 기존 위치에서 mirror를 조회하고 조회 실패를 빈 snapshot으로 취급한다. phantom file dirty 정리→탭 복귀→state write→dirty/event의 순서를 유지한다.
    실제 창 닫힘 이벤트의 WindowRegistry 제거와 TaskSupervisor의 auxiliary-tab-return 등록은 Tauri 조립부에 남으며 runtime은 registry를 임의로 forget하지 않는다.
    synthetic 검사와 본문 대조는 실제 OS 창 생성/close 및 실패 뒤 OS 창 rollback 성공을 증명하지 않는다. 해당 실기는 M7과 창 정책 QA gate에서 확인한다.
    `taide-runtime::lsp_install_actions`는 download 설정·플랫폼/checksum 확인→다운로드→검증→감독된 extraction→atomic 적용→진행 이벤트의 정책을 소유한다.
    `taide-runtime::lsp_actions`는 세션 snapshot 조회·bundled server 확인 뒤 기존 root 탐지·설치 취소를 소유한다. worker lease가 끝나기 전 설치 슬롯을 해제하지 않는 기존 store 정책을 유지한다.
    같은 runtime은 현재 proc의 write await와 crashed generation의 재초기화 확인/실패 기록·status event 조립도 소유한다. 기존 실패 문구·generation/status 조건·이벤트 payload를 보존한다.
    send의 IPC counter는 Tauri adapter에 남으며 같은 EventSink adapter를 사용한다. proc 실기·write 중 취소의 완전성은 이 메모리 검사로 증명하지 않는다.
    설치 application의 app shutdown→bundled spec→store 입장→전략 선택/오류도 같은 runtime action에 있다. 기존 install guard/lease와 download/toolchain 구현을 사용하며 Tauri는 같은 상태/이벤트 adapter로 위임한다.
    세션 spawn/stop/restart의 project/server gate·owner/root 재사용·subscriber·workspace notification·등록/rollback·epoch/status 조립도 같은 runtime action에 있다.
    stop은 같은 guard 안에서 full entry를 unlink한 뒤 guard 밖에서 기존 protocol shutdown을 수행한다. restart는 entry를 유지하고 비가드 shutdown 뒤 guard를 재취득해 membership을 다시 확인한다.
    LspActionContext는 같은 상태/감독자를 전달하고 LspSpawnPorts는 lazy Channel sink·UUID·bare native process 생성을 주입한다. runtime의 LspStore.spawn_process가 application process를 한 번만 등록한다.
    native 자동 재시작은 기존 등록 wrapper를 사용하며 bare factory의 epoch/message/exit callback과 recovery/backoff/healthy reset 정책은 유지한다. 중첩 process registry 잠금은 취득하지 않는다.
    post-admission action은 같은 감독자의 operation으로 추적하며 guard가 lease보다 먼저 Drop된다. 닫힌 supervisor의 새 action은 기존 LSP shutdown 오류를 반환한다.
    Tauri의 공개 LSP async signature/문서와 OS PATH server 탐지 본문은 유지한다. 자기 echo process에서 workspace 알림·같은 ID restart·정상 root와 unlink 뒤 stop abort를 검증했다.
    초기 guard 입장 대기·protocol write 도중 취소의 완전성·동기 native 생성의 강제 중단·실제 AppHandle callback/GUI·OS 오류·직접 Exit/강제 bounded 종료는 이 결과로 보장하지 않는다.
    실제 이벤트 전송은 주입된 EventSink/Tauri adapter에 남는다. InstallStore의 요청 guard Drop은 취소를 요청하고 worker lease가 살아 있는 동안 서버 슬롯을 해제하지 않는다.
    extraction과 download 파일 create/write/flush worker는 lease와 UUID 임시 경로 소유자를 보유하며 이미 시작한 작업은 abort로 끝났다고 간주하지 않는다. 실제 작업/임시 경로 정리가 끝나야 재등록할 수 있다.
    infra의 DownloadFileIo는 기존 Tokio 파일 경로와 runtime의 감독된 파일 경로를 제공한다. 실제 설치 action은 후자를 사용하며 stream/hash/throttle 정책은 공유한다.
    열린 파일 소유자는 파일 닫힘→임시 경로 소유자→lease 순으로 Drop한다. queued work도 work capture의 cleanup 뒤에 마지막 worker lease를 해제하도록 하나의 소유 구조체로 캡처한다.
    store 취소와 최종 atomic 적용은 같은 gate에서 직렬화한다. 취소가 앞서면 적용/Done을 거절하고, 적용이 먼저 성공했으면 늦은 취소로 완료 결과를 되돌리지 않는다.
    ExitRequested/Exit는 설치 admission을 닫고 취소를 알리며 신규 command도 AppState 종료 gate를 확인한다.
    정상 ExitRequested는 prevent_exit 후 root callback이 소유한 runtime ExitDrain에서 감독 task/공유 operation owner의 실제 완료·모든 설치 lease·일반 LSP wait/reader/callback·같은 AppServices의 AI token owner·PTY spawn과 PTY worker 종료를 기다린다. coordinator는 자신이 멈추는 TaskSupervisor 밖에 있어 self-wait가 없으며 성공한 완료 뒤 원래 exit code로 종료를 다시 요청한다. PTY join 오류는 준비 플래그/종료 callback을 실행하지 않는다.
    정상 ExitRequested 대기 중 native 이벤트 루프는 계속 동작해 메뉴 worker의 main-thread 응답을 처리할 수 있다. ExitRequested 없이 바로 Exit가 오면 사용자 선택에 따라 같은 등록 자원 전체의 완료를 동기로 기다린다. OS I/O·main-thread callback이 멈추면 시간 제한 없이 대기할 수 있고, 실제 native Exit/GUI 교착 여부와 등록되지 않은 자원 회수는 검증되지 않았다.
    HTTP 파일 생성 중 요청 Drop의 늦은 파일 1개와 슬롯 조기 해제를 재현하고 create/write/flush의 감독 소유권으로 수정했다.
    `taide-runtime::lsp_install_toolchain`은 감독된 blocking worker 안에서 취소 gate와 child spawn을 직렬화한다. store는 자원을 weak 등록해 순환 소유 없이 요청 Drop·명시 취소·shutdown을 동기로 전달한다.
    child 소유자는 직접 child를 kill/reap한 뒤 lease를 해제한다. Unix infra는 waitid의 WNOWAIT로 부모 PID를 회수하지 않고 종료를 관찰하며 자기 그룹에 KILL을 전달한 뒤 부모를 회수한다. mutex 안의 회수 플래그로 늦은 취소/Drop의 PID 재사용을 막고 0/1을 거절한다.
    macOS에서 종료한 부모 하나만 자기 PGID에 남은 경우만 EPERM을 빈 그룹으로 판정하며 다른 권한 오류는 반환한다. libc는 기존 0.2.189를 Unix 직접 의존으로 재사용하고 사용자/전체 프로세스 목록을 조회하지 않는다.
    stdout/stderr reader는 별도 감독 task에서 pipe와 lease를 보유한다. child 종료 뒤 EOF를 최대 500ms 기다리고 지연되면 abort 후 실제 JoinHandle 완료를 확인해 마지막 20줄을 반환한다.
    정상 완료/기존 실패 출력 마스킹·진행 payload는 공유 EventSink를 경유하며 취소가 앞선 경우 Done을 거절한다. 자기 생성 TERM 무시/부모 선종료 자손과 정상 종료 coordinator의 실제 대기는 검증했다. 그룹을 벗어난 자손·실제 native 종료 이벤트/Windows process tree·감독되지 않은 nested worker는 미검증이고 전체 lifecycle은 미완료다.
    IDE diff의 mutation guard·blocking 실행과 Forbidden 예외 정책은 기존 호출자에 남는다.
    setup은 상태 복원 뒤 AppState·TaskSupervisor·원격 제한기·플랫폼 포트·시크릿 포트·IDE 저장 포트·EventSink를 주입해 `Arc<AppServices>`를 만들고, 나머지 저장소는 AppServices가 초기화한다.
    AppState·SearchStore·AiRequestStore·TreeStore·TerminalStore·PluginStore·AgentStore·AgentHooksStore·GitStore·RemoteStore·IdeStore·SecretStoreState·IdeSaveFile·LspStore·LspInstallStore·SystemUsageStore·RemoteDispatchLimiter·PlatformServices·WindowRegistry·TaskSupervisor의 20개 상태·포트를
    기존 Tauri State로 등록한다. 추가 EventSink는 `Arc<dyn EventSink>`로 보유해 AppServices의 전체 필드는 21개다.
    한 TauriPlatformServices Arc가 OS 포트와 이벤트 포트 모두를 제공하며 publish는 기존 빌린 TauriEventSink의 매핑으로 위임한다.
    이벤트 포트의 별도 Tauri State나 새 이벤트 버스는 만들지 않는다. 나머지 Tauri 관리 상태와 application action facade 추출은 후속 경계다.
    부팅 1회성 복원은 `lib.rs`가 상태 로드→관리 상태 등록→워처 재부착 순서를 소유한다.
    `project::commands`는 순수 대상 선정과 프로젝트별 guard·경합 제어를 유지하고,
    layout/settings 로드는 독립 crate, file/git watcher build/register는 조립부 포트를 사용한다.
  - 불가피한 잔여 엣지는 **화이트리스트로 명시 승인**한다. 현재 허용 목록은 비어 있다.
    `src-tauri/tests/domain_boundaries.rs`
    의 소스 스캔 테스트가 화이트리스트 밖의 도메인 간 참조와 infra→domain 참조를 기계 강제로
    거부한다(미등재 = 실패, 실재하지 않는 등재 = 실패 — T1-K "기본 거부"와 동형). 각 항목의
    승인 사유는 추가 시 그 파일의 화이트리스트 doc 에 기록한다. 원격의 전 도메인 command 게이트웨이는
    `src-tauri/src/remote_gateway.rs` 조립 계층으로 옮겨 파일 단위 예외가 없다.

### 2.1 스레딩 모델

- Tauri 기본 tokio 런타임 사용. 명령은 원칙적으로 `async fn`.
- 블로킹 작업(git2 호출, 대형 파일 IO)은 `spawn_blocking` 으로 격리한다 — libgit2 는 동기 API 다.
- 장수 태스크(pty reader, LSP stdio pump, watcher)는 tokio task 로 상주하며,
  소유 도메인의 세션 구조체가 JoinHandle/CancellationToken 을 보유해 종료를 보장한다.
- AppState 접근은 짧은 잠금 원칙: 잠금 안에서 IO 금지.
- `spawn_blocking` 스레드풀(tokio 기본 상한 512)은 **프로세스 전역**이며 데스크톱 IPC 커맨드와
  원격 dispatch 커맨드가 같은 풀을 공유한다 — 원격 쪽은 `RemoteDispatchLimiter`(128, d-35)로
  자기 몫만 상한을 두지만 데스크톱 쪽 `spawn_blocking` 호출에는 별도 상한이 없다.

### 2.2 성능 계측 (`infra/perf.rs` — 상주, 게이트 뒤)

성능 작업은 **추측이 아니라 실측 우선**이라는 결정(`2026-09-04-usability-batch4-user-decisions.md`
§2)의 Rust 측 구현이다. 사용법·슬롯 목록은 `docs/debugging.md` §4.1, IPC 표면은
`docs/ipc-contract.md` 의 app 도메인.

- **소유권**: `infra::perf` 가 레지스트리와 슬롯 표를 소유한다. 프로세스 전역 인스턴스는
  `perf::global()` 1벌이며 `app.manage` 하지 **않는다** — `perf::span()` 이 `State` 주입 없이
  어느 코드에서나 불릴 수 있어야 커맨드 시그니처를 건드리지 않고 계측 지점을 늘릴 수 있다
  (`locale::service` 의 `OnceLock` 웜업 캐시와 같은 프로세스 전역 선례).
- **게이트가 설계 제약이다**: 계측은 릴리스 빌드에도 컴파일된다(사용자가 `TAIDE_PERF=1` 로
  재현 수치를 보낼 수 있게). 그래서 모든 계측 지점은 off 일 때 **원자적 `load` 1회**만 쓴다 —
  `Instant::now()` 도 하지 않는다. 게이트는 부팅 시 1회 결정되고 실행 중 바뀌지 않는다.
- **전역 잠금 금지**: 기록은 `&'static str` 이름의 **고정 슬롯 배열 인덱스 + `AtomicU64`** 다.
  `Mutex<HashMap<&str, _>>` 로 두면 병렬 검색 워커·pty 리더 스레드·git `spawn_blocking` 풀이
  한 락에 줄서서 **자기 경합을 측정**한다(조사 3b §7). 슬롯 집합이 컴파일 타임에 닫혀 있는
  이유가 이것이고, 슬롯 추가는 `SpanSlot`/`CounterSlot` 의 `ALL`·`name` 동시 갱신을 요구한다
  (인덱스 파리티 테스트가 강제).
- **고빈도 경로는 카운터만**: pty 리더 루프·`lsp_send` 는 누적 바이트/횟수만 올린다. 구간 시간을
  넣으면 계측이 측정 대상을 왜곡한다. 처리량은 두 스냅샷 사이의 벽시계로 나눠 구한다.
- **커맨드별 호출 수는 assembly 가 배선한다**: `lib.rs` 의 `invoke_handler` 클로저 한 곳에서
  이름별 카운터를 올린다(커맨드 본문 무수정). 이름 표는 `dispatch::IMPLEMENTED_JSON_COMMANDS`
  ⊎ `RAW_CHANNEL_COMMANDS` 를 `perf::init` 에 넘겨 만든다 — 커맨드 이름의 정본이 이미 그
  표이고 `collect_commands!` 파리티 테스트가 그것을 핀 고정하므로, 두 번째 목록을 만들지 않는다.
  표에 없는 invoke(tauri 플러그인 커맨드 등)는 `command.unlisted` 로 모인다.
- **레이어**: `infra::perf` 는 domain 을 참조하지 않는다(§2 의 `infra → domain` 금지). 와이어
  타입(`PerfSnapshot`/`PerfEntry`/`PerfCounterEntry`)은 `domain/app/types.rs` 가 소유하고
  `domain/app/service.rs::perf_snapshot` 이 ns → ms 환산과 정렬을 담당한다 — specta 가 64비트
  정수 export 를 금지하므로 환산이 한 곳에 있어야 한다.

### 2.3 파일 워처의 파일-ID 캐시는 무시 목록으로 스코프된다 (`infra::watcher::ScopedIdCache`)

`notify-debouncer-full` 은 rename 의 두 절반을 잇기 위해 파일-ID 캐시를 둔다. 기본값
(`RecommendedCache` = macOS 의 `FileIdMap`)은 감시 루트를 `WalkDir(follow_links, max_depth=MAX)`
로 훑으며 **엔트리마다 `stat`** 하고, `IGNORED_DIR_NAMES` 를 적용하지 않는다. TAIDE 는 그 자리에
`ScopedIdCache`(`FileIdCache` 자체 구현)를 주입해 **`group_relevant_changes` 가 이벤트를 남기는
경로 집합과 정확히 같은 집합**만 인덱싱한다 — 무시 디렉토리는 **자신 1엔트리로 끝내고 그 아래로
내려가지 않는다**(그 디렉토리 자신의 이벤트는 "마지막 성분 제외" 규칙 때문에 살아남으므로 캐시에
남겨야 짝짓기가 유지된다).

- **효과**(이 저장소 실측, warm): 인덱스 대상 557,474 → **1,461 엔트리**(약 382배), `stat` 워크
  3.79s → **0.01s**, 상주 메모리 프로젝트당 대략 105MB → **0.2MB**(엔트리당 `PathBuf` 24B +
  `FileId` 24B + 해시맵 오버헤드 + 경로 문자열 힙 — 경로 텍스트만 65.9MB → 0.11MB). rename 마다
  도는 `remove_path` 의 O(n) `retain` 도 같은 배수로 줄어든다. `project_open` 은 이미 §3.1
  로 락 밖에서 build 하므로 이 절감은 잠금 시간이 아니라 **CPU·메모리**로 돌아온다. 부가 효과로
  `npm install` 처럼 무시 디렉토리를 새로 만드는 이벤트가 전 트리 재인덱싱을 유발하던 것도 사라진다
  (debouncer 의 `Create` 처리가 `add_path` 를 다시 부르기 때문이었다).
- **d-35 §4-e 의 `NoCache` 기각과 다르다**: 기각 대상은 **전면** `NoCache` 였다. macOS FSEvents 는
  rename cookie 를 주지 않아 짝짓기가 `file_ids_match` 단 하나뿐인데, 전면 무캐시는 모든 rename 의
  짝짓기와 함께 `push_rename_event` 의 **재귀속**(같은 디바운스 창에 old 경로로 큐잉된 다른 이벤트를
  new 경로로 옮기는 처리)까지 잃는다. 스코프 캐시는 앱이 보고하는 경로쌍의 짝짓기를 **그대로
  유지**하고, 무시 경계를 넘는 이동에서만 짝짓기를 포기한다 — 그리고 그 경우 소비자가 보는 결과는
  동일하다: 짝지어지면 `paths=[old,new]` 한 그룹에서 무시 쪽 절반이 필터로 떨어지고, 짝지어지지
  않으면 두 이벤트가 각각 `Renamed` 로 매핑돼 무시 쪽이 필터로 떨어진다. 남는 유일한 저하는
  "수정 후 300ms 안에 무시 경계 너머로 이동" 시퀀스의 재귀속 상실이며, 그 결과는 사라진 경로에 대한
  무해한 `FILE.CONTENT` 무효화 1회다.
- **열린 탭 추종(`layout_apply_path_change`)은 영향받지 않는다**(착수 전 선행 확인): 추종은
  `entities/file/file.query.ts` 의 `useRenameEntry.onSuccess` → `followRenamedPathInTabs` 가
  구동한다. 워처의 `fs:changed` `Renamed` 그룹은 이 경로를 부르지 않으므로, 이벤트가 1건이든
  2건이든 탭 추종 계약과 무관하다.
- **git 워처(`WatchScope::GitDir`)는 무필터**: 이벤트 필터와 같은 이유다 — `refs/heads/build/login`
  의 `build` 는 브랜치 이름이지 무시 대상 디렉토리가 아니다.
- **심링크**: 링크 엔트리 자체는 인덱싱하되 그 아래로 내려가지 않는다. notify 의 FSEvents 백엔드가
  감시 루트를 canonicalize 하므로 심링크 경로로 주소지정된 이벤트는 애초에 오지 않는다 —
  `follow_links(true)` 가 만들던 엔트리는 조회될 수 없는 사본이었고, 루프 노출도 함께 사라진다.
- **플랫폼 분기 없음**: 릴리스·CI 타깃이 macOS 단독이라 `cfg` 로 캐시 타입을 갈아끼우지 않는다
  (그러면 `WatcherHandle` 이 플랫폼마다 다른 타입이 된다). Linux 에서는 inotify 가 cookie 를 주므로
  이 캐시가 **틀린 게 아니라 불필요**할 뿐이다.

## 3. 프로젝트 추상화 (핵심 확장 포인트)

프로젝트는 "폴더 + 부착된 capability 집합"이다. 미래 기능(remote-control 등)은 새 capability 로 부착한다.

이 확장점은 **T1-I(2026-08-19)에서 실현됐다** — 정본은 `domain/project/capability.rs` 다. 실현
시그니처는 초안(async + `Result` + `CapabilityCtx`)과 달리 **동기·무오류**다: 이관 대상이던
`project_open`/`project_close` 의 수기 조립 호출이 전부 동기(또는 attach 내부 spawn)였고 실패를
log-warn 으로 삼키는 형태였기 때문에, 실코드의 계약을 그대로 옮겼다.

```rust
trait ProjectCapability: Send + Sync {
    fn detected_kind(&self, root: &Path) -> Option<CapabilityKind>;  // Project.capabilities 에 기록될 검출 결과 (없으면 None)
    fn build_attachment(&self, app: &AppHandle, state: &AppState, project: &Project) -> ProjectAttachment; // 자원 부착 1단 (락 밖)
    fn detach(&self, app: &AppHandle, state: &AppState, project_id: &ProjectId); // 자원 회수 (project_close) — 반드시 대칭
}
```

- 구현체는 각 도메인의 `capability.rs`(layout·file·git·terminal·tree·ide·agent)에 있고, 조립부
  `lib.rs` 의 `project_capabilities()` 가 **정적으로 등록**한다(동적 플러그인 레지스트리가 아니다 —
  과설계 금지, 계약 §1.1). `project_open`/`project_close` 는 등록 목록을 앞에서부터 순회 호출만
  한다. **등록 순서가 곧 close 의 자원 회수 순서 계약**이며(§6.3), lib.rs 의 소스 스캔 테스트가
  순서를 핀으로 고정한다.

- 프로젝트 열기 = core(파일 접근·레이아웃) 초기화 + 감지된 capability 자동 부착
  (예: `.git` 있으면 Git). `detected_kind` 의 검출 결과와 `project::service::open_project` 가
  기록하는 `Project.capabilities` 의 정합은 lib.rs 파리티 테스트가 기계 강제한다.
- capability 는 각자 명령·이벤트 네임스페이스를 가진다(`git_*`, `lsp_*`, ...).
- **remote-control(미래)**: `RemoteControl` capability 가 프로젝트별 로컬 서버를 열고 웹 패널을 서빙하는
  형태로 부착된다. 코어는 capability 등록 API 외에 어떤 전제도 갖지 않는다.

### 3.1 attach 는 2단이다 — build(락 밖) → register(락 안)

`attach` 는 2026-09-04 배치 4에서 **`build_attachment` 1개 훅**으로 재정의됐다. 반환값
`ProjectAttachment`(불투명 boxed 클로저)가 "가드 안에서 실행할 등록 절반"이고, 비싼 절반은 전부
`build_attachment` 본문에서 **가드를 쥐지 않은 채** 끝난다. 조립은
`domain/project/commands.rs::attach_project_capabilities` 한 곳이다.

- **이유**: `project_open` 이 앱 전역 단일 `begin_mutation` 을 쥔 채 워처 attach 를 동기 호출해,
  프로젝트를 여는 동안 `file_save`·`git_*`·`layout_*`·`tree_toggle`·주기 flush 가 전부 정지했다
  (당시 `notify-debouncer-full` 의 `FileIdMap` walk 는 무시 목록 미적용이라 대형 워킹트리에서
  초 단위였다 — 그 walk 자체는 이후 §2.3 이 스코프했지만, 락 분리는 walk 비용과 무관하게 유지된다:
  attach 는 워처 외에도 IDE lockfile 재작성·에이전트 훅 등 IO 를 계속 수행한다).
  §2.1 "잠금 안에서 IO 금지" 를 가장 크게 어기던 지점이다. 부팅 복원 경로(d-25)가 이미 쓰던
  "밖에서 build, 안에서 register + 재검증" 패턴을 trait 로 승격해 `project_open` 에도 적용했다.
- **순서 계약은 그대로**: build 순회와 commit 순회 **둘 다** 등록 순서 전방이고, commit 은 build 가
  만든 벡터를 같은 인덱스로 재생한다. 따라서 capability 별 attach 상대 순서는 이전과 동일하다
  (`domain/project/capability.rs` 단위 테스트가 두 순회를 핀 고정, `project/commands.rs` 소스 스캔
  테스트가 "build → 가드 획득 → commit" 위치를 핀 고정).
- **동작 불변**: `project_open` 은 여전히 attach 완료를 기다린 뒤 `ProjectOpened`/`ProjectActivated`
  를 방출하고 반환한다 — 옮긴 것은 **락**이지 attach 가 아니다. 워처 핸들은 build 가 끝난 시점에
  이미 구독 중이라 build~register 사이 이벤트도 유실되지 않는다.
- Native `project_open`·`project_open_in_slot`·`project_group_open`은 TaskSupervisor의 취소되지 않는 완료 operation에서 runtime action을 호출한다. 요청 future가 사라져도 attach·실패 rollback·event까지 같은 작업이 끝나고 정상 root 종료가 기다린다. menu·원격 gateway도 같은 감독자를 전달한다. 실제 watcher·GUI 종료 검증은 남아 있다.
- **재검증**: 가드 재획득 시 프로젝트가 아직 열려 있는지 확인하고, 아니면 build 결과를 commit 하지
  않고 **버린다**(핸들 drop = 워처 종료). 부분 커밋이 없으므로 detach 대칭이 유지된다.
- **d-25 보정**: attach 완료 후 git 워처가 등록됐으면 `GitStatusChanged` 를 1회 방출한다
  (이벤트로만 갱신되는 `GIT.PROJECT` 캐시 보정). 일반 경로에서는 방출 시점에 아직 `ProjectOpened`
  를 보내지 않았으므로 구독자가 없어 비용 0이고, 실제로 일하는 곳은 후절화가 넓힌 유일한 창 —
  attach 진행 중에 같은 경로로 들어온 **두 번째 `project_open`** 이다(그 호출은 `already_open`
  분기라 워처를 기다리지 않고 반환한다).
- `AppState` 를 건드리지 않는 attach 부수효과(IDE lockfile 재작성, 에이전트 훅 reconcile spawn)는
  build 단계에서 수행하고 `ProjectAttachment::none()` 을 반환한다 — 가드 밖이 항상 더 낫다.

## 4. IPC 경계

패턴은 3종으로 고정한다. 상세·전체 목록은 `docs/ipc-contract.md`.

| 종류 | 방향 | 형태 | 예 |
|------|------|------|-----|
| query | view→Rust | `invoke('git_status', { projectId })` → 데이터 반환 | 상태 조회. 부수효과 금지 |
| mutation | view→Rust | `invoke('git_stage', { projectId, paths })` → Ack/결과 | 의도 전달. 성공 시 Rust 가 이벤트 발행 |
| event | Rust→view | `emit('git:status-changed', payload)` | view 는 관련 query invalidate |

- 고속 스트림(터미널 출력, LSP 알림)은 이벤트 대신 **Channel** 을 사용한다(연구 결과에 따라 확정,
  `docs/research/tauri-v2.md`).
- Rust 타입 → TS 타입 자동 생성(ADR-0011)으로 계약을 단일 출처화한다.
- 명령 이름은 `snake_case` `{domain}_{action}`, 이벤트 이름은 `{domain}:{event-kebab}` 으로 통일한다.
- **원격(웹) 접속에서는 이 IPC 경계가 그대로 노출되지 않는다** — `src-tauri/src/remote_gateway.rs`
  가 명시 허용 목록(`REMOTE_ALLOWED_COMMANDS`)·명시 거부 목록
  (`REMOTE_DENIED_COMMANDS`) 둘 중 하나에 등재된 커맨드만 실핸들러로 위임하는 **기본 거부** 게이트다
  (T1-K, 2026-08-19). 새 커맨드는 `match` arm 추가만으로 원격 도달 가능해지지 않고, 두 목록 중
  하나에 이름을 등재해야 한다 — 상세 분류·전수 목록은 `docs/ipc-contract.md` §"원격 dispatch 정책".

### 4.1 WebView 네비게이션 가드 (앱 창은 브라우저가 아니다)

> 계약: `docs/acknowledge/2026-09-04-usability-batch3-contract.md` §B.2-6. 정책·단위 테스트는
> `src-tauri/src/platform/navigation_guard.rs` 한 곳이고, 기존 `infra/navigation_guard.rs`는
> 공개 경로 facade다. 부착 지점은 `lib.rs` 의
> `create_main_window` 와 `domain/window/commands.rs` 의 `open_auxiliary_window` 두 곳뿐이다
> (앱이 만드는 창이 그 둘뿐이므로 정책이 갈라질 여지가 없다).

- **왜 필요한가**: 앱 창 자체가 외부 사이트로 이동해 버리면 되돌아올 UI 가 없다(주소창·뒤로가기가
  없다). 프론트의 단일 오프너(`entities/system/external-url.ts`)가 1차 방어지만, OSC 8
  하이퍼링크·마크다운 앵커·`window.open()` 처럼 그 오프너를 우회하는 JS 경로가 실제로 있었다
  (`docs/bug/2026-09-04-external-link-opens-in-app-window.md`). 이 가드는 **어떤 JS 경로가 새더라도**
  창이 오리진 밖으로 나가지 못하게 하는 2중 방어다.
- **네비게이션 허용 목록**(`is_navigation_allowed`): 스킴 `tauri`·`asset`·`about`·`blob`,
  `http(s)` 중 호스트가 `tauri.localhost`·`asset.localhost`·`ipc.localhost` 인 것, 그리고
  **dev 빌드에서만** `build.devUrl` 과 오리진(scheme+host+port)이 같은 URL. 그 외는 전부 거부하고
  `log::warn!` 로 남긴다. `devUrl` 은 릴리스 바이너리의 임베드 config 에도 남지만 Tauri 자신이
  `cfg(dev)` 에서만 그리로 이동하므로, 가드도 `cfg!(dev)` 일 때만 그 오리진을 신뢰한다.
- **`about:`·`blob:` 을 반드시 허용해야 하는 이유 — 정책 함수는 프레임을 구분하지 않는다**:
  wry 0.55.1 의 `wkwebview/navigation.rs` 는 델리게이트에 URL 문자열만 넘기므로 **iframe 의
  네비게이션도 같은 함수를 탄다**. `features/preview/html-preview.tsx` 의 `sandbox=''` blob
  iframe(과 그 초기 문서 `about:srcdoc`/`about:blank`)이 여기서 막히면 HTML 프리뷰가 죽는다 —
  CSP 의 `frame-src 'self' blob:` 과 짝을 맞춘 항목이다. 새 스킴을 허용 목록에 넣기 전에
  "iframe 도 이걸 통과한다"를 먼저 따진다.
- **새 창 요청은 OS 브라우저로 승격**(`open_new_window_externally`): `window.open()` 계열 요청의
  응답은 **항상 `NewWindowResponse::Deny`** 다(앱이 소유하지 않는 두 번째 웹뷰는 만들지 않는다).
  다만 그 URL 이 `infra::external_url::validate_external_url` 을 통과하면
  `tauri_plugin_opener::open_url` 로 OS 브라우저에 넘긴다 — `system_open_external_url` 커맨드와
  **문자 그대로 같은 화이트리스트**(http(s) 전용·제어/스푸핑 문자 금지·userinfo 금지)다.
  그 검증기가 `domain/system/commands.rs` 가 아니라 `taide-infra` 에 있는 이유는, 소비자가 platform
  (navigation_guard)와 domain(system) 둘인데 §2 가 `infra → domain` 참조를 전면 금지하기
  때문이다(도메인이 infra 를 쓰는 정상 방향으로 뒤집었다).
- **main 창을 `create: false` + `from_config` 로 직접 만드는 이유**: `on_navigation`/
  `on_new_window` 는 빌더에만 붙는데 `tauri.conf.json` 이 자동 생성하는 창에는 빌더가 없다.
  그래서 `app.windows[0]` 에 `"create": false` 를 두고(`tauri::app::setup` 이 `create=false` 창을
  건너뛴다) `lib.rs` `.setup` 의 **첫 문장**에서 `WebviewWindowBuilder::from_config` 로 같은 config
  를 그대로 재생한다. 라벨·크기·`backgroundColor`·`visible:false`·Overlay 타이틀바가 전부 config
  에서 오므로 FOUC 정책(`features/window-chrome.md` §2)과 capability 의 라벨 매칭
  (`capabilities/main.json` 의 `"windows": ["main", "editor-*"]`)은 무변경이고, 생성 시점도
  자동 생성 구간과 동일하다(Tauri 는 setup 훅 **직전**에 config 창을 만든다) — 부팅 지연과
  `MAIN_WINDOW_LABEL` 참조 순서에 영향이 없다. 창 생성 실패는 setup 오류로 전파한다.
- **opener 플러그인의 JS 클릭 인터셉터는 끈다**(`tauri_plugin_opener::Builder::new()
  .open_js_links_on_click(false)`): capability 에 `opener:allow-open-url` 이 없어 링크를
  `preventDefault` 하고 아무 데도 열지 않던 죽은 인터셉터였다. 앵커 클릭의 단일 소유자는
  `app/providers/external-link-provider.tsx` 다.

## 5. View(React) 구조 — FSD

컨벤션 fsd.md 를 데스크톱 SPA 에 맞게 적용한다. pages 레이어는 사용하지 않는다(단일 윈도우 앱,
라우팅 없음 — app 이 직접 widgets 를 조립).

```
src/
├── app/                     진입점, 프로바이더(QueryClient, 테마), 전역 레이아웃 조립
├── widgets/                 IPC 를 소비하는 조립 블록 (비즈니스 로직 O — 아래는 대표 예시, 전체 28슬라이스)
│   ├── app-shell/           최상위 셸 조립 (사이드바 | 탐색 | 에디터 영역)
│   ├── app-sidebar/         프로젝트 목록·아이콘 상태·세로 DND
│   ├── editor-area/         pane 트리 렌더 + 탭 바 + 단일 DndContext (탭 DND·5분할 드롭)
│   ├── editor-pane/         Monaco 마운트, git gutter·blame 데코, LSP 세션 연결
│   ├── terminal-pane/       xterm 마운트, flow control, 세션 spawn/attach
│   ├── explorer/            파일 트리(가상 스크롤) + 뷰 전환(파일/검색/Git)
│   ├── search-panel/        전역 검색 결과
│   ├── git-panel/           changes·graph·commit UI
│   ├── diff-pane/           diff 탭 (Monaco DiffEditor)
│   ├── settings-view/       설정 화면
│   ├── command-palette/     ⌘⇧P / ⌘P
│   └── ...                  (그 외: plugin-manager·task-runner·theme-editor·keybindings-editor·
│                             preview-pane·problems-panel·outline-panel·search-editor·file-history·
│                             commit-file-diff·claude-diff-pane·snippet-editor·window-chrome 등)
├── features/                순수 표시 컴포넌트 (props+콜백만 — 비즈니스 로직 X)
├── entities/                IPC 데이터 계층 (도메인별)
│   └── {domain}/
│       ├── {domain}.ipc.ts      invoke 래퍼 (자동 생성 바인딩 사용)
│       ├── {domain}.query.ts    TanStack Query 훅 + queryOptions + 이벤트 구독→invalidate
│       └── {domain}.type.ts     생성된 타입 re-export/파생
└── shared/                  ui(shadcn vendored)·hooks·constants·lib·api(생성 bindings)
    ├── api/bindings.ts      **tauri-specta 생성물** (커밋 대상, 직접 수정 금지)
    ├── lib/monaco/          monaco setup(worker 배선)·테마 파생 — **monaco 접근은 반드시 이 경유**
    ├── lib/lsp/             자체 경량 LSP 클라이언트 + Monaco 어댑터 20종 (ADR-0007)
    └── constants/           query-key(중앙 관리)·platform·terminal
```

- 서버 상태 = "Rust 상태"로 치환해 query.md 컨벤션을 그대로 적용한다: queryOptions 팩토리,
  QUERY_KEY 중앙 관리, 이벤트 수신 시 invalidate (ADR-0008).
- view 전역 상태(zustand)는 순수 UI 상태(드래그 중 표시 등)가 실수요를 증명할 때만 도입한다(frontend.md §6).

### 5.1 `widgets` ↔ `features` 배치 기준 (T1-F, fsd.md §2.1 데스크톱 특화)

eslint `no-restricted-imports` 는 import **방향**만 강제하고 레이어의 **성격**은 검사하지 않는다 —
그 결과 감사(C1)에서 explorer·problems·search·outline·settings·plugin·snippet·tab 8개 슬라이스가
두 레이어에 걸쳐 있는 것이 드러났다. 이 프로젝트에서 두 레이어를 가르는 유일한 판정 기준은
**query/mutation/IPC 소유 여부**다(fsd.md §2.1 의 "props 로만 받는가" 질문을 이 앱의 "Rust 상태 =
서버 상태" 축으로 구체화한 것).

- **`widgets` 로 판정**: 컴포넌트/훅 자신이 `useQuery`·`useMutation`·`useSuspenseQuery`(entities
  query 훅 포함) 또는 `entities/*.ipc.ts` 의 invoke 래퍼를 **직접 호출**한다. 전역 키맵·전역
  이벤트 브리지 구독처럼 IPC 를 감싸는 부수효과도 여기 포함된다.
- **`features` 로 판정**: 위 어느 것도 호출하지 않고, 데이터·콜백을 전부 **props 로만** 받는다.
  타입 import(`@entities/*/*.type.ts`, `@shared/api/bindings`)는 이 판정에 영향을 주지 않는다 —
  entities 는 두 레이어 모두에서 항상 허용된다(§2 fsd.md 매트릭스).
- **`shared` 로 강등**: React 에 의존하지 않는 순수 함수/로직이면서 소비처가 widgets 한 곳뿐이어도
  `shared/lib` 이 맞다 — `features` 는 "컴포넌트" 레이어이므로 React 와 무관한 유틸을 두는 자리가
  아니다.
- **판정 결과 예시(2026-08-18 이동)**: `agent-hooks-project-row`/`-list`·`agent-cli-status-row`·
  `use-zen-mode`(조회+변이+IPC 소유) → `widgets/settings-view`·`widgets/app-shell` 로 승격.
  `tab-context-menu`·`sortable-tab`·`file-tree`·`problems-panel`·`search-panel`·`outline-panel`·
  `plugin-list-body`·`vsix-import-grammars-section`·`snippet-entry-editor`·`snippet-file-list`
  (props+콜백만) → `features/*` 로 강등. `vsix-theme-import`·`selection-line-range`(React 무관
  순수 함수) → `shared/lib` 로 강등. `snippet-entry-editor` 는 `widgets/snippet-editor/
  snippet-draft.ts`(드래프트 변환 타입·순수 함수, React 무관)를 import 하고 있어 그 파일까지
  `shared/lib/snippet-draft.ts` 로 함께 강등해야 features→widgets 역참조 없이 이동이 성립했다
  (Phase D 접합부 수정 — F 1차 실행 시 이 종속을 확인하지 못해 10개 강등 대상 중 이 한 파일만
  누락돼 있었다).
- 판정이 애매하면 **더 아래 레이어**를 고른다(fsd.md §2.1) — 승격은 실제 사용처가 생겼을 때 언제든
  가능하지만, 강등은 소비처 전체를 다시 훑어야 한다.

## 6. 수명주기·누수 방지 규칙 (전 기능 공통)

1. **구독은 생성자와 해제가 한 곳에**: `listen()` 은 반드시 unlisten 을 useEffect cleanup 에서 호출.
   커스텀 훅 `useTauriEvent(name, handler)` 하나로 표준화하고 직접 listen 을 금지한다.
2. **무거운 객체는 dispose 의무**: Monaco model/editor, xterm 인스턴스는 소유 위젯 unmount 시 dispose.
   전역 캐시에 남기는 경우(모델 재사용) LRU 상한과 방출 정책을 명시한다(`features/editor.md`).
3. **Rust 자원은 세션 구조체가 소유**: pty·LSP·watcher 는 세션 drop 시 자식 프로세스 종료까지 보장해야 한다
   (Drop 구현 + 명시적 shutdown 경로 이중화). 현재 일반 LSP wait/reader/callback·PTY spawn/worker의 정상 완료 대기와 Drop 종료 요청, 설치 직접 child/부모 선종료 그룹 정리·정상 종료 coordinator는 자기 fixture로 검증했다. Drop의 종료 요청은 실제 join과 다르며 partial spawn/OS 오류·직접 native Exit·runtime 오류/그룹 이탈 자손 및 실제 native 종료의 전체 소유권 gate는 M6 미완료 항목이다.

   **§6.3 `project_close` 자원 회수 목록 (정본)** — 프로젝트 종료 시 회수되는 전체 목록이다.
   T1-I(2026-08-19)부터 각 항목의 회수는 그 도메인의 `capability.rs` `detach` 가 소유하고,
   `project_close` 는 `ProjectCapabilities::detach_all` 순회만 한다(§3). 회수 **순서**는 lib.rs
   `project_capabilities()` 의 등록 순서가 계약이다. 새 도메인이 프로젝트 수명에 묶인 상태를
   추가하면 capability 구현 + lib.rs 등록 + 이 표 세 곳에 함께 추가한다.

   부착 쪽이 2단(build/register)으로 나뉜 뒤에도(§3.1) 이 표의 대칭 요구는 그대로다 — build 가
   만든 자원은 **commit 된 것만** `AppState` 에 들어가고(커밋되지 않은 build 결과는 drop 되어 그
   자리에서 소멸), 들어간 것은 전부 여기 등재된 `detach` 가 회수한다. 새 capability 를 추가할 때
   "build 에서 만든 것 ↔ detach 에서 회수하는 것" 이 1:1 인지 확인한다.

   | 자원 | 회수 방법 | 실패 시 |
   |---|---|---|
   | `dirty_layouts`/`layouts` | 남은 dirty 레이아웃 동기 flush 후 두 맵에서 제거 | 미저장 레이아웃 유실 |
   | `watchers`/`git_watchers` | 맵에서 제거 (핸들 drop이 watcher 스레드 종료) | 닫힌 프로젝트 파일 변경을 계속 감시 |
   | pty 세션 | `TerminalStore::kill_project` | 프로세스+fd+스레드가 앱 종료까지 잔존 |
   | `GitStore` (projectId→repo_root 캐시 · `git_status` 결과 캐시) | `GitStore::remove` | 재오픈 시 옛 repo_root·옛 status 부활 + 메모리 잔존 |
   | `TreeStore` (트리 캐시) | `TreeStore::remove` | 재오픈 시 옛 디렉터리 목록 부활 + 메모리 잔존 |
   | asset 프로토콜 접근 | **회수 불필요** — 아래 참고 (T1 2차, X1#7 근본 수정) | 해당 없음 |

   **`git_status` 결과 캐시 (사용성 배치 4 · 계약 §C.2-6 ③)**: taide-git의 `GitStore`가 소유하는 상태 캐시는
   프로젝트별 슬롯 하나에 마지막 `GitStatus` 와 그것을 읽은 시각·무효화 세대를 담는다. 슬롯은
   `git_status` 가 처음 조회될 때 생기고 위 표의 `GitStore::remove` 가 회수한다 — attach 쪽 짝은
   없다(§3.1 의 "build 에서 만든 것 ↔ detach 에서 회수하는 것" 1:1 요구는 build 가 만드는 것이 없어
   자동 충족). 무효화 축은 프론트가 `QUERY_KEY.GIT.STATUS` 를 다시 묻는 축과 **정확히 같다**:
   `fs:changed`(d-44 워크트리 축) · `git:status-changed` · `git:refs-changed`. 세 이벤트 구독은
   `GitStore::ensure_invalidation_listeners`가 받은 콜백을 공유 저장소당 1회 실행하며 Tauri adapter가 repo root·캐시 조회 전에 지연 등록한다(부팅 복원 경로가
   capability 순회를 타지 않으므로 capability 훅으로는 놓친다), 이 도메인이 직접 발행하는 지점
   (`emit_status_changed`·`emit_refs_changed`·`git/watch.rs` 워처 콜백)은 emit **이전에** 같은
   무효화를 한 번 더 호출해 순서를 보장한다(`Manager::emit` 은 Rust 리스너보다 웹뷰에 먼저 넘긴다).
   어떤 워처도 보고하지 못한 변경(FSEvents 드롭·attach 지연 구간·네트워크 마운트)에 대비한 상한은
   **2초 TTL** 이고, 계산 중 무효화가 들어오면 그 결과는 저장하지 않는다(세대 비교).
   슬롯을 회수한 뒤 같은 ID로 새 슬롯을 만들면 identity가 달라 이전 계산은 저장하지 않는다.

   **asset 프로토콜 재구현 완료 (T1 2차, X1#7)**: 1차 배치는 이 항목을 Tauri 내장
   `asset_protocol_scope()`(`scope::fs::Scope`)의 allow/forbid 가 **둘 다 추가 전용**이라(되돌릴
   API 없음, `forbid`가 `allow`보다 항상 우선) `project_close`에서 단순히 `forbid_directory(root)`를
   호출하면 "같은 폴더를 다시 열어도 영원히 asset 을 못 읽는" 새 회귀를 만든다는 이유로 보류했다.
   2차 배치는 근본 수정을 실행했다 — 현재 `platform::asset_protocol::respond`가
   `register_uri_scheme_protocol("asset", ...)`(`lib.rs`, `Builder` 체인의 `.setup()` 이전)으로
   asset 스킴 자체를 앱이 직접 재구현해 서빙한다. Tauri는 같은 이름("asset")의 스킴이 이미
   등록돼 있으면 내장 핸들러를 건너뛴다(`tauri-2.11.5/src/manager/webview.rs`
   `prepare_pending_webview`의 `if !registered_scheme_protocols.contains(&"asset".into())`로
   직접 확인) — `convertFileSrc`(`@tauri-apps/api/core`)도 `tauri.conf.json`의
   `asset:`/`http://asset.localhost` CSP 소스도 스킴 **이름**에만 의존하므로 **무변경**이다.
   `is_allowed` 판정은 append-only 스코프 대신 `root_guard::resolve_owning_project`로 **현재 열린
   프로젝트 집합**(`AppState::projects`)을 매 요청마다 조회한다 — `/__taide/file`(remote 라우트)이
   이미 쓰는 같은 함수라 경로 봉쇄 보장이 두 서빙 경로에서 동일하다. 프로젝트가 닫히면
   `AppState::projects`에서 즉시 제거되므로(§6.3 위 표의 기존 `project_close` 흐름), 별도의
   "회수" 단계 없이 **다음 요청부터 자동으로 거부**된다 — 이것이 표에 "회수 불필요"로 적은 이유다.
   `platform::asset_protocol::respond` 자신은 `AppState`를 직접 조회하지 않고 열린 프로젝트 맵을
   파라미터로 받는다(`infra::root_guard`의 다른 함수들과 같은 모양) — `AppHandle`/`State` 조회는
   `lib.rs`의 `register_uri_scheme_protocol("asset", ...)` 등록 클로저 한 곳에만 있고, 그 결과를
   `respond`에 넘긴다. 현재 구현은 `platform/asset_protocol.rs`가 소유하고 기존
   `infra/asset_protocol.rs`는 재수출 facade다. `platform::` 구현은 `crate::state`에 직접 의존하지 않아
   `respond`도 실제 `AppHandle` 없이 단위 테스트할 수 있다(감사 지적 반영, 2026-08-19).

   `Range` 요청(비디오/오디오 탐색)은 Tauri 벤더 소스의 `tauri::protocol::asset::get_response`가
   `pub(crate)`(애플리케이션 코드에서 접근 불가)라 알고리즘만 읽고 처음부터 재작성했다 — 단일
   range·`RANGE_CHUNK_LIMIT`(1000KB) 상한, 확장자→MIME 매핑, 슬라이스 읽기는
   `domain::remote::serving::file_range`와 동일 로직이 필요해 처음에는 두 파일에 그대로
   중복시켰으나, 감사에서 그 중복 로직 자체에 버그(뒤집힌 `Range: bytes=500-100` 같은 요청을
   거부하지 않아 길이 계산이 언더플로하는 결함)가 있는 것으로 드러나 **`infra::range_file`
   공유 모듈로 추출했다**(2026-08-19) — `RANGE_CHUNK_LIMIT`·`RANGE_RESPONSE_CSP`·`extension_mime`·
   `parse_range`(언더플로 수정 + `bytes=-N` 접미 범위를 RFC 7233 §2.1 대로 파일의 **마지막** N
   바이트로 읽는 수정 포함 — d-50 S6, 2026-08-29)·`read_slice`를 두 파일이 공통으로 import 한다(공유 방향은
   `domain::remote::serving`(도메인)→`infra::range_file`(인프라)로, 기존에 이미 있던
   `domain::remote::serving`→`infra::root_guard` 참조와 같은 방향). `extension_mime`에는 이
   추출 과정에서 `m4v`(프론트 `preview-kind.ts`가 이미 video 로 분류하는 확장자) 매핑도 함께
   추가했다 — 이전에는 두 서빙 경로 모두 `.m4v`를 `application/octet-stream`으로 응답해
   `X-Content-Type-Options: nosniff`와 겹치면 재생이 막힐 수 있었다.

   응답에는 `/__taide/file`과 동일하게 `Content-Security-Policy`(`default-src 'none';
   script-src 'none'; style-src 'none'; sandbox`)·`X-Content-Type-Options: nosniff`·
   `Cache-Control: no-store`를 부여한다(`no-store`도 감사 반영 — 닫은 프로젝트의 파일이
   webview HTTP 캐시에 남아 핸들러를 거치지 않고 재생되는 경로를 막는다, `/__taide/file`과
   동일한 방어). 내장 Tauri asset 핸들러가 모든 응답에 붙이는
   `Access-Control-Allow-Origin`(요청 오리진)과 `Range` 응답의
   `Access-Control-Expose-Headers: content-range`는 의도적으로 생략했다 — 이 핸들러의 현재 유일한
   소비처(`preview-pane.tsx`의 `<video>`/`<audio src={convertFileSrc(...)}>`)는 같은 오리진
   엘리먼트 `src`로만 로드되어 CORS 프리플라이트도, `fetch`/XHR로 응답 헤더를 읽는 경로도 타지
   않는다. `UriSchemeContext`는 내장 핸들러의 `window_origin`에 해당하는 값을 노출하지 않아,
   향후 `fetch()`로 `asset://`를 직접 호출하는 소비처가 생기면 그때 헤더 복원이 필요하다
   (`platform::asset_protocol` 모듈 doc에 동일 내용 기록).

   **KNOWN ISSUE(실기 미검증)**: 에이전트는 앱을 실행할 수 없어 이 핸들러를 실제 webview 로
   검증하지 못했다 — `<video>`/`<audio>` 탐색(Range 응답)이 실제로 매끄러운지, WKWebView/WebView2가
   커스텀 `register_uri_scheme_protocol` 핸들러를 내장 핸들러와 동일하게 라우팅하는지, CORS 헤더
   생략이 실제로 무해한지는 코드 리딩과 단위 테스트(`platform::asset_protocol::tests`,
   `infra::range_file::tests`)로만 확인했다. 실기 확인 항목은
   `docs/quality-assurance/2026-08-11-qa6-checklist.md` "감사 T1 정비 2차 재검" 절 참고.

   **`project_close`에 묶이지 않는 별개의 세션 수준 자원 회수** (같은 배치에서 함께 수정, 프로젝트
   종료가 아니라 각 자원 자체의 생명주기에 걸림):
   - `infra::lsp_proc::LspProcHandle::kill` — 기존에는 `AtomicBool` 플래그만 세우고 실제 kill은
     `spawn`의 백그라운드 폴링 태스크(최대 50ms 지연)에 위임했다. 앱 종료(`RunEvent::Exit`)
     핸들러가 이 함수를 동기 호출한 직후 `std::process::exit`로 프로세스가 즉시 종료되므로, 그
     백그라운드 태스크가 스케줄될 기회조차 없이 언어 서버가 고아 프로세스로 남을 수 있었다. 이제
     `kill()` 자신이 `sysinfo`로 PID를 동기적으로 kill한다 (`infra::pty::PtySession::kill`과 동일한
     패턴).
   - `domain::lsp::commands`의 `restart_count` — 크래시마다 증가만 하고 명시적 `lsp_restart`
     외에는 리셋되지 않아, 여러 날에 걸친 세션에서 서로 무관한 크래시 3회가 누적되면 이후
     영구적으로 자동 재시작이 멈췄다. 재시작된 프로세스가 `LSP_RESTART_HEALTHY_RESET_MS`(30초)
     동안 교체 없이 살아있으면 `restart_count`를 0으로 리셋한다.
   - `domain::lsp::commands::LspInstallStore` — `begin`/`finish` 쌍이 `.await` 정상 반환 경로에만
     의존해, 설치 퓨처가 패닉하거나 태스크가 드롭되면 `server_id`가 영구히 "설치 중"으로 잠겼다.
     `LspInstallGuard`(Drop)로 이중화했고 현재는 요청 guard Drop이 취소를 요청한다. 감독된 extraction worker의 `LspInstallLease`까지 모두 Drop된 뒤에만 슬롯을 해제한다.
     요청 종료와 실제 worker 종료를 구분하며 download 파일 create/write/flush 및 toolchain child/reader도 실제 소유자가 슬롯을 보유한다. 정상 ExitRequested coordinator는 감독 task 완료와 설치 마지막 lease를 기다린다. native 직접 Exit 경로의 전체 작업/자손·실제 앱 회수는 M6 미완료 gate다.
   - `infra::shell_integration`이 만드는 zsh/bash 임시 디렉터리 — 주입된 스크립트 자신의
     `rm -rf` 한 줄에만 의존했고, 셸이 그 줄에 도달하지 못하면(크래시·조기 종료) OS 임시 디렉터리
     아래 영구히 남았다. 이제 `PtySession`이 생성 시점의 경로를 들고 있다가 자신의 `Drop`에서
     결정적으로 제거한다.
   - `infra::shell_integration`이 그 임시 디렉터리 안에 쓰는 `.zshenv`/`.zprofile`/`.zshrc`/
     `init.bash`(T1 2차, R2#11) — 기존 `std::fs::write`는 프로세스의 기본 umask(보통 `022`)를
     그대로 물려받아, 다른 로컬 사용자도 읽을 수 있는 공용 OS 임시 디렉터리 아래 파일이 세션이
     살아있는 동안 그 umask 권한(보통 world-readable)으로 노출됐다. `infra::persist::
     write_private_atomic`(소유자 전용 `0o600`, temp-then-rename 원자적 쓰기)로 교체했다 —
     이 함수는 T0 배치에서 settings/secret 파일에 이미 쓰던 것을 재사용한 것이라 신규 유틸이
     아니다.
4. **이벤트 페이로드는 소형 유지**: 대형 데이터(파일 내용, diff 본문)는 이벤트에 싣지 않고
   "변경됨" 신호만 보내 query 로 다시 읽게 한다. 스트림 데이터만 Channel 예외.
5. 각 `features/*.md` 문서는 "수명주기" 절에서 이 규칙의 해당 기능 적용을 구체화한다.
6. **프론트 모듈 스코프 싱글톤(레지스트리/브리지)은 소유권 범위·수명 종료 시점을 선언한다**
   (감사 클러스터 C3/C4, `docs/quality-assurance/2026-08-18-architecture-audit.md` §C3·§C4 —
   `docs/acknowledge/2026-08-19-audit-t1-batch3-contract.md` §1.2·§1.3 로 정비).

   **§6.4 소유권 계약(정본)** — `shared/lib`·`entities/*` 에 두는 모듈 레벨 `Map`/`Set` 기반
   레지스트리·브리지는 만들 때 아래 두 질문에 답해야 한다. 새 레지스트리를 추가할 때도 이 표에
   함께 등록한다.

   - **소유권 범위**: 이 레지스트리의 키(엔트리)는 무엇 하나에 묶이는가 — 탭? 프로젝트? 창(OS
     라벨, `owner`)? 프로세스 전체(앱 수명)? 창별로 독립된 JS 모듈 상태(각 Tauri 창은 별도
     webview·별도 모듈 인스턴스)라 "창 스코프"는 코드를 전혀 안 써도 자동으로 성립하지만, "탭
     스코프"·"프로젝트 스코프"는 그 대상이 사라지는 시점을 레지스트리가 직접 알아채야 한다.
   - **수명 종료 시점**: 그 스코프가 끝났다는 신호를 레지스트리가 어떻게 받는가 — 명시적 해제
     호출(`unregister`/`release`, 보통 `useEffect` cleanup)만으로 충분한가, 아니면 그 신호가
     누락될 수 있는 경로가 있어(탭이 accept/reject 없이 닫힘, 프로젝트가 닫힘, 앱이 재시작 없이
     장시간 실행)와 함께 **TTL·용량 상한·`projectClosed`/`app/providers` 이벤트 구독** 중 최소
     하나로 뒷받침해야 하는가.

   | 레지스트리/브리지 | 소유권 범위 | 수명 종료 시점 |
   |---|---|---|
   | `entities/editor/reveal-registry.ts` | pending reveal 요청(**탭 id** 키 — 그 탭의 에디터 마운트 대기, d-66) | 소비(`consumePendingReveal(tabId, editor)`) 또는 `REVEAL_PENDING_TTL_MS`(5s) 만료 — 탭/프로젝트 이벤트 구독 없음, TTL 단독 |
   | `entities/editor/open-with-registry.ts` | 파일 경로별 "이 확장자를 어떤 뷰어로 열지" 오버라이드 | 탭 닫기 시 다른 곳에 안 남아있으면 즉시 해제(`useCloseTab` → `setOpenWithOverride(path, null)`) + 프로젝트 종료 시 다른 프로젝트에 안 남은 경로만 일괄 해제(`ipc-sync-provider.tsx`의 `projectClosed` → `pruneOpenWithOverrides`) — 두 이벤트 모두 놓친 경로(경로는 탭/프로젝트에 1:1 로 안 묶여 재사용 가능)만 `OPEN_WITH_OVERRIDE_MAX_ENTRIES`(200) LRU-by-write 상한이 뒷받침 |
   | `entities/ide/claude-diff-registry.ts` | requestId 별 미해결 Claude diff 요청 | accept/reject(`removePendingClaudeDiff`) 또는 그 탭이 레이아웃에서 실제로 사라짐(`claude-diff-pane.tsx` unmount 시 `queryClient.getQueryData(LAYOUT.DETAIL)` 로 재확인 — `projectClosed` 의 `PROJECT_SCOPED_KEYS` 캐시 제거가 이 재확인을 간접적으로 성립시킨다) |
   | `shared/lib/bridge/terminal-write-bridge.ts` | 탭별 pty 쓰기 큐 + 핸들러 슬롯 | 정상 해제(`register`/`unregister`) 시 즉시 회수, 또는 `TERMINAL_WRITE_QUEUE_TTL_MS`(30s) 경과분을 다음 호출에서 기회적으로 스윕(`sweepStaleSlots`) — 탭 종료 이벤트 구독 없음, TTL 단독 |
   | `entities/lsp/lsp-session-flush-registry.ts` + `entities/lsp/lsp-session-registry.ts` | 프로젝트/창/서버/root 별 LSP 세션(`sessionsByKey`) | 참조 카운트 0 도달 후 `LSP_SESSION_DISPOSE_GRACE_MS` 유예, 또는 `projectClosed` 이벤트(`ipc-sync-provider.tsx` → `flushLspSessionsForProject`)로 유예 없이 강제 회수, 또는 앱 종료(`HotExitFlushProvider` → `flushAllLspSessionDisposals`) |
   | `shared/lib/bridge/fire-and-forget-bridge.ts`·`shared/lib/bridge/external-store-bridge.ts` 로 만든 팩토리형 브리지 12+종 | 팩토리 자체는 무상태 — 소유권 범위는 **호출부가 정의**(대개 프로세스 전체, 창별 모듈 인스턴스로 자동 격리) | 팩토리는 구독자 0 정책(`emptyPolicy`)만 제공, TTL/용량은 호출부 책임(예: terminal-write-bridge 의 레이어) |
   | `entities/agent/agent-wait-marker-registry.ts` | tabId 별 외부 오픈 대기 마커 | `useCloseTab` 해제 경로 + `clearStaleWaitMarkersOnStartup`(앱 부팅 시 잔존분 정리) |

   **앱 수명 부수효과(이벤트 구독·전역 상태 동기화)는 조건부 렌더 위젯이 아니라 상시 마운트
   프로바이더(`app/providers/*`)가 소유한다(C4)** — `AppSidebar`·`StatusBarContent`·
   `KeybindingsEditor` 다이얼로그처럼 Zen 모드나 보조 창 분기에 따라 마운트/언마운트되는
   위젯에 `useAgentStateSync()`류 훅을 직접 두면, 그 위젯이 숨겨지는 동안(Zen 모드) 또는 애초에
   렌더되지 않는 창(보조 창)에서 부수효과 자체가 사라진다. 정답은 "표시"(조건부 위젯)와
   "구독"(상시 프로바이더)의 소유자를 분리하는 것 — `app/app.tsx` 의 프로바이더 트리에 상시
   마운트하고, 조건부 위젯은 그 프로바이더가 채운 캐시/스토어를 읽기만 한다.

   | 부수효과 | 이전 소유(사라지던 조건) | 현재 소유(상시 프로바이더) |
   |---|---|---|
   | IDE(Claude Code) 프로토콜 3종 + 상태 동기화 + 진단 push | `StatusBarContent`(Zen + 상태바 숨김) | `app/providers/ide-sync-provider.tsx`(메인 창 전용 — 원격 IDE 프로토콜은 창마다 중복 처리하면 안 되므로 보조 창엔 미마운트, `app.tsx` 상단 doc 참조) |
   | 에이전트 상태 push 동기화 | `AppSidebar`(Zen) | `app/providers/agent-state-sync-provider.tsx`(메인+보조 창 전부) |
   | monaco 키바인딩 오버라이드 적용 | `KeybindingsEditor` 다이얼로그(보조 창 전체) | `app/providers/keybindings-runtime-provider.tsx`(메인+보조 창 전부 — 다이얼로그 자신도 이 프로바이더가 controlled 로 렌더) |
   | 키맵 에디터 열기 브리지 구독 | 동상 | 동상(`keybindings-runtime-provider.tsx`) |
   | `QUERY_KEY.LSP.SESSIONS` 무효화(`lsp:session-status-changed` 이벤트) | 없음(무효화 지점 0건) | `app/providers/ipc-sync-provider.tsx`(`useLspSessionsQueryInvalidationSync`, 메인+보조 창 전부) |

## 7. 플랫폼 분기 격리 (NFR-6)

- OS 분기는 Rust `infra/` 안에서만 한다(`#[cfg(target_os)]`): 셸 감지, 프로세스 트리 조회,
  경로 처리, ConPTY. view 와 domain 로직은 플랫폼 중립.
- view 의 유일한 분기는 modifier 키 표기(cmd/ctrl)로, `shared/constants/platform.ts` 한 곳에 둔다.

## 8. 프로세스 구성 요약

- TAIDE 앱 프로세스(Rust) 1개 — WebView 1개(멀티윈도우는 추후).
- 자식 프로세스: 프로젝트별 pty N개, 언어별 LSP 서버 N개(가능하면 프로젝트 간 공유 —
  `docs/research/lsp-servers.md` 결과에 따름).
- `taide` CLI: single-instance 로 기존 앱에 위임(deep link/IPC). `--wait` 는 탭 닫힘 시그널까지 블록
  (`features/agent-integration.md`).
