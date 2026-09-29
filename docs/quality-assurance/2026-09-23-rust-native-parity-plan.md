# Rust-native 기능·성능 동등성 계획

> 상태: Phase 0 계약·기능·성능 기준선 완료. M6 직접 Exit 전체 adapter gate와 M8 native 동등성은 별도
> 계약: `docs/acknowledge/2026-09-23-rust-native-transition-contract.md`
> 로드맵: `docs/roadmap-rust-native.md`
> 현행 실기 기준선: `docs/quality-assurance/2026-09-04-perf-baseline.md`

## 1. 증거 원칙

- 현재 앱의 동작을 문서 설명만으로 추정하지 않고 기존 테스트, 실제 fixture, 실기 결과로 기준선을 고정한다.
- native 구현은 같은 input과 fixture에 대해 semantic output, event ordering, persistent state, 화면과 성능을 비교한다.
- M7·Phase 0의 현행 release 기준선은 [단일 검증 결정](../acknowledge/2026-09-29-m7-one-pass-validation-scope.md)에 따라 같은 기기·fixture의 유효 관찰 한 건으로 기록합니다. 이 값으로 중앙값·p95·p99 또는 통계적 비악화를 주장하지 않습니다.
- 이후 native 구현의 정량 동등성 비교에는 같은 기기·release build·fixture에서 반복 측정한 중앙값과 p95·p99가 필요합니다. 그 비교 전에 현행 baseline을 다시 측정합니다.
- 자동화할 수 없는 IME·accessibility·window·GPU·packaging은 대상 OS별 실기 gate로 남긴다.
- 통과하지 않은 항목을 완료로 표시하지 않고 TS/Tauri fallback을 제거하지 않는다.

## 2. Phase 0 기준선

- [x] command, event, raw channel 이름·payload·error code manifest — `src-tauri/tests/fixtures/rust-native/ipc-contract-manifest.json`, `src-tauri/tests/rust_native_phase0_contract.rs`
- [x] remote allow·deny, authentication, session revoke와 binary channel fixture — `remote-wire-session-v1.json`의 서비스·store·protocol API 3건, [실제 인증 HTTP/WebSocket 대표 명령](2026-09-29-m7-remote-authenticated-session.md), [느린 수신자 및 활성 연결 폐기](2026-09-29-m7-remote-slow-receiver-revoke.md), [제품 라우터 가상 7일 만료 HTTP 401·WebSocket 4001](2026-09-29-m7-remote-ttl-deterministic.md), 256프레임 포화 신호 검사를 연결했습니다. 실제 7일 경과와 내부 큐 점유량은 관찰하지 않았습니다.
- [x] IDE/MCP request·response와 CLI `taide --wait` marker fixture — IDE protocol API 3건·CLI marker bin 18건과 [실제 앱의 CLI 대기/해제](2026-09-29-m7-cli-wait-gui.md)·[인증 WebSocket 도구/탭 수명](2026-09-29-m7-ide-ws-authenticated-open-file.md)을 각각 한 번 확인. 전체 IDE 도구 전수는 별도
- [x] settings, session, project, layout, hot-exit buffer의 versioned fixture — `persistence-v1.json`과 실제 settings load·session/project restore·v1→v2 layout load·legacy mirror list 3건 통과; 사용자 실제 데이터와 GUI 복원은 별도
- [x] `docs/quality-assurance/2026-09-04-perf-baseline.md` 실기 지표 작성 — 11개 세부 행의 단일 표본. 부팅은 [사용자 승인 화면 표시 준비 대리지표](../acknowledge/2026-09-29-m7-boot-visible-ready-proxy.md)이고 실제 첫 픽셀 시각이 아닙니다. 터미널은 [전면 200만 줄 렌더](2026-09-29-m7-terminal-foreground-render.md)의 writer 기준 참고 처리량이며 native의 독립 픽셀 처리량과 같다고 보지 않습니다.
- [x] editor, LSP, terminal, preview, shell 기능 inventory에 근거 파일·시험 연결 — [5도메인 구현·자동 검사 경로와 미검증 항목](2026-09-28-rust-native-function-inventory.md)을 기록하고 [대표 앱 실기](2026-09-29-m7-debug-function-gui-smoke.md)를 연결했습니다. 형식별 preview·개별 기능 전수 실기는 완료로 주장하지 않습니다.
- [x] 현재 TS view의 212개 경로에 실제 컴포넌트·자동 근거·미검증 상태를 연결하고 [대표 화면·다이얼로그·키보드·테마/로케일·멀티윈도·접근성 경로](2026-09-29-m7-ts-view-representative-gate.md)를 한 번씩 실측. 개별 경로 전수·native 동등성은 별도

