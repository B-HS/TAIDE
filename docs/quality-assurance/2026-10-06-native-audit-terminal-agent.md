# Native 전환 감사 — 터미널·에이전트·IDE 서버·태스크 (2026-10-06)

상태: 읽기 전용 감사 완료. 빌드·테스트는 실행하지 않았으며 모든 판정은 실제 파일과 호출 체인 검색에 근거합니다.

## 1. 범위

### 1.1 읽은 TS 경로

- `src/widgets/terminal-pane/` 전체(`terminal-session.tsx`, `terminal-pane.tsx`, `terminal-flow-control.ts`, `terminal-replay-budget.ts`, `terminal-clipboard-availability.ts`, `terminal-split-availability.ts`, `pending-terminal-input.ts`)
- `src/widgets/task-runner/task-runner-dialog.tsx`
- `src/features/terminal/` 전체(`terminal-view.tsx`, `terminal-context-menu.tsx`, `terminal-osc133.ts`, `terminal-file-link.ts`)
- `src/entities/{terminal,agent,task,ide,notification}/` 전체(테스트 제외)
- 위 엔티티의 실제 소비처: `src/app/providers/{native-notification-provider,ide-sync-provider,agent-external-open-provider}.tsx`, `ipc-sync-provider.tsx` 425~480행, `src/widgets/claude-diff-pane/claude-diff-pane.tsx`, `src/widgets/editor-pane/use-editor-ide-selection.ts`, `src/widgets/editor-area/editor-area.tsx` 225~338행, `src/widgets/editor-area/pane-tab-bar.tsx`(에이전트 부분), `src/widgets/app-sidebar/app-sidebar.tsx`(에이전트 부분), `src/features/project/project-icon-button.tsx`, `src/features/window/status-bar.tsx`(IDE 부분), `src/shared/lib/{agent-status-text,xterm-theme}.ts`, `src/shared/lib/bridge/terminal-write-bridge.ts`, `src/shared/constants/terminal.ts`
- inventory: `2026-09-28-ts-view-inventory.md` 45행, `2026-09-28-ts-overlay-inventory.md` 23·26행, `2026-09-29-ts-provider-inventory.md` 16·18행

### 1.2 읽은 native 경로

- `native/taide-native-terminal/src/{lib.rs,input.rs,session.rs}` 전체, `vendor/alacritty-terminal/src/term/mod.rs` 784~801행
- `native/taide-native-app/src/terminal_surface.rs` 운영 코드(1~4940행) 구역별: 1~1340, 1661~2130, 2412~2545, 2933~4940. 4941행 이후는 테스트 모듈이라 이름 목록만 확인했습니다. 1340~1660(메뉴 키보드 탐색), 2130~2411·2545~2932(포인터 캡처·포커스 순서 보존)는 함수 시그니처와 호출 관계만 확인했습니다.
- `native/taide-native-app/src/{terminal_host.rs(221~960), terminal_tabs.rs, terminal_dispatch.rs, terminal_environment.rs, terminal_file_links.rs, terminal_links.rs, terminal_fonts.rs(1~140), terminal_frames.rs, terminal_ruler.rs, terminal_settings.rs, agent-hooks.rs, ide-server.rs(120~300), ide-tools.rs(100~400), status-ide.rs(1~200), remote-agents.rs, application-ports.rs, bootstrap.rs, events.rs, shell_keymap.rs(1~200), lib.rs, main.rs}`
- `native/taide-native-app/src/application.rs` 40~58, 196~455, 614~702, 3364~3413, 4840~4930행과 터미널·IDE·에이전트 관련 grep 결과 전부
- `native/taide-native-app/src/host.rs` 명령 목록과 707~963, 1176~1191행
- `crates/taide-runtime/src/{terminal_env.rs, agent_actions.rs(128~220)}`, `agent_host.rs` 공개 API 목록
- `crates/{taide-terminal,taide-agent,taide-ide,taide-task,taide-notification}/src` 는 파일 구성과 native 호출부 기준으로 확인했습니다(본문 정독은 하지 않았습니다. 6장 참조).

### 1.3 실제 앱 연결 확인 결과(요약)

`main.rs` 40행 → `NativeApplication::new`(`application.rs` 197행) → `terminal_tabs::Tabs::new`(267행) + `HostBridge::connect_with_application_ports`(302행) → pane 렌더에서 `terminal_views.show_with_keymap`(4856행) → `HostCommand::AttachTerminal`(`terminal_surface.rs` 3217행) → `host.rs` 853행 → `Tabs::attach` → `Hub::spawn` → `terminal_actions::pty_spawn`(`terminal_host.rs` 729행). 실제 PTY 가 앱 실행 경로에서 생성·표시됩니다.

## 2. 기능 대응표

판정: done / partial / unwired / missing / n/a. effort 는 native 에 남은 작업량입니다(S 반나절 이하, M 1~2일, L 3~5일, XL 1주 초과).

### 2.1 터미널 표면

