# M8 창 단위 keymap과 기존 shell action 연결

## 대상과 계약

- `native/taide-native-app/src/keymap.rs`, `shell_keymap.rs`, `terminal_surface.rs`, `application.rs`, `lib.rs`, `native/taide-native-ui/src/editor_surface.rs`가 대상입니다. 원본 APP_KEYMAP/dispatch/chord store, `editor-area.tsx`, `app-shell.tsx`, `project-shell.tsx`의 실제 handler와 기존 ShellIntent/Controller worker를 사용했습니다.
- PROCESS N4-C의 공통 창 입력/기존 shell action 항목 안에서 main이 직접 구현했습니다. workflow·서브에이전트·OS 실기 앱·클립보드·입력기/VoiceOver는 사용하거나 조작하지 않았습니다. 제품 TS/root manifest/MSRV·보호 bundle은 유지했습니다.
- 이전 결정기/41개 카탈로그/기본 terminal 결과는 `2026-10-03-m8-native-terminal-keymap.md`가 정본입니다. 그 이후의 창 입력 연결을 이 문서에서 구분합니다.

## 기본 연결 결과

- [x] viewport별 Keymap에 현재 frame의 raw event index→Decision을 캐시합니다. terminal/editor/global listener는 같은 physical 이벤트에 대해 같은 결정을 읽으며 chord 상태를 다시 전이하지 않습니다. 다음 frame에서 cache/문자 소비 상태를 비우고 닫힌 viewport/shutdown에서 회수합니다. egui가 보정한 repeat 값은 raw index 비교에서 제외해 실제 repeat를 다른 이벤트로 오인하지 않습니다.
- [x] terminal의 `show_with_keymap`, editor의 `show_with_keymap`와 NativeApplication의 남은 입력 경로를 같은 결정기에 연결했습니다. editor gate는 현재 실제 widget focus와 canonical composition을 사용하고 terminal은 현재 preedit/focus를 사용합니다. 같은 raw Key 다음의 동일 logical Text 소비 규칙을 공통화했습니다. 기존 standalone editor show는 항상 통과하는 gate로 기존 편집 처리를 유지합니다.
- [x] `close-tab`, `toggle-sidebar`, `split`, `tab-cycle-next/prev`, `editor-next/previous`의 7개 기존 action을 실제 NativeApplication→ShellIntent 경로에 연결했습니다. focused shell/project/pane의 active tab과 원본 right split·tab wrap을 사용합니다. 닫기는 기존 request_tab_close의 pinned 경고/dirty 확인을 우회하는 direct CloseTab 명령으로 바꾸지 않습니다.
- [x] sidebar는 window Zen 중 소비 후 no-op이며 focused project의 shell_view만 바꿉니다. 빈 active/no focused project는 mutation을 만들지 않습니다. focused shell이 없으면 close/split/cycle의 단일 dispatch handler는 존재하지 않는 것으로 처리해 키를 삼키지 않고, window-level sidebar handler와 무조건 소비하는 chord 단계는 별도로 유지합니다.
- [x] 에디터의 사용자 지정 bare J가 실제 action 한 번만 생성하고 문서에 j를 삽입하지 않으며, 일반 X와 합성 IME의 J/漢 commit은 문서 `x漢`을 유지합니다. actual 2-pass run_ui와 같은 이벤트를 보는 두 listener에서 중복 상태 전이/동작을 방지했습니다.

## 실제 검사

Cargo는 기존 `CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo`, `--locked --offline --target-dir experiments/native-shell-spike/target`으로 직렬 실행했습니다.

