# Rust-native 기능·성능 동등성 계획

> 상태: 계획 확정, 측정·구현 미착수
> 계약: `docs/acknowledge/2026-09-23-rust-native-transition-contract.md`
> 로드맵: `docs/roadmap-rust-native.md`
> 현행 실기 기준선: `docs/quality-assurance/2026-09-04-perf-baseline.md`

## 1. 증거 원칙

- 현재 앱의 동작을 문서 설명만으로 추정하지 않고 기존 테스트, 실제 fixture, 실기 결과로 기준선을 고정한다.
- native 구현은 같은 input과 fixture에 대해 semantic output, event ordering, persistent state, 화면과 성능을 비교한다.
- 벽시계 수치는 같은 기기·release build·fixture에서 3회 이상 측정한 중앙값과 p95·p99만 비교한다.
- 자동화할 수 없는 IME·accessibility·window·GPU·packaging은 대상 OS별 실기 gate로 남긴다.
- 통과하지 않은 항목을 완료로 표시하지 않고 TS/Tauri fallback을 제거하지 않는다.

## 2. Phase 0 기준선

- [ ] command, event, raw channel 이름·payload·error code manifest
- [ ] remote allow·deny, authentication, session revoke와 binary channel fixture
- [ ] IDE/MCP request·response와 CLI `taide --wait` marker fixture
- [ ] settings, session, project, layout, hot-exit buffer의 versioned fixture
- [ ] `docs/quality-assurance/2026-09-04-perf-baseline.md` 실기 지표 작성
- [ ] editor, LSP, terminal, preview, shell 기능 inventory에 근거 파일·시험 연결

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