| 기능 | TS 근거 | native 상태 | native 근거 | 빠진 것 | effort |
|---|---|---|---|---|---|
| PTY 생성·부착(측정 크기, 기본 셸·cwd, env) | `terminal-session.tsx` 145~193 | done | `terminal_surface.rs` 3203~3222, `terminal_tabs.rs` 397~479, `terminal_host.rs` 706~854 | 없음 | - |
| 탭 전환 후 세션 유지·스크롤백 보존 | `terminal-session.tsx` 304~341(attach replay) | done | 그리드가 세션 소유(`session.rs` 42~53), 뷰는 `hub.get` 으로 재결합(`terminal_surface.rs` 3203, 3358~3370) | 없음(replay 자체가 불필요한 구조) | - |
| 셀 렌더링(16/256/truecolor, inverse, dim, hidden, 밑줄, 취소선, wide char) | `terminal-view.tsx` 263~285(xterm+WebGL) | partial | `terminal_surface.rs` 139~206, 4748~4822 | 볼드 굵기·이탤릭 글꼴 없음(`terminal_fonts.rs` 는 `Weight::NORMAL` 만 로드, 175·187·200행), 밑줄 변형(double/curly/dotted/색) 단일 선 처리(4805행), `Color::Indexed` 0~7 의 볼드 밝기 미적용(187~189행) | M |
| 테마 색·실시간 테마 변경 | `terminal-session.tsx` 419, `xterm-theme.ts` | done | `terminal_surface.rs` 109~137, `application.rs` 583·599·3330·3346 | 없음 | - |
| 글꼴 크기·패밀리(실시간) | `terminal-view.tsx` 217~233 | done | `application.rs` 3380~3404, `terminal_fonts.rs` 49~127 | fallback 목록이 macOS 글꼴뿐(`terminal_fonts.rs` 15행) | - |
| 리거처 | 해당 addon 미사용 | n/a | - | TS 도 리거처를 그리지 않습니다(`terminal-view.tsx` addon 목록 6~10행) | - |
| WebGL 렌더러·컨텍스트 손실 복구 | `terminal-view.tsx` 318~328 | n/a | - | 웹 렌더러 전용 | - |
| 커서 스타일(bar/block/underline)·blink | `terminal-view.tsx` 247~257 | done | `terminal_settings.rs`, `terminal_host.rs` 611~622, `terminal_surface.rs` 1162~1190, 4833~4902 | 없음 | - |
| 스크롤(휠·트랙패드, Shift+PageUp/Down, 입력 시 하단 복귀, alt screen 에서 화살표 변환) | xterm 기본 | done | `terminal_surface.rs` 814~919, 4168~4169, 4021~4024 | 없음 | - |
| 스크롤바(드래그 가능한 thumb) | xterm 6.0.0 viewport 스크롤바(`node_modules/@xterm/xterm/package.json` version 6.0.0, `overviewRuler.width` 14) | missing | `terminal_ruler.rs` 125~149 는 ruler 도색만, `terminal_surface.rs` 3985~3990 은 폭 예약만 | thumb 표시·드래그·트랙 클릭. 검색어: `scrollbar`, `thumb`, `slider` | S |
| 스크롤백 줄 수 설정의 실행 중 세션 반영 | `terminal-view.tsx` 241~245 | partial | `terminal_tabs.rs` 424~428(spawn 시점 history 고정) | 실행 중 세션의 표시 버퍼 크기 변경 | S |
| 리사이즈(fit, PTY resize, 측정 불가 크기 거부) | `terminal-view.tsx` 88~99, 336~341 | partial | `terminal_surface.rs` 3345~3357, 3981~4003 | 포커스가 있을 때만 resize 를 보냅니다(3345행 `response.has_focus()`). 측정 불가 크기에서는 배경도 그리지 않고 반환합니다(3187~3189행) | S |
| 선택(드래그, 더블클릭 단어, 트리플클릭 줄, Shift 확장, Alt 블록, 가장자리 자동 스크롤, 출력 중 선택 유지) | xterm 기본 | done | `terminal_surface.rs` 578~645, 921~1120, 4405~4516 | 없음 | - |
| 복사(Cmd+C·메뉴)·붙여넣기(Cmd+V·메뉴, bracketed paste, CRLF 정규화) | `terminal-pane.tsx` 158~168, `terminal-view.tsx` 421 | done | `terminal_surface.rs` 4148~4161, 1944~1956, 2073~2117, `input.rs` 310~344, `host.rs` 804~824 | 없음 | - |
| 전체 선택(Cmd+A·메뉴) | `terminal-view.tsx` 419 | done | `input.rs` 407~411, `terminal_surface.rs` 4170~4182, 1957~1961 | 없음 | - |
| Clear(메뉴) | `terminal-view.tsx` 420 | done | `terminal_surface.rs` 1962~1966, vendor `term/mod.rs` 784~801 | 없음 | - |
| 키 인코딩(화살표·Fn·Ctrl·app cursor·Tab·Backspace) | xterm 기본 | done | `input.rs` 351~420, `terminal_surface.rs` 4243~4283 | kitty 키보드·UTF-8 마우스 모드는 오류 반환(`input.rs` 303~305, 352~354) | - |
| Shift+Enter → LF | `terminal-view.tsx` 61~63, 311~316 | done | `input.rs` 402 | 없음 | - |
| Option-as-Meta(`macOptionIsMeta`) | `terminal-view.tsx` 271 | unwired | 인코더는 Alt 문자에 ESC 를 붙입니다(`input.rs` 193~203). 그러나 표면은 Ctrl/Cmd 가 있을 때만 문자 키를 넘깁니다(`terminal_surface.rs` 4271행) | Alt 단독 문자 키를 `Key::Character` 로 전달하고 중복 `Event::Text` 를 억제. 검색어: `option_is_meta`, `macOption`, `meta` | S |
| IME 조합(preedit 표시, commit, 조합 중 키 억제, 후보창 위치) | `terminal-view.tsx` 343~381 | done | `terminal_surface.rs` 4041~4082, 4903~4925 | preedit 이 배경·밑줄 없이 셀 위에 겹쳐 그려집니다(4904~4912행) | - |
| 마우스 리포팅(X10/VT200/drag/any, SGR, SGR-pixel, 휠) | xterm 기본 | done | `input.rs` 233~294, `terminal_surface.rs` 3744~3866 | 없음 | - |
| URL 링크(평문 http/https, 줄바꿈 연결, Cmd/Alt-click, hover 밑줄·커서) | `terminal-view.tsx` 48~49, 290~293 | done | `terminal_links.rs` 23, 224~344, 346~353, `terminal_surface.rs` 4529~4716, `host.rs` 763~782 | 없음 | - |
| OSC 8 하이퍼링크(http/https 만) | `terminal-view.tsx` 278~284 | done | `terminal_links.rs` 267~303 | 없음 | - |
| 파일 경로 링크(`path:line:col`, 실제 파일만, 행 캐시, 열고 위치 이동) | `terminal-file-link.ts`, `terminal-session.tsx` 230~235 | done | `terminal_links.rs` 14~22, 176~214, `terminal_file_links.rs`, `host.rs` 707~762, `terminal_tabs.rs` 90~175, `application.rs` 618~651 | 없음 | - |
| 링크 열기 실패 피드백(toast) | `terminal-session.tsx` 220~222 | partial | `application.rs` 652~662, 1336~1340 | toast 가 아니라 `self.status` 문자열로 표시 | S |
| 터미널 내 검색 | `terminal-view.tsx` 288(SearchAddon 로드만, 호출 UI 없음) | n/a | - | TS 에 사용자가 쓸 수 있는 검색 UI 가 없습니다(`findNext` 호출부 없음) | - |
| OSC 133 명령 블록 장식(gutter 2px, overview ruler, 성공·실패 색) | `terminal-osc133.ts` 203~273 | done | `terminal_surface.rs` 221~230, 4823~4832, `terminal_ruler.rs` | 없음 | - |
| 이전·다음 명령 점프(키맵) | `terminal-pane.tsx` 177~182 | done | `terminal_surface.rs` 3492~3501, 4119~4141 | 없음 | - |
| OSC 7 cwd 추적(이벤트, split 상속, 링크 기준 cwd) | `terminal-session.tsx` 371~374, 233 | done | `terminal_dispatch.rs` 163~178, `terminal_tabs.rs` 272~283, `terminal_surface.rs` 4556~4572 | 없음 | - |
| OSC 0/2·9·777 스캔(에이전트 감지 입력) | Rust 백엔드 경유 | done | `native-terminal/src/lib.rs` 171~271, `terminal_dispatch.rs` 191~200 | UI 소비는 2.2 참조 | - |
| 명령 완료 이벤트 발행(OSC 133 D + 소요 시간) | `native-notification-provider.tsx` 178 의 입력 | done | `terminal_dispatch.rs` 179~190 | 소비(알림)는 2.3 참조 | - |
| 컨텍스트 메뉴(복사·붙여넣기·전체 선택·지우기·분할 하위 메뉴·새 터미널·종료, 비활성 규칙, 포커스 복귀, 키보드 탐색) | `terminal-context-menu.tsx` 67~113, `terminal-pane.tsx` 142~156 | partial | `terminal_surface.rs` 1661~1989 | 항목 아이콘 없음(텍스트 버튼만, 1788~1930행) | S |
| 분할(새 터미널을 새 pane 에, cwd 상속, 공간 부족 시 비활성) | `terminal-session.tsx` 247~267, `terminal-split-availability.ts` | done | `terminal_surface.rs` 1719~1736, `terminal_tabs.rs` 266~302 | 없음 | - |
| 새 터미널(메뉴·탭 추가 메뉴·키맵) | `terminal-session.tsx` 269~273 | done | `terminal_tabs.rs` 246~265, `host.rs` 941~963, `shell_keymap.rs` 148~153, `taide-native-ui/src/shell.rs` 748 | 없음 | - |
| 터미널 종료(kill) | `terminal-session.tsx` 275 | done | `terminal_tabs.rs` 234~245, 314~318 | 없음 | - |
| toggle-terminal 키맵 | `editor-area.tsx` 228~247 | done | `shell_keymap.rs` 154~171 | 터미널이 활성일 때 돌아갈 탭 선택이 "첫 비터미널 탭"입니다(TS 는 `resolveTerminalToggleFallbackTab`). 동등성은 확인하지 못했습니다 | - |
| 프로세스 종료 화면 + 재시작 | `terminal-session.tsx` 399~408 | done | `terminal_surface.rs` 3301~3333, 4217~4241, `terminal_tabs.rs` 336~370 | 없음 | - |
| spawn 실패 화면 + 재시작 + toast | `terminal-session.tsx` 177~182, 388~397 | partial | `terminal_surface.rs` 3207~3212, 3310, `application.rs` 686~696 | toast 없음, `Phase::Failed` 문구가 현지화되지 않은 debug 문자열(`native terminal failed: {failure:?}`) | S |
| spawn 전 입력 버퍼링(4096) | `pending-terminal-input.ts` | done | `terminal_surface.rs` 67, 4084~4108, 2439~2455 | 없음 | - |
| spawn 중 탭이 닫히면 고아 세션 정리 | `terminal-session.tsx` 123~135 | done | `terminal_tabs.rs` 61~72, 430~478 | 없음 | - |
| 흐름 제어(HIGH/LOW water pause·resume) | `terminal-flow-control.ts`, `terminal-pane.tsx` 127~131, 199~209 | missing | `set_paused` 호출부는 원격 디스패치뿐(`remote-terminal.rs` 157) | 과부하 시 pause 대신 세션이 실패합니다(3장 결함 1). 검색어: `set_paused`, `pty_set_paused`, `Capacity` | M |
| 스크롤백 바이트 예산(설정 → ring) | `scrollback-budget.ts` | done | `terminal_tabs.rs` 14, 424~425 | 없음 | - |
| 포커스된 pane 의 터미널 자동 포커스 | `terminal-view.tsx` 331 | done | `application.rs` 4849~4853, `terminal_surface.rs` 3177~3179 | 없음 | - |
| 세션 roster 캐시 동기화(`terminal:spawned`·`terminal:exited`) | `terminal-session-cache.ts`, `ipc-sync-provider.tsx` 446~478 | n/a | - | TanStack Query 캐시 전용. native 는 hub 가 단일 출처입니다(`terminal_host.rs` 528~533) | - |
| 클립보드 가용성 probe | `terminal-clipboard-availability.ts` | n/a | - | secure context 전용 | - |
| IME·성능 디버그 기록 | `terminal-view.tsx` 351~373, 397 | n/a | - | 웹뷰 진단 전용 | - |
| Run Selected Text in Terminal | `terminal.commands.ts`, `editor-area.tsx` 283~288 | missing | 없음 | 명령·키바인딩 카탈로그 항목·에디터 선택 텍스트 전달. 검색어: `runSelectedText`, `run-selected-text`, `run_selected_text` | S |
| run-in-terminal 쓰기 경로(대상 터미널 탭 선택·생성, spawn 전 큐) | `editor-area.tsx` 257~281, `terminal-write-bridge.ts` | missing | 재료만 존재: `Session::write_raw`(`terminal_host.rs` 389~429, 원격 `pty_write` 전용) | 탭 단위 쓰기 요청 큐와 호스트 명령. 검색어: `run-in-terminal`, `run_in_terminal`, `write_raw` | M |
| 터미널 설정(글꼴·셸 override·셸 프로필·스크롤백·커서) | `settings-terminal-section.tsx` | done | `taide-native-ui/src/settings-code-view.rs` 126~176, 415~437, `host.rs` 673 | 설정 영역 감사에서 상세 판정합니다 | - |

