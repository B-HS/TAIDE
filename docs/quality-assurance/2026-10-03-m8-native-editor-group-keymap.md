# M8 native editor의 그룹 chord mirror

## 대상·원본 근거

- 대상은 `native/taide-native-app/src/keymap.rs`이며 PROCESS N4-C의 기존41 action 연결 중 editor group mirror를 main이 직접 구현했습니다. 기존 NativeEditor→공통 Windows.route→ShellIntent 경로를 사용합니다.
- 원본 `editor-group-shortcut-actions.ts`, `monaco-group-shortcut-actions.ts`와 tests, `use-global-keymap.ts`, `monaco-keybinding.ts`, 설치된 Monaco의 `abstractKeybindingService.js`와 `keybindingResolver.js`를 읽었습니다. 에디터 focus에서 전역 chord는 Observe/Defer하며 7개 그룹/이동/Close All chord를 editorTextFocus action으로 별도 등록합니다.
- 1~9 포커스는 단일 키라 mirror에 포함하지 않습니다. override가 chord를 없애거나 Monaco KeyCode가 없는 stage를 사용하면 editor binding을 만들지 않습니다. 중복 editor 규칙은 마지막 등록이 우선합니다. 원본 global capture가 단일 Save를 먼저 소비하면 뒤 editor binding이 그것을 빼앗지 않습니다.

## 구현

- [x] effective APP entries에서 정확한7개·유효한2단 key만 editor resolver에 연결했습니다. 전역 resolver의 Decide/Observe/Defer/기존 handler 우선순위는 그대로 두고 소비되지 않은 실제 editor 키만 mirror에 전달합니다. standalone/global/terminal에 editor chord를 강제로 켜지 않습니다.
- [x] editor pending prefix는 viewport별 Keymap에 유지하고 같은 frame/raw index의 결과·이미 소비한 여부를 별도 cache로 공유합니다. 두 editor listener가 같은 mirror resolve를 봐도 실행 handler는 한 번 호출합니다. viewport retain/shutdown에서 같은 registry를 회수합니다. 전역 일반 handler 전체의 exactly-once를 증명했다고 확대하지 않습니다.
- [x] 단일 키 override는 기존 global dispatcher만 실행합니다. editor binding 충돌은 reverse 등록 순서, modifier는 Monaco CtrlCmd/WinCtrl 구분, 입력 가능한 KeyCode는 실제 원본 mapping을 사용합니다. modifier-only·합성 composition의 non-command 키는 pending을 유지하며 Monaco가 repeat를 별도 거절하지 않는 경로도 맞췄습니다.
- [x] 기존 egui event→DOM key/physical code/modifier 변환을 두 resolver가 공유합니다. 원본 카탈로그/JSON/settings/MSRV/root/제품 TS·보호 bundle·OS clipboard/browser/IME/VoiceOver는 변경하지 않았고 dependency/unsafe/suppression을 추가하지 않았습니다.

## 실제 검사

Cargo는 기존 CARGO_HOME·locked/offline·격리 target으로 직렬 실행했습니다.

1. app `--lib editor_group`의 실제 NativeEditor focus/2-pass Cmd/Ctrl+K→Cmd/Ctrl+Right는 변경 전 action `[]`≠`[focus-group-right]` RED 1 FAIL(compile 2.82초·suite 0.02초), 수정 뒤 1 PASS(compile 3.04초·suite 0.02초)입니다. 실제 widget focus와 단일 action/문서 텍스트 불변·legacy Save 미요청을 확인했습니다. 동일 성공은 다시 실행하지 않았습니다.
2. app `--lib editor_group_override`는 신규1 PASS(compile 2.63초·suite 0.01초)입니다. 같은창 two-listener의 재지정 J/H·left/right 충돌의 later-right 승리, single override 단독 실행, Unknown stage 비등록, bare H composition ignore/후속 resolve, global Save suffix 우선, modifier-only/6초 만료·non-mac 기본 modifier 투영·repeat·F13/비ASCII KeyCode 거절·viewport retain을 확인했습니다. 합성 Keymap/Window 검사가 실제 OS 입력기/다른 플랫폼 GUI 검증은 아닙니다.
3. 변경된 공통 route cache의 기존 app `--lib window_keymap` 영향은1 PASS(compile 0.21초·suite 0.00초)입니다. neutral/editor 두 listener에 같은 global prefix/repeat/resolve를 공유해 기존 pending 상태를 바꾸지 않습니다. app lib strict exit0(0.91초), 대상 exact fmt/추적 diff exit0입니다. 기존 Wry17 경고는 authored strict와 구분합니다.

기존 group target/runtime·Close All coordinator/host/dialog·Save/PTY/41 catalog/기본 egui adapter의 변경 없는 성공은 해당 QA에서 재사용합니다. 새 NativeEditor 기본 검사와 독립된 override/cache 검사를 구분했고 통과한 검사를 세 번 반복하지 않았습니다.

## 남은 전체 gate

- [ ] 전체 Monaco의 내장21개 Cmd+K 명령·open-keybindings/Zen action·command-binding rows와 palette/미일치·chord 상태 표시를 연결합니다. 7개 mirror만으로 전체 editor namespace나 전체41 action을 완료로 세지 않습니다.
- [ ] 원본의 500ms chord checker/document focus loss·5초 근처 timer와 native의 이벤트 시각 만료 차이, 실제 focus/창·aux·keyboard layout/nonASCII provenance·OS IME/AX·override/규칙 교체·aggregate/CPU/RSS를 전체 gate에서 확인합니다. 이번 lazy timeout 단위 검사를 타이머/실기 전체로 확대하지 않습니다.
- [ ] 실제 NativeApplication group focus 후 Core view/selection/PTY/close/save 수명, 초기/숨김/재표시 capture ordering·여러 global handler의 부수 효과, remount A/B 결정은 남습니다. 전체 M8 N1~N8 0/8·213 view/cutover/TS 제거·성능/보안/배포는 미완료이며 전체 완료 뒤만 commit/push합니다.

## 정적 기록

docs를 제외하는 기본 Prettier 대신 이 QA만 `--ignore-path /dev/null`로 명시한 write unchanged/check exit0입니다. authored Rust/이 QA의 no-index whitespace 출력은 비어 있으며 /dev/null 내용 차이 exit1은 실패로 분류하지 않습니다.
