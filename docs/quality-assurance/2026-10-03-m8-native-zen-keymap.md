# M8 native Zen keymap·Escape·전체화면 전환

## 대상·원본 근거

- 대상은 `native/taide-native-app/src/{zen,shell_keymap,application,terminal_surface,lib}.rs`, `native/taide-native-ui/src/editor_surface.rs`입니다. PROCESS N4-C 기존41 action 연결을 main이 직접 수행했습니다.
- 원본 `use-window-chrome.ts`, APP_KEYMAP/dispatch와 `useGlobalKeymap`, Monaco `coreCommands.js`/`cursorMoveCommands.js`를 대조했습니다. Zen은 window chrome이고 default Cmd/Ctrl+K→Z는 !terminalFocus입니다. editorTextFocus에서 app chord는 전역 실행하지 않고 Monaco로 넘깁니다. 원본에는 editor Zen mirror가 없으므로 임의로 추가하지 않았습니다.
- 원본 Escape는 다른 handler의 preventDefault 뒤 bubble 단계에서 실행하며 IME는 제외합니다. Monaco는 Escape/Shift+Escape로 secondary cursor 제거를 우선하고, 단일 non-empty selection은 head로 축소합니다. 빈 단일 cursor에서는 Zen의 Escape가 실행될 수 있습니다.
- egui0.36.2의 Modal::should_close는 Escape를 consume하며 Memory::top_modal_layer는 직전 frame 값입니다. 후자를 현재 modal 판정에 사용하면 이미 닫힌 modal이 다음 Escape를 막는 차이가 생겨 사용하지 않습니다. 공식 viewport Fullscreen(bool) 명령과 고정 source를 읽었습니다.

## 구현

- [x] no-project에서도 공통 keymap→기존 SetWindowChrome mutation으로 Zen을 toggle합니다. project layout/sidebar state를 직접 바꾸지 않고 sidebarRailCollapsed 등 다른 window chrome 축은 보존합니다. 기존 native shell의 Zen rail/panel/status 숨김을 그대로 소비합니다.
- [x] NativeEditor의 Escape는 secondary cursor를 제거하되 primary selection을 보존하고, 다음 Escape는 단일 선택을 head로 축소합니다. undo group을 분리하며 scroll/folds·문서 text/revision을 유지합니다. 빈 selection의 Escape는 소비하지 않고 상위 창으로 남깁니다. 기존 composition branch의 소유권은 바꾸지 않습니다.
- [x] App에서 editor/explorer/global keymap·delete/close Modal 뒤 남은 Escape만 처리합니다. 최초/현재 modal·저장/삭제 busy 상태는 실제 App pending 상태로 차단하고 popup/raw focus/IME frame/editor composition도 제외합니다. 현존 dialog를 소비하지 않고 직전 modal layer만으로 이후 frame을 막지도 않습니다.
- [x] fullscreen은 최초 실제 desired값(zen && opt-in)을 seed하고 첫 frame에서는 명령을 내지 않습니다. Zen/설정의 computed값이 바뀔 때만 현재 viewport로 Fullscreen을 보냅니다. Zen 중 setting off→exit, setting on→enter와 Zen exit→false를 연결하며 수동 OS fullscreen 상태를 첫 draw의 false 명령으로 해제하지 않습니다. 종료 중에는 새 전환을 내지 않습니다.
- [x] dependency/unsafe/suppression/comment·제품 TS/root/MSRV·보호 bundle·사용자 데이터·OS fullscreen/IME/VoiceOver/clipboard를 조작하지 않았습니다.

## 실제 검증

Cargo는 기존 CARGO_HOME·locked/offline·격리 target으로 직렬 실행했습니다. 서로 다른 위험을 검사하며 같은 successful 시나리오를 반복하지 않았습니다.

1. app `--lib zen_keymap -- --nocapture` 신규1 PASS(compile3.93초·suite0.06초)입니다. no-project runtime toggle/기타 chrome 보존, 실제 egui global chord 단일 action·editor deferral no Zen, opt-in transition/동일값 no-op/복원 첫 draw no-op의 viewport 명령을 확인했습니다. 실제 Modal의 Escape 소비 뒤 Zen no-op, actual NativeEditor의 secondary cursor→selection collapse→남은 Escape 전달과 Core text/revision 불변을 확인했습니다.
2. 첫 fixture의 deprecated ImeEvent::Enabled와 test 이름 non_snake_case 경고를 suppression 없이 제거했습니다. 기존 성공의 runtime/chord/viewport/editor branch는 다시 실행하지 않고 재사용하며 IME를 현재 API의 별도 최소 `--lib zen_ime`로 검사했습니다. 신규1 PASS(compile2.72초·suite0.01초)이며 Preedit/Commit frame의 Escape가 Zen으로 전달되지 않습니다. 합성 IME이지 실제 OS 입력기가 아닙니다.
3. Memory의 이전-frame modal 의존을 제거하고 current App pending 상태를 사용한 변경은 `--lib zen_modal_exit` 신규1 PASS(compile2.89초·suite0.02초)입니다. 직전 frame modal layer가 남아 있어도 지금 modal이 없으면 새 Escape를 전달합니다. 기존 successful Zen 전체 검사를 반복한 것이 아닙니다.

최종 app lib strict exit0(0.91초), UI lib strict exit0(0.42초), 변경6파일 exact rustfmt/추적diff exit0입니다. 이전 app strict1.38초 뒤 modal 상태 판정이 바뀌어 최종 해당 정적 검사만 수행했습니다. 기존 Wry17 경고는 dependency 경고이고 새 authored test warning은 마지막 신규 검사 compile에서 없었습니다. 이전 group/Save/Close All/terminal/reopen/font/editor 입력의 변경 없는 성공은 해당 QA에서 재사용합니다.

## 남은 gate

- [ ] 실제 NativeApplication controller snapshot→shell chrome→OS fullscreen/첫 launch 수동 fullscreen 보존·설정 live 교체·focus/selection/readonly/Shift+Escape·popup·모든 busy/dialog/여러 raw event/키 해제·queue-full/late·OS IME/AX·aux/다중 project를 검증합니다. headless viewport 명령을 실제 OS fullscreen 성공으로 확대하지 않습니다.
- [ ] 원본 editor default Zen chord는 동작하지 않는 그대로 재현했으며 palette bridge·status/monaco no-match 알림·custom overrides·전체 Monaco21을 이어서 구현합니다. native raw IME frame guard와 OS event provenance 전체 검증도 남습니다.
- [ ] 현재30 shell action+terminal 자체2 이동은 기본 연결 수이며 전체41/실기 완료율이 아닙니다. 나머지9 공통 action·전체M8 N1~N8 0/8·213view/cutover/TS제거·성능/보안/배포는 미완료입니다. remount A/B는 응답 대기이며 전체완료 뒤만 commit/push합니다.

## 문서 검사

docs 제외 기본 설정 대신 이 QA만 `prettier --ignore-path /dev/null`로 실제 포맷 검사합니다. untracked authored 파일의 no-index 내용 차이 exit1과 whitespace 출력을 구분합니다.