### 2.2 에이전트

| 기능 | TS 근거 | native 상태 | native 근거 | 빠진 것 | effort |
|---|---|---|---|---|---|
| 에이전트 감지 주기 폴링 | `src-tauri/src/lib.rs` 1077~1089 | unwired | `agent_actions::poll_agents`(`crates/taide-runtime/src/agent_actions.rs` 135~174)는 native 에 호출부가 없습니다 | 폴링 루프 기동. 검색어: `poll_agents`, `agent_list`, `foreground_pids` | S |
| 에이전트 상태 UI 동기화(`agent:state-changed`) | `agent.query.ts` 60~66, `agent-state-sync-provider.tsx` | unwired | 이벤트는 원격 fan-out 만(`event-relay.rs` 165~168). `PaintSink` 는 Fs·Settings·Theme 만 처리(`events.rs` 35~60, `presentation-refresh.rs` 23~30) | 스냅샷에 에이전트 목록 반영 | M |
| 사이드바 프로젝트 아이콘의 에이전트 배지·접근성 라벨 | `app-sidebar.tsx` 84~136, `project-icon-button.tsx` 42~59 | missing | 없음(`taide-native-ui` 의 agent 언급은 설정 토글과 테마 토큰뿐) | 배지 표시, `agentStatusBadgeEnabled` 반영. 검색어: `DetectedAgent`, `agent.status.`, `agent.blocked` | M |
| 터미널 탭 아이콘(Sparkles + 활동 색)·툴팁 | `pane-tab-bar.tsx` 53~62, 185~197 | missing | `taide-native-ui/src/shell.rs` 에 탭 아이콘·에이전트 표시 없음 | 탭 아이콘과 `agent.sessionTooltip`. 검색어: `Sparkles`, `agent.sessionTooltip`, `AgentStateChanged` | M |
| 에이전트 훅 수신 서버(HTTP) | Rust 백엔드 | partial | `agent-hooks.rs` 36~68, 206~246, 기동은 `application-ports.rs` 98~105 | 폴링이 없어 `agents_for` 가 비어 있고 `apply_hook_payload` 가 조기 반환합니다(`agent_actions.rs` 207~210) | S |
| 훅 설치·제거·상태 UI(설정) | `agent-hooks-project-list.tsx`, `agent-hooks-project-row.tsx`, `agent.query.ts` 40~57 | unwired | 원격 디스패치에만 존재(`remote-agents.rs` 137~172) | native 설정 화면 행·호스트 명령. 검색어: `agentHooks`, `hooksInstall`, `agent_hooks_install` | M |
| CLI 설치 상태·설치·제거·외부 에디터 연결 명령 | `agent.commands.ts`, `agent-cli-status-row.tsx` | missing | 상태 조회만 원격 디스패치(`remote-agents.rs` 60~64, 136). 설치·제거 구현은 native 에 없습니다 | 명령 3종, 설정 행, toast. 검색어: `cliInstall`, `agent_cli_install`, `connectExternalEditor` | M |
| 외부 열기(`taide <file>`, Claude Code Ctrl+G, `--wait` 마커) | `agent-external-open-provider.tsx`, `agent-wait-marker-registry.ts`, `src-tauri/src/lib.rs` 821~829, 982 | missing | `main.rs` 는 `--data-dir` 외 인자를 거부(`bootstrap.rs` 25~52). `AgentExternalOpen` 은 무시(`event-relay.rs` 203). 마커는 종료 시 일괄 정리만(`application.rs` 4502) | 단일 인스턴스 인자 수신, 대기열 소비, 탭 닫힘 시 마커 해제, 안내 toast. 검색어: `external_open`, `parse_cli_payload`, `wait_marker`, `app.externalEditorTabHint` | L |
| 터미널 env 주입(`EDITOR`, `CLAUDE_CODE_SSE_PORT`, 프로토콜 버전) | Rust 백엔드 | done | `terminal_environment.rs`, `crates/taide-runtime/src/terminal_env.rs` 26~44, `host.rs` 861 | 없음 | - |

