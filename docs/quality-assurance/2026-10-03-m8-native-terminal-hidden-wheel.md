# M8 native 숨김 터미널의 일반 캡처 스크롤

## 대상·근거

`native/taide-native-app/src/terminal_surface.rs`, `tests/terminal-host.rs`, `native/taide-native-terminal/tests/fixtures/session.rs`입니다. N4-B의 일반 wheel 숨김·포화 처리이며 전체 terminal 입력 순서 완료는 아닙니다.

설치된 xterm 6.0.0 `src/browser/CoreBrowserTerminal.ts`의 wheel listener는 scrollback 없는 buffer에서 wheel 한 건을 live application-cursor 방향키 하나로 전송합니다. 현재 native 구현도 이 기준을 유지합니다. [egui 0.36.2 Context](https://docs.rs/egui/0.36.2/egui/struct.Context.html)의 UI frame과 logic-only 경계를 확인했습니다. 실제 OS 입력·픽셀 페인트나 원본 DOM의 숨김 동작까지 동등성을 검증한 것은 아닙니다.

## 재현·수정

- [x] 실제 writer count=1에서 문자 64개로 Outbox를 채운 뒤 같은 frame의 일반 wheel을 캡처했습니다. 탭을 숨겨 두 frame을 진행하면 기존 코드는 stale target/frame을 이유로 packet을 지우며, child는 방향키를 받지 못합니다. timeout 실패 뒤에도 close/join·supervisor 회수를 수행했습니다.
- [x] 일반 wheel도 캡처 순번·원래 geometry를 보유하고, 단순 숨김/다음 frame에서 대기 큐를 지우지 않습니다. 새 wheel hit는 직전 유효 target의 clip/layer/viewport를 계속 요구합니다. 숨긴 target에 새 wheel을 받도록 범위를 넓힌 것은 아닙니다.
- [x] `flush_captured_pointer`가 mouse와 일반 wheel의 앞 packet 중 작은 전역 캡처 순번을 선택합니다. 완료/처리 frame·현재 keyboard event 앞의 경계와 기존 Outbox 예산을 지키며, 숨긴 view의 대기 wheel도 background에서 승인합니다. 화면 끝에서도 같은 drain을 먼저 실행합니다.
- [x] 일반 scrollback은 writer 슬롯이 없어도 로컬 offset을 처리합니다. scrollback 없는 buffer의 방향키는 슬롯이 생길 때까지 raw packet을 보존합니다. UI에 다시 나타나거나 geometry가 바뀌어도 캡처 당시 geometry로 처리합니다.
- [x] disabled/종료 target 등록, 없는 session, phase 종료, tracking mode 전환, 앱 취소 경계는 미승인 wheel을 정리합니다. 승인된 writer 전송을 취소하거나 되돌린다고 주장하지 않습니다. 실제 모든 종료/모드 조합은 아래 미완료 범위로 남깁니다.

일반 raw wheel은 view별 최대 64 packet입니다. mouse raw queue도 별도의 기존 64개 상한이며, 공유 순번은 checked 증가합니다. 이 두 큐의 수 상한을 앱 전체 RSS/aggregate 또는 Session admission 256개 상한의 증명으로 계산하지 않습니다. frame별 새 입력 포화의 오류 표시·복구 정책도 전체 완료가 아닙니다.

## 실제 검사

공통 Cargo 환경은 `CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo`, 직렬 `--locked --offline --target-dir experiments/native-shell-spike/target`입니다.

| 대상·검사 | 실제 결과 |
| --- | --- |
| app `test --test terminal-host hidden_wheel -- --nocapture`, 최초 | exit 101, compile 4.32초 / suite 6.33초. `hidden wheel was not delivered` |
| 같은 검사, 구현 후 | 1 PASS, compile 6.15초 / suite 1.59초. 64개 x→숨김 중 ESC OA, 복원 뒤 이전 view의 ESC OB→새 view의 z. child literal 검증·epoch+67·exit 0·close/join/task 0 |
| app `test --lib wheel_ -- --nocapture`, 새 unit 전 | 기존 3 PASS, compile 2.07초 / suite 0.05초. smoothing tail·disabled/hidden/viewport/budget·wheel 정책 |
| app `test --test terminal-host mouse -- --nocapture` | 영향 2 PASS, compile 0.21초 / suite 1.47초. 실제 binary/SGR 혼합과 숨김 release/다른 view 순서·join |
| app `test --lib wheel_hidden -- --nocapture` | 신규 1 PASS, compile 2.66초 / suite 0.02초. 두 frame 숨김 뒤 packet 2개 유지·64 receipt 포화 중 캡처 geometry로 로컬 offset 3·core display offset 0·남은 packet 취소·writer/task 0 |
| app `test --test terminal-host headless_wheel -- --nocapture` | 영향 1 PASS, compile 2.53초 / suite 0.35초. Text/IME/Tab·live alt/app-cursor 방향키 literal/exit/join |
| app `clippy --lib --test terminal-host -- -D warnings` | exit 0, 1.43초 |
| native terminal `clippy --bin native-session-fixture -- -D warnings` | exit 0, 0.24초 |

authored 3파일 exact rustfmt와 추적 diff 검사는 각각 exit 0입니다. 명시된 untracked 코드/QA/bug 파일의 공백 검색은 exit 1·일치 없음입니다. 기존 Wry 17 warnings와 authored strict 결과를 구분합니다. 같은 성공을 반복하지 않고 변경 위험에 해당하는 검사만 실행했습니다. 후속 unit 추가는 production 변경이 아니므로 앞선 실제 PTY/기존 wheel 성공을 재사용했습니다. 빈 hunk와 중복 문서 대상의 apply_patch는 검증 단계에서 거절되어 파일 변경이 없었고 실제 문맥으로 다시 적용했습니다.

## 미완료 경계

### 후속: buffer/rows 세대 정리

- [x] `reconcile_view`는 buffer/rows epoch 변경 때 mouse와 wheel 누적 상태는 지웠지만 일반 captured-wheel 큐는 지우지 않았습니다. 실제 Core의 alternate buffer 왕복 뒤 최종 mode가 원래와 같아도 오래된 raw wheel이 방향키 1개를 생성했습니다. 단순 mode 비교로 buffer ABA를 식별할 수 없음을 확인했습니다.
- [x] 기존 `NativeSelectionStamp.buffer_epoch`와 `rows_epoch` 정리 분기에 captured-wheel 큐도 함께 연결했습니다. 기존 vendor의 `swap_alt`/reset·실제 행 수 resize가 세대를 갱신하는 source를 확인했으며 새로운 세대/의존성/제품 상태는 추가하지 않았습니다. 이미 Session에 들어간 encoded pending은 이 raw 정리로 되돌리지 않습니다.
- [x] app `test --lib wheel_epoch -- --nocapture`의 첫 작성은 `Key`에 PartialEq가 없어 E0369로 compile 실패했습니다. 빈 목록이라는 동일 기대를 `is_empty`로 표현한 뒤 실제 RED는 exit 101, compile 1.87초 / suite 0.01초, `stale wheel produced 1 keys`입니다. buffer 왕복 첫 경계에서 실패했으므로 reset/rows의 수정 전 개별 실행 통과·실패를 주장하지 않습니다.
- [x] 구현 뒤 같은 `wheel_epoch` 1 PASS(exit 0)입니다. actual Core의 alternate 왕복·RIS reset·rows resize 세 경계에서 epoch 변경과 stale 키 0/packet 0을 확인했습니다. 같은 성공을 반복 실행하지 않습니다. 종료된 Cargo handle 74074의 후속 조회는 `Unknown process id`여서 재시작하지 않았습니다.
- [x] 영향 `test --lib wheel_hidden -- --nocapture` 1 PASS, compile 0.26초 / suite 0.01초입니다. 일반 숨김/포화/geometry/local/cancel은 유지됐습니다. app `clippy --lib -- -D warnings` exit 0, 2.54초, authored 1파일 exact fmt/추적 diff exit 0입니다. 실제 PTY·fixture·기존 wheel의 변경 없는 성공은 재사용합니다.

buffer epoch와 rows epoch의 이 세 경계만 확인했습니다. mouse tracking mode 자체의 off/on ABA·columns-only resize·encoding 사이 변경·pre-Session query/focus 전체 순서와 실제 OS/aggregate는 여전히 미완료입니다.

### 다음 제품 연결의 원본 대조

`src/features/terminal/terminal-view.tsx`와 `docs/features/terminal.md` §8을 확인했습니다. TS는 SearchAddon을 생성·로드·dispose하지만 검색 바/`findNext` 공개 handle은 미구현입니다. 새로운 검색 UI를 만들어 원본 parity 완료로 계산하지 않습니다. 후보 spike의 실제 Alacritty search 성공과 native TerminalCore의 공개 search API 미연결은 구분합니다. [Alacritty 0.26.0 Term API](https://docs.rs/alacritty_terminal/0.26.0/alacritty_terminal/term/struct.Term.html#method.search_next)도 조회했습니다.

실제 TS 메뉴 `terminal-context-menu.tsx`의 copy/paste/select-all/clear·네 방향 split·new terminal·kill과 닫힌 뒤 focus 복구가 제품 연결 대상입니다. native `terminal_surface.rs`에는 아직 context-menu 배선이 없습니다. 복사/붙여넣기 handle 및 기존 native input 처리만으로 해당 메뉴의 parity를 완료 처리하지 않습니다. 다음 구현은 기존 N4 terminal/N5 전체 화면 계약에 해당하는 실제 메뉴·링크·project Hub 연결이며, raw 순서/세대의 미완료를 면제하지 않습니다.

- [ ] raw packet이 아직 Session 순서 표식을 예약하기 전의 query/focus와 순서, output parse→Dispatcher 시점, 중복 event의 일부 소비·UI 주입은 남습니다. 캡처 순번과 등록된 Session/query FIFO를 하나의 전체 입력 순서 증명으로 합치지 않습니다.
- [ ] buffer/mode ABA와 인코딩 사이 변경, retired/full queue 조합·닫힌 viewport·다중 window/button·forced selection, 포화 오류 뒤 복구와 aggregate/RSS는 남습니다.
- [ ] 실제 OS/보조 창·DPR/transform 픽셀·IME/AX/VT/GUI/성능, context/search/link/project Hub·213 TS view/full cutover/TS 제거와 M8 전체는 미완료입니다.

보호 실기 bundle·사용자 데이터/clipboard·OS 설정·기존 TS/root/MSRV를 유지했습니다. M8 N1~N8은 0/8이며 전체 완료 뒤만 commit·push합니다.