UI 착수 전 gate: 위 inventory와 Phase 0 기능·데이터·성능 baseline 및 기능별 crate 분리 M1~M7이 모두 준비·검증돼야 합니다. native UI 구현은 이 gate 이후에만 시작합니다. 각 TS view 항목에 native 대응 경로·자동 검사·실기 결과를 연결하고 미대응 항목이 0이 될 때까지 TS/Tauri view를 유지합니다. 시각적 구성의 유사성은 테마별 캡처와 실제 창 크기·포커스·IME·보조 창 동작에서 비교하며, 기능 동등성을 단순한 화면 유사성으로 대체하지 않습니다.

## 3. Application shell

- [ ] 프로젝트 열기·닫기·전환·recent·group과 shell slot 복원
- [ ] tabs, horizontal·vertical split, drag/drop, auxiliary window 이동·회수
- [ ] Explorer, Search, Git, Problems, Outline, Settings, Theme, Task panels
- [ ] command palette, remappable shortcut와 chord, context menu, clipboard
- [ ] native menu, file dialog, notification, external URL·path open
- [ ] theme 36종, syntax·terminal palette, en·ko·ja와 사용자 locale
- [ ] keyboard-only navigation, focus order, semantic role·name·state
- [ ] VoiceOver 실기와 상태 변경 announcement
- [ ] 한글·일본어·중국어 IME, emoji·bidi, clipboard·DnD
- [ ] multi-window, sleep/wake, scale factor와 GPU device-loss fallback

## 4. Editor

- [ ] 동일 문서 2개 이상 split의 buffer 공유와 view별 selection·scroll·fold
- [ ] typing, multi-cursor, paste, undo·redo grouping, snippet placeholder
- [ ] 한글 IME composition cancel·commit, emoji ZWJ, bidi hit-test·selection
- [ ] save 중 타이핑, autosave, format-on-save, hot-exit와 crash restore
- [ ] external change+dirty conflict, rename+unsaved edit+undo, delete·recreate
- [ ] read-only, 손실 인코딩 차단과 2MB·20MB·50MB size tier
- [ ] syntax highlight, semantic token, diagnostics, inlay hint와 stale-result 차단
- [ ] Git gutter·blame, diff, conflict decoration, Peek·reveal·breadcrumb
- [ ] snippets, Emmet, AI inline completion·edit, selected text to terminal
- [ ] large file scroll·selection·search와 model·atlas·decoration memory 회수

## 5. LSP

- [ ] detect·install·checksum·archive·spawn·initialize 성공 경로
- [ ] initialize timeout, process crash, stderr redaction, exponential retry budget
- [ ] `Detected/Installing/Spawning/Initializing/Running/Degraded/Restarting/Stopping/Stopped` transition
- [ ] split 동일 문서의 `didOpen` 1회, revision별 `didChange` 1회, 마지막 view `didClose` 1회
- [ ] crash generation 증가 후 initialize·열린 문서 replay·capability 등록 1회
- [ ] timeout·cancel 뒤 pending request 0건과 stale response 폐기
- [ ] UTF-8·UTF-16 position encoding, 한글·emoji surrogate pair 변환
- [ ] completion, hover, signature, definition·type definition·implementation·references
- [ ] rename, code action, code lens, formatting 3종, workspace symbol·document symbol
- [ ] workspace edit의 create→edit→rename→delete와 root guard·dirty transaction
- [ ] multi-root, project switch, auxiliary window, sleep/wake와 장시간 session
- [ ] LSP unavailable 시 TypeScript·JavaScript fallback 또는 명시적 degraded UX

## 6. Terminal

- [ ] CSI·SGR·erase·cursor, primary·alternate screen, resize·reflow
- [ ] UTF-8 chunk split, combining mark, CJK width, emoji·ZWJ와 grapheme copy
- [ ] application cursor·keypad, bracketed paste, focus, mouse reporting mode
- [ ] macOS 한글·일본어·중국어 IME preedit·cancel·commit 1회 전송
- [ ] selection, word·line·block copy, search, context menu, font size
- [ ] OSC 7 cwd, OSC 8 HTTP(S), OSC 9·777 notification, OSC 133 block·duration·jump
- [ ] file `path:line:column`의 wide-cell hit-test와 project root guard
- [ ] unterminated·oversized OSC, C0 payload, OSC52 기본 차단
- [ ] spawn 이전 4,096자 입력 queue, writer ordering, resize·pause·kill
- [ ] attach replay-before-live의 중복·누락 0, multi-window subscriber convergence
- [ ] 2–32MiB scrollback eviction, background tab, hidden pane와 renderer 회수
- [ ] 64KiB burst와 10분 sustained stream의 데이터 유실·무한 RSS 증가 0

## 7. Git·검색·파일·프로젝트