### 2.3 알림

| 기능 | TS 근거 | native 상태 | native 근거 | 빠진 것 | effort |
|---|---|---|---|---|---|
| 에이전트 완료·입력 대기 OS 알림(작업 시간 임계, 프로젝트 제목, 탭 이름, 차단 사유 문구) | `native-notification-provider.tsx` 143~165 | missing | 없음 | 완료 판정(`evaluateAgentCompletions` 대응), 문구 조립. 검색어: `notification.agent`, `AgentStateChanged`, `notification_notify` | M |
| 명령 완료 OS 알림(소요 시간 임계, exit code) | 같은 파일 178~194 | missing | 발행만 존재(`terminal_dispatch.rs` 183) | 이벤트 소비와 문구 조립. 검색어: `TerminalCommandFinished`, `notification.task`, `notification.exitCode` | S |
| LSP 설치 결과 OS 알림 | 같은 파일 196~204 | missing | 없음 | 검색어: `LspInstallProgress`, `notification.lspInstall` | S |
| 첫 전달 안내 toast | 같은 파일 137~141, `notify.ts` | missing | 없음 | 검색어: `notification.enableHint`, `subscribeNativeNotificationDelivered` | S |
| 알림 게이트(설정·포커스) + 플랫폼 전송 | `notification.ipc.ts`, `notify.ts` | unwired | `notification_actions.rs` 29, `bootstrap.rs` 151~164(macOS `osascript`)는 있으나 native 호출부 없음 | 위 세 알림에서 호출 | S |
| 설정 화면의 테스트 알림·시스템 설정 열기 | `settings-notification-section.tsx` | missing | 없음 | 검색어: `notificationsTest`, `TestNotification`, `notificationsOpenSystem` | S |

