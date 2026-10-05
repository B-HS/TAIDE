# M8 native 터미널 전환·새 터미널 keymap

## 대상·근거

- 대상은 `native/taide-native-app/src/{shell_keymap,terminal_surface,application}.rs`, `native/taide-native-ui/src/commands.rs`입니다. PROCESS N4-C의 기존41 action 연결 범위에서 main이 직접 구현했습니다.
- 원본 `editor-area.tsx`의 toggleTerminal, `terminal-tab-targets.ts`의 fallback, `command-palette.tsx`의 new-terminal, `layout.query.ts`의 useOpenTerminalTab과 기존 native HostCommand::NewTerminal을 대조했습니다. 공통 APP_KEYMAP의 Ctrl+Backquote와 Ctrl+Shift+Backquote를 그대로 소비합니다.

## 구현

- [x] focused shell project/pane의 active tab이 terminal이면 strip의 첫 non-terminal을 활성화합니다. terminal만 있으면 no-op이며 다른 terminal로 순환하지 않습니다. non-terminal/없는 active이면 첫 terminal을 활성화하고 없을 때 새 terminal을 만듭니다. pinned 탭도 원본처럼 선택 대상입니다.
- [x] new-terminal은 기존 terminal 유무와 무관하게 항상 새 탭 생성 intent를 보냅니다. 빈 pane에서 active ID를 요구하지 않으며 잘못된 focused pane은 거절합니다. App의 기존 intent→bounded host→layout_open_tab 경로와 locale title·preview=false·sessionId 빈 문자열·cwd 미지정 정책을 재사용합니다. 실제 PTY 시작은 기존 attach 경로이며 단축키 handler가 직접 process를 실행하지 않습니다.
- [x] 프로젝트 없는 new-terminal도 키를 소비하고 기존 locale `app.openProjectFirst`를 ShowOpenProjectNotice intent로 보냅니다. 현재 native status에 표시하며 원본 toast의 화면/수명 재현까지 완료했다고 주장하지 않습니다. toggle-terminal은 프로젝트가 없으면 소비하지 않습니다.
- [x] dependency/unsafe/suppression/code comment·제품 TS/root/MSRV·보호 bundle·OS 설정·사용자 파일을 변경하지 않았습니다.

## 실제 검증

Cargo는 기존 CARGO_HOME·locked/offline·`experiments/native-shell-spike/target`으로 직렬 실행했습니다.

1. `cargo test --manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target --lib terminal_tab_keymap -- --nocapture`: 최초 compile E0308(&String 문자열 pattern)·E0502(test의 mutable leaf borrow 수명)을 발견해 as_str/별도 borrow로 수정했습니다. 최종 compile4.01초,1 PASS·suite0.01초입니다. 실제 Runtime ActivateTab, 첫 대상·pinned·all-terminal/empty/invalid-active/missing-pane·프로젝트 없는 intent·Shift의 물리 Backquote fallback, 실제 HostBridge 연속 두 신규 탭/고유 ID·preview/pinned/cwd/session·타 프로젝트 불변·worker 종료/task0을 확인했습니다. 이 host 검사에서 PTY를 attach/spawn하거나 clipboard를 읽고 쓰지 않았습니다.
2. 독립된 no-project input gate는 `--lib terminal_tab_gate` compile2.66초,1 PASS·suite0.01초입니다. 실제 egui raw Backtick/physical key·Ctrl/Shift에서 프로젝트 없는 toggle 비소비와 new-terminal 단일 action을 확인했습니다. 기존 successful host 검사를 다시 실행하지 않았습니다.
3. app lib `cargo clippy ... --lib -- -D warnings` exit0(1.49초), 변경4파일의 exact rustfmt check·tracked diff check exit0입니다. 기존 Wry17 warnings는 dependency warning이며 authored strict 실패가 아닙니다.

기존 terminal tab/startup/승인 attach/PTY lifetime·group/Save/Close All·editor mirror·APP41 resolver 성공은 각 QA에서 재사용합니다. 새 keymap target/host 검사와 독립된 no-project adapter 검사만 실행했습니다.

## 미완료 gate

- [ ] 원본 no-project toast UI/수명, 새 탭→실제 NativeApplication loading/attach/selection/focus·원본 재표시 정책·집계 큐 포화/late input·빠른 연속 command·프로젝트 교체/닫힘·aux window target·OS layout/IME/AX/실제 키를 검증합니다. 구성 요소 host 성공을 전체 앱/GUI 완료로 계산하지 않습니다.
- [ ] 나머지13개 공통 action·전체 Monaco21 내장·팔레트/상태/UI 연결과 N1~N8 전체 gate를 이어갑니다. 현재26 shell action+terminal 자체2 이동은 기본 연결 수이며 전체41/실기 완료율이 아닙니다. remount A/B는 응답 대기입니다.
- [ ] 전체M8 N1~N8 0/8·213view/cutover/TS 제거·성능/보안/배포는 미완료이며 전체완료 뒤만 commit/push합니다.

## 문서 검증

docs가 제외되는 기본 Prettier 대신 이 QA만 `--ignore-path /dev/null`로 실제 검사합니다. untracked authored 파일은 no-index whitespace 출력으로 구분하고 내용 차이 exit1을 whitespace 실패로 세지 않습니다.