- [ ] status, staged·unstaged diff, line·hunk·file stage·unstage·discard
- [ ] commit, amend, push·pull·fetch, SSH·HTTPS credential, branch·tag·stash
- [ ] conflict ancestor·ours·theirs, resolve, graph, blame, history와 watcher invalidation
- [ ] file open·save·create·rename·delete·trash·root guard·symlink
- [ ] tree pagination·virtualization·selection·reveal·filesystem watcher rescan
- [ ] text search·replace, ignore rules, scope, cancellation와 streaming result
- [ ] project capability attach·detach와 session/layout restore·flush

## 8. Preview·plugin·나머지 기능

- [ ] image·SVG, video·audio, PDF, HTML, XLS·XLSX·CSV, PPTX, HWP·HWPX 정상 fixture
- [ ] 손상·대형·권한 밖 preview와 parser/helper crash containment
- [ ] HTML script 차단, bridge 없음, root 제한 resource와 external URL gate
- [ ] theme/VSIX, grammar, snippets, locale·theme user overlay
- [ ] AI provider streaming·cancel·keyring, sync와 remote-control
- [ ] agent detection·hooks·notification, IDE/MCP, CLI install·`--wait`
- [ ] task detection·execution, system usage, app update·release integration

## 9. 성능 게이트

현행 release baseline이 아직 채워지지 않은 지표는 Phase 0에서 먼저 측정한다. 아래 절대값은 초기 목표이며 실제 기기·framework spike 결과로 별도 승인 없이 느슨하게 만들지 않는다.

| 지표 | Gate |
| --- | --- |
| keypress → paint | p95 16.7ms 이하, p99 33.3ms 이하 |
| tree·editor·terminal scroll frame | p95 16.7ms 이하 |
| LSP response 수신 → UI paint | p95 16.7ms 이하, 전체 요청은 같은 server·fixture에서 현행 대비 비악화 |
| terminal 64KiB burst 중 입력 반응 | p95 50ms 이하, 데이터 유실 0 |
| boot, project/file open, palette, tree, Git, search | 현행 release 중앙값 대비 5% 이상 악화 금지 |
| terminal throughput | 동일 fixture MB/s 비악화 |
| idle CPU와 aggregate RSS | 현행 release 비악화, 숨김 pane frame loop 0 |
| 20개 file open·close 후 RSS | 안정화 뒤 지속 증가 0, document/view/atlas 회수 확인 |
| 10,000행 tree·50 tabs | 입력·scroll budget 유지 |
| 2MB·20MB·50MB·300K line | 현행 size tier보다 기능·메모리 정책 악화 금지 |

native 전환이 자동으로 성능을 개선한다고 가정하지 않는다. 각 최적화는 profiler·counter·trace 근거와 전후 fixture를 가진다.

## 10. 데이터·보안·protocol 게이트

- [ ] 기존 data directory read·write·restart와 이전 앱 재실행
- [ ] future schema 거부·fallback, atomic write와 crash recovery
- [ ] dirty buffer·hot-exit·flush-before-teardown 무손실
- [ ] root guard, symlink, external file authorization와 file permission
- [ ] keyring account namespace와 secret·token·PII log 비노출
- [ ] remote Host·Origin·cookie·password·rate limit·session TTL·revoke
- [ ] remote command allow·deny parity와 raw/binary stream budget
- [ ] non-http external URL 차단, HTML/SVG/preview 비신뢰 입력 격리
- [ ] IDE/MCP와 CLI wire compatibility

## 11. Framework·dependency 승인 gate

- [ ] 공식 manifest의 version, MSRV, license, transitive dependency와 maintenance 확인
- [ ] VoiceOver·CJK IME·multi-window·external DnD·menu·dialog hard gate
- [ ] release packaging, signing·notarization, GPU fallback 실기
- [ ] unsafe·FFI 경계와 parser attack surface 보안 검토
- [ ] upstream update·CVE·fork maintenance owner 합의
- [ ] 동일 spike fixture의 기능·성능 결과 기록

후보가 이 gate를 통과하기 전에는 Cargo dependency를 제품에 고정하거나 대규모 구현을 시작하지 않는다.

## 12. Cutover·release·rollback

- [ ] native slice가 opt-in으로 기존 core·data와 함께 동작
- [ ] shadow comparison 또는 golden fixture 불일치 0
- [ ] native 기본 전환 후 Tauri fallback build를 한 beta/release 유지
- [ ] 실제 프로젝트 장기 session, crash/restart, sleep/wake, multi-window soak
- [ ] macOS Apple Silicon release signing·notarization·fresh install·upgrade
- [ ] 이전 안정 Tauri release 재설치와 데이터 복구 절차 확인
- [ ] beta regression 기간에 P0·P1 미해결 0, 데이터 손실·보안 회귀 0
- [ ] TS·React·Tauri·Monaco·xterm 참조와 frontend build 자산 0건

마지막 항목까지 통과하기 전에는 기존 UI source와 dependency를 삭제하지 않는다.