### 2.4 IDE(MCP) 서버·Claude diff

| 기능 | TS 근거 | native 상태 | native 근거 | 빠진 것 | effort |
|---|---|---|---|---|---|
| IDE MCP 서버(WebSocket, 인증, lockfile, 상태 이벤트) | Rust 백엔드 | done | `ide-server.rs` 120~297, 기동 `application-ports.rs` 55~66, 98~105 | 없음 | - |
| 상태바 IDE 연결 표시 | `status-bar.tsx` 32~34, 85~87 | done | `status-ide.rs` 162~200, `application.rs` 5147~5152 | 없음 | - |
| openFile 도구 | Rust 백엔드 | done | `ide-tools.rs` 102~142 | 없음 | - |
| openDiff → Claude diff 탭(원본·제안 비교, 제안 편집, 수락·거절, 탭 닫힘 시 거절) | `ide-sync-provider.tsx` 49~68, `claude-diff-pane.tsx` | missing | `ide-tools.rs` 144~197 은 이벤트만 발행. `TabKind::ClaudeDiff` 는 "native tab surface is not connected"(`application.rs` 4916~4922) | 탭 열기, diff 표면, 수락·거절 호스트 명령. 검색어: `ClaudeDiff`, `ide.acceptChanges`, `ide_resolve_diff` | L |
| `ideAutoOpenDiff` 설정 반영(꺼져 있으면 즉시 거절) | `ide-sync-provider.tsx` 53~58 | missing | 설정 토글만 존재(`taide-native-ui/src/settings-controls.rs` 178) | 소비 로직. 검색어: `ide_auto_open_diff`, `IdeAutoOpenDiff` | S |
| saveDocument 요청 처리(dirty 탭 저장 후 응답, read-only 거부) | `ide-sync-provider.tsx` 70~129 | unwired | `ide-tools.rs` 239~284 는 발행 후 5초 대기. 소비·`ide_resolve_save` 호출은 원격 디스패치뿐(`remote-ide.rs` 65~66) | native 에디터 저장 경로 연결 | M |
| close_tab·closeAllDiffTabs | Rust 백엔드 | done | `ide-tools.rs` 286~310, 366~383, `tabs.rs` 128~132 | 없음 | - |
| 선택 영역 push(getCurrentSelection·getLatestSelection) | `use-editor-ide-selection.ts` | unwired | `ide_set_selection`·`ide_clear_selection` 은 원격 디스패치뿐(`remote-ide.rs` 44~49) | native 에디터 선택 변경 시 debounce push | M |
| 진단 push(getDiagnostics) | `ide-sync-provider.tsx` 137~158 | unwired | `ide_publish_diagnostics` 는 원격 디스패치뿐(`remote-ide.rs` 50~52). native LSP 진단 저장소(`application.rs` 375 `lsp_diagnostics`)와 미연결 | 진단 변경 시 push | S |
| 닫힌 프로젝트의 대기 요청 정리 루프 | `src-tauri/src/lib.rs` 1070~1075 | unwired | `ide-server.rs` 223~229 `reconcile_stale_pending` 은 테스트에서만 호출(`ide-server-tests.rs` 287) | 주기 호출 | S |
| getOpenEditors·getWorkspaceFolders·checkDocumentDirty | Rust 백엔드 | done | `ide-tools.rs` 224~237, 339~361 | 없음 | - |