1. app `cargo check --lib`: 실제 NativeApplication/두 surface 배선 compile exit 0(2.54초)입니다. 이후 관련 test/strict가 최종 변경 상태를 다시 compile합니다.
2. app `--lib window_keymap`: 1 PASS(compile 4.45초·suite 0.01초)입니다. 두 listener가 같은 prefix를 보아도 둘 다 EnterChord이며, 다음 frame의 actual egui repeat/raw index 0은 Ignore, 다음 ArrowRight는 같은 ResolveChord입니다. window retain 회수도 확인했습니다.
3. app `--lib shell_keymap`: 최종 1 PASS(compile 1.91초·suite 0.04초)입니다. 실제 NativeEditor/Core의 bare J/Text 소비·일반 입력·합성 IME·2-pass, no focused shell의 Cmd/Ctrl+W 비소비와 기존 ShellController worker의 sidebar 변경·prev wrap·right split·다른 프로젝트 불변·Zen/no-focus no-op을 확인했습니다. close는 실제 target TabId의 RequestCloseTab intent를 확인하고 dirty/pinned dialog의 실기 통과로 확대하지 않습니다. worker join·TaskSupervisor 0을 확인하고 이 fixture에서 생성한 전용 임시 data dir만 회수했습니다.
4. app `--test terminal-host terminal_commands`: 새 window coordinator의 영향 1 PASS(compile 9.44초·suite 0.57초)입니다. 40 OSC133 commands·default/custom/chord/no-match·2-pass 이동·PTY epoch 불변·exit Some(0)/dispatch join/task 회수를 유지했습니다. 이후 no-focused-shell helper 변경은 standalone terminal show/명령 이동 경로를 바꾸지 않았으므로 이 성공을 재사용합니다.
5. native-ui `--test editor_surface 실제_editor_surface`: standalone wrapper 영향 1 PASS(compile 1.24초·suite 0.02초)입니다. 입력·선택·IME commit·stale 거절을 유지했습니다. native-ui lib/해당 test strict exit 0(0.65초)입니다. 앱 strict 결과는 아래 최종 정적 검사에 기록합니다.

초기 app unit test compile은 fixture가 egui 0.36.2의 Preedit에 없는 `cursor`를 사용해 E0559로 실패했습니다. 서로 다른 두 filter 명령이 같은 compile 오류로 막혔으며 테스트 통과나 두 번의 실측으로 세지 않습니다. 설치된 source의 `active_range_chars`로 정정한 후 해당 신규 검사들이 통과했습니다. 이후 no-focused-shell handler 경계를 추가해 변경된 shell 검사만 다시 실행했습니다. 기존 성공을 3회 반복하지 않았습니다.

최종 app lib/terminal-host strict exit 0(2.83초), authored Rust 6파일 exact rustfmt check와 tracked diff check exit 0입니다. 기존 Wry 17 경고는 authored strict 통과와 구분하며 suppression·unsafe·의존성을 추가하지 않았습니다. JSON 카탈로그는 변경하지 않아 원본 41개 exact 비교 성공을 재사용합니다. live Cargo handle은 없습니다.

프로젝트 `.prettierignore`는 docs 전체를 제외하므로 기본 옵션의 문서 write/check를 실제 문서 검사로 계산하지 않습니다. 이번 두 keymap QA만 `--ignore-path /dev/null`의 명시된 범위로 확인해 실제 Prettier check exit 0입니다. authored Rust/두 QA의 명시된 untracked 8개 no-index whitespace 출력도 모두 비어 있습니다(exit 1은 /dev/null과 내용 차이).

## 남은 원본 동등성

- [ ] 연결된 7개 action과 terminal 이동 2개를 전체 41개 실행/command palette 완료로 세지 않습니다. Save의 override/legacy editor shortcut 일관성, 나머지 action/command-binding rows·palette UI·Monaco 명령 대응·chord 표시/no-match 알림은 다음 구현입니다.
- [ ] Explorer draft/search/dialog/임베디드 webview 등 다른 text 입력보다 앞서는 전체 window capture ordering, 같은 frame의 click/focus 전환·source shell slot/auxiliary scope, 여러 listener의 부수 효과 정확히 한 번 및 handler registry 전체를 검사합니다. 이번 coordinator는 결정 상태의 한 번 전이를 증명하지만 모든 future handler의 한 번 실행을 자동으로 보장했다고 주장하지 않습니다.
- [ ] 기존 no-op/단일 탭·pinned/dirty close·close confirmation·queue-full·shutdown/late scope·aux 실제 창과 전체 OS/IME/AX를 완료로 세지 않습니다. 현재 실제 Main App 배선은 compile와 해당 component/worker 검사이며 NativeApplication 전체 창을 실행한 검사가 아닙니다.
- [ ] raw Unicode/dead key/다중 codepoint Text provenance, Copy/Cut/Paste semantic 선변환, non-macOS Super와 raw index를 찾을 수 없는 synthetic event, override/cache aggregate·CPU/lock budget은 terminal-keymap QA와 전체 raw input/성능 gate에 남습니다.
- [ ] 원본 remount replay와 단일 Core 파싱의 A/B 질문은 응답 대기이며 임의로 선택하지 않았습니다. 전체 M8 N1~N8 0/8·213 view/cutover/TS 제거·성능/보안/배포는 미완료입니다. 전체 완료 후에만 commit/push합니다.