### 2.5 태스크

| 기능 | TS 근거 | native 상태 | native 근거 | 빠진 것 | effort |
|---|---|---|---|---|---|
| Task Runner 다이얼로그(열기, 퍼지 필터, 로딩·빈 상태, 출처 라벨, 선택 시 실행·닫기) | `task-runner-dialog.tsx` | missing | 없음 | 다이얼로그 전체. 검색어: `runTask`, `noTasksFound`, `TaskRunner`, `task-runner` | M |
| `task.runTask` 명령(프로젝트 활성 조건, 키바인딩 카탈로그 노출) | `task.commands.ts` | missing | `taide-native-ui/src/keybinding-catalog.rs` 에 항목 없음 | 명령 등록 | S |
| 태스크 감지(npm·make·cargo) | `task.ipc.ts` | unwired | `task_actions::detect_tasks` 는 원격 디스패치뿐(`remote-utilities.rs` 139~140) | 호스트 명령 | S |

### 2.6 집계

총 78개: done 38, partial 7, unwired 10, missing 17, n/a 6.

## 3. 잘못 구현됐거나 보강이 필요한 native 코드

1. **출력 과부하 시 세션 강제 종료(높음).** 프레임 큐는 `try_acquire` 방식의 고정 한도(64개, 4MiB — `application.rs` 37~38)이고 초과 시 `Error::Capacity` 로 채널 전체를 실패 처리합니다(`terminal_frames.rs` 117~178). 전달 실패는 `DeliveryGuard` 가 코어를 `Failed(Delivery)` 로 만들고 PTY 를 kill 합니다(`native-terminal/src/session.rs` 70~82, 419~431). TS 의 pause·resume(`terminal-flow-control.ts`)에 해당하는 backpressure 가 없어, 리더 스레드가 액터보다 빠른 구간에서 터미널이 죽을 수 있습니다. `PtySession::set_paused`(`crates/taide-infra/src/pty.rs` 379)가 이미 있으므로 큐 수위 기반 pause 로 바꿔야 합니다. 실제 재현 여부는 실행 검증이 필요합니다.
2. **openDiff·saveDocument 가 소비자 없이 대기(높음).** `ide-tools.rs` 27행의 `IDE_DIFF_TIMEOUT` 은 600초입니다. native UI 가 `IdeDiffRequested` 를 소비하지 않으므로 Claude Code 의 수정 제안이 최대 10분 멈춘 뒤 거절로 끝납니다. saveDocument 는 5초 뒤 항상 실패합니다(26, 272~283행). diff 표면이 준비되기 전까지는 즉시 거절하거나 IDE 통합을 켜지 못하게 해야 합니다.
3. **에이전트 감지가 통째로 동작하지 않음(높음).** `poll_agents` 호출부가 없어 `AgentStore` 가 채워지지 않고, 훅 payload 도 `updated.is_empty()` 로 버려집니다(`crates/taide-runtime/src/agent_actions.rs` 207~210). 훅 서버는 뜨지만 효과가 없습니다.
4. **포커스가 있을 때만 PTY resize(중간).** `terminal_surface.rs` 3345행. 에디터에 포커스를 둔 채 분할선을 끌거나 글꼴 크기를 바꾸면 터미널 그리드가 예전 크기로 남고, 클릭해야 맞춰집니다. TS 는 `ResizeObserver` 로 포커스와 무관하게 맞춥니다.
5. **`view.error` 가 지워지지 않음(중간).** 설정 지점은 여러 곳(예: 3591, 3612, 4029, 4045행)인데 초기화 지점이 없습니다(`error = None` 검색 결과 3937행의 지역 변수뿐). 한 번 발생한 일시 오류 문구가 3616~3624행에서 터미널 좌상단에 계속 덧그려집니다. 세션 교체·재시작으로 `View` 가 새로 만들어질 때만 사라집니다.
6. **프로젝트 닫기 시 hub 세션 미정리(중간).** `projects.rs` 187~197 `detach_all` 은 `services.terminal.kill_project` 만 호출하고 `Hub::discard`·`close` 를 부르지 않습니다. `Entry` 가 그리드와 admission permit(최대 64 — `application.rs` 36)을 쥔 채 남습니다(`terminal_host.rs` 516~533, 624~646).
7. **렌더 비용(중간).** `paint` 가 매 프레임 전체 그리드를 배경·전경 2회 순회하며 셀마다 `painter.text` 를 호출합니다(`terminal_surface.rs` 4748~4822). `TerminalCore::damage`(`native-terminal/src/lib.rs` 673)는 호출되지 않습니다. 커서 blink 와 입력 폴링(`RECEIPT_POLL` 16ms)이 재도색을 유발하므로 큰 창에서 CPU 사용이 커질 수 있습니다. 행 단위 galley 캐시나 damage 기반 갱신이 필요합니다.
8. **모듈 경계(낮음~중간).** 앱 전역 키맵 라우팅(`capture_window_keymap`, `route_window_keys`, `chord_status`, `keymaps` 필드)이 `terminal_surface::Views` 안에 있습니다(1226, 2937~3139행). 호출부는 터미널과 무관한 문맥입니다(`application.rs` 3442, 3621, 3730, 3918행). 파일 하나가 렌더·입력 순서 보존·메뉴·선택·링크·키맵을 8,300줄에 담고 있어 수정 위험이 큽니다.
9. **오류 표현 불일치(낮음).** 터미널 관련 실패가 toast 가 아니라 `self.status` 로 가고(`application.rs` 652~701), 실패 문구가 현지화되지 않은 debug 포맷입니다(`terminal_surface.rs` 3310). 입력 거절도 `{error:?}` 로 노출됩니다(`terminal_host.rs` 254, 287).
10. **macOS·격리 실행 전제(중간).** 알림·열기는 macOS 가 아니면 오류를 반환합니다(`bootstrap.rs` 151~181). 글꼴 fallback 도 macOS 글꼴뿐입니다(`terminal_fonts.rs` 15). 실행은 `--data-dir <격리 경로>` 만 허용하고(`bootstrap.rs` 25~52) secret 서비스명도 `net.gumyo.taide.native-isolated`(15행)라, CLI 외부 열기와 실제 사용자 데이터 연결이 구조적으로 막혀 있습니다.

보조 관찰: `ide-server.rs` 212~221 의 `pub fn refresh_lockfile` 은 테스트에서만 쓰이고 운영 경로는 `projects.rs` 295행의 별도 구현을 씁니다(중복).

## 4. 실제 앱 연결이 끊긴 지점

| 지점 | 존재하는 것 | 끊긴 곳 |
|---|---|---|
| 에이전트 폴링 | `agent_actions::poll_agents`, `agent_host::detect_agents_for_pids_blocking` | native 에 주기 호출 없음(원격 `agent_list` 요청 때만 probe — `remote-agents.rs` 112~125) |
| 에이전트 상태 → UI | `AppEvent::AgentStateChanged` 발행(`agent_actions.rs` 165, 214) | `PaintSink`(`application.rs` 52~58)가 무시, 스냅샷·셸에 에이전트 필드 없음 |
| 명령 완료 → 알림 | `AppEvent::TerminalCommandFinished`(`terminal_dispatch.rs` 183) | 소비자 없음. `notification_actions`·`send_notification` 호출부 없음 |
| IDE diff | `IdeDiffRequested` 발행, `services.ide.insert_pending_diff_owned`(`ide-tools.rs` 171~186) | 탭 열기·표면·`ide_resolve_diff` 호출 모두 원격 디스패치에만 존재(`remote-ide.rs` 54~63) |
| IDE save | `IdeSaveRequested` 발행(`ide-tools.rs` 267) | native 소비자 없음 |
| IDE selection·diagnostics | `IdeStore` 의 selection·diagnostics 저장소 | native 에디터·LSP 진단에서 push 하는 코드 없음 |
| IDE stale 정리 | `ide_server::reconcile_stale_pending` | 테스트 전용 호출 |
| 외부 열기 | `AgentStore::push_pending_external_open`, `parse_cli_payload`(`crates/taide-agent/src/service.rs` 256) | native 에 인자 수신·대기열 소비 없음 |
| 훅 설치·CLI 상태 | `agent_hook_actions::*`, `agent_host::cli_install_status` | 원격 디스패치(`remote-agents.rs`)에만 연결 |
| 태스크 감지 | `task_actions::detect_tasks` | 원격 디스패치(`remote-utilities.rs` 139)에만 연결 |
| Option-as-Meta | `input.rs` 193~203 의 Alt 인코딩 | `terminal_surface.rs` 4271 에서 Alt 단독 문자 키를 넘기지 않음 |
| 터미널 외부 쓰기 | `Session::write_raw`(`terminal_host.rs` 389) | 원격 `pty_write` 에서만 사용 |
| OSC 스트림 포트 | `EffectPorts::event`·`stream` | native 표면은 no-op 을 넘깁니다(`terminal_surface.rs` 4212~4213). 현재 TS 대비 누락은 없으나 탭 제목 등 확장 시 채워야 합니다 |

## 5. 권장 구현 순서

1. **안전 조치(선행, S).** diff 표면이 없을 때 openDiff 를 즉시 거절하고 saveDocument 를 실패로 즉시 응답하도록 합니다(결함 2). `view.error` 초기화(결함 5), 프로젝트 닫기 시 hub 정리(결함 6)를 함께 처리합니다.
2. **터미널 안정성(M).** 프레임 큐 수위 기반 `set_paused` backpressure(결함 1), 포커스와 무관한 resize(결함 4), Option-as-Meta 연결. 이후 항목이 모두 터미널 위에서 동작하므로 먼저 고정합니다.
3. **이벤트 → UI 통로(M).** `PaintSink` 또는 셸 컨트롤러에 Agent·Ide·TerminalCommandFinished·LspInstallProgress 이벤트 수신 지점을 만듭니다. 4~7단계의 공통 선행 조건입니다.
4. **에이전트 감지·표시(M).** 폴링 루프 기동 → 스냅샷에 에이전트 목록 → 사이드바 배지·탭 아이콘·툴팁. 탭 바·사이드바 영역 작업과 파일이 겹치므로 해당 영역 담당과 순서를 맞춥니다.
5. **OS 알림(M).** 3·4단계 결과 위에 완료 판정과 문구 조립, 게이트 호출, 첫 전달 toast, 설정 테스트 버튼.
6. **run-in-terminal 경로 → Run Selected Text → Task Runner(M).** 탭 단위 쓰기 큐(`Session::write_raw` 재사용)를 먼저 만들고, 에디터 선택 텍스트 명령과 태스크 다이얼로그를 얹습니다. 다이얼로그는 명령 팔레트·퍼지 필터 구현과 공용 부품을 공유해야 하므로 명령 팔레트 영역 진행 상황에 의존합니다.
7. **IDE 통합 완성(L).** selection·diagnostics push(S~M) → saveDocument 처리(M, native 저장 경로 의존) → Claude diff 표면(L, native diff 뷰어 의존). diff 뷰어는 git diff 영역과 공용 부품이므로 그쪽 일정이 선행입니다.
8. **외부 열기·CLI(L).** 실행 인자·단일 인스턴스 정책과 `--data-dir` 격리 해제 결정이 먼저 필요합니다(사용자 결정 사항). 이후 대기열 소비, wait 마커 해제, CLI 설치·제거 명령과 설정 행, 훅 설치 UI.
9. **렌더 품질(M).** 볼드·이탤릭 글꼴, 밑줄 변형, 스크롤바 thumb, 컨텍스트 메뉴 아이콘, 오류 toast·현지화, damage 기반 도색.

## 6. 확인하지 못한 것

- 빌드·테스트·실행을 하지 않았습니다. 결함 1(과부하 종료)과 결함 7(렌더 비용)은 코드 구조상의 위험이며 실제 발생 빈도는 측정하지 않았습니다.
- `terminal_surface.rs` 의 1340~1660, 2130~2411, 2545~2932행은 정독하지 않았습니다(메뉴 키보드 탐색, 포인터 캡처, 포커스·입력 순서 보존). 해당 구역의 세부 결함은 판정에 포함하지 않았습니다.
- `crates/taide-{terminal,agent,ide,task,notification}` 본문과 `terminal_host.rs` 1~220행, `terminal_writer.rs` 는 정독하지 않았습니다. 이 crate 들은 기존 Tauri 백엔드와 공유되는 코드로 보고 native 호출 여부만 판정했습니다.
- xterm 6.0.0 의 스크롤바가 TS 앱 화면에서 실제로 보이고 드래그되는지는 실행으로 확인하지 않았습니다(패키지 버전과 옵션 근거의 판정).
- egui 가 macOS 에서 Option+문자 입력을 어떤 이벤트로 전달하는지는 `vendor/egui-input` 패치 내용을 읽지 않아 확정하지 못했습니다. `terminal_surface.rs` 4271행 기준으로는 Alt 단독 문자 키가 인코더에 도달하지 않습니다.
- 보조 창(`WindowScope::Auxiliary`, `application.rs` 4138)에서 터미널이 같은 경로로 그려지는지는 호출 체인을 끝까지 따라가지 않았습니다.
- toggle-terminal 의 복귀 탭 선택이 TS `resolveTerminalToggleFallbackTab` 과 같은지 비교하지 않았습니다.
- 설정 화면의 터미널 섹션이 실제 앱에서 도달 가능한지는 설정 영역 감사에 맡겼습니다.
- `ide-tools.rs` 의 openFile 로 바뀐 레이아웃이 native 화면에 즉시 반영되는지는 확인하지 않았습니다.
