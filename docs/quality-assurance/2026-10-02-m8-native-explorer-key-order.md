# M8 native 탐색기의 같은 frame 키 입력 순서

## 대상·원본·실패

- 원본은 `src/features/explorer/file-tree.tsx`의 discrete handleKeyDown과 `explorer-shortcuts.ts`, `src/shared/lib/typeahead.ts`입니다. 대상은 `native/taide-native-app/src/explorer.rs`, `tests/explorer.rs`입니다. 기존 선택 입력 결과는 `2026-10-02-m8-native-explorer-selection.md`와 구분합니다.
- 기존 consume helper는 같은 Key의 pressed event를 전부 지우고 bool 하나를 반환했습니다. shortcut도 한 frame 전체에서 먼저 찾았으므로 앞선 방향키/문자보다 뒤의 shortcut이 먼저 실행될 수 있었습니다. 신규 actual egui event 재현에서 ArrowDown 두 번의 기대 card/실제 button으로 FAIL, 0.02초였습니다.
- 한 input frame의 events를 순회하며 매 Key/Text마다 최신 selection·modifier를 사용합니다. 같은 Key 반복은 각각 처리하고 실제 처리한 event만 제거합니다. 원본 shortcut의 exact modifier 우선 판정은 해당 Key마다 적용합니다. Text 앞의 실제 Key/ModifiersChanged를 사용하며 새 의존성·keymap·재바인딩 정책은 없습니다.
- Space shortcut 뒤의 중복 Text(" ")는 검색 버퍼에 넣지 않습니다. 다음 Key/Text부터 다시 정상 검색합니다. rename/create/pending parent가 시작되면 이후 트리 key 처리는 멈추고 기존 inline renderer에 맡깁니다. 실제 생성/rename worker·파일 정책은 변경하지 않았습니다.

## 실제 검증

Cargo 공통 옵션은 `--manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target`입니다.

- 신규 `cargo test ... --test explorer 동일frame_키입력` 최초 1건 FAIL, 0.02초였고 위 제품 수정 뒤 동일 재현의 반복 이동/shortcut/문자 순서는 통과했습니다. 이어 Space assertion에서 actual Open(Cn)/fixture 기대 Open(card)로 FAIL, 0.02초였습니다.
- 원본 검색은 현재 행부터 검사합니다. 당시 primary가 Cn이고 prefix c였으므로 Cn을 여는 실제 구현이 맞았습니다. source와 어긋난 fixture 기대 경로만 Cn으로 정정했습니다. 다음 a는 누적 ca로 card를 선택하는 assertion을 유지했고 제품의 현재 행 우선 규칙을 약화하지 않았습니다.
- [x] 최종 신규 단일 재현 PASS, 0.03초입니다. 두 방향키 반복, modifier 이동 뒤 Command+Down의 새 primary 열기, 같은 frame의 B/u 문자 누적, 검색 뒤 방향키, Space 중복 Text·이후 a 검색, 이동 뒤 rename·뒤 방향키의 선택 비변경, Escape 뒤 이동/create·뒤 방향키의 선택 비변경을 확인했습니다. 동일 성공은 반복하지 않았습니다.
- [x] dispatch 구조가 바뀐 방향키/타이핑 경계만 `cargo test ... --test explorer 탐색기_선택검색 -- --skip 포인터는_anchor`로 2건 PASS, 0.01초입니다. 코드가 바뀐 해당 위험의 검사이며 입력/코드가 같은 성공의 반복이 아닙니다. 기존 포인터·생성 worker·삭제/실제 Trash·GUI startup 검사는 재실행하지 않았습니다.
- [x] 제품 dispatch의 lib/bin/explorer strict clippy exit 0, 0.70초입니다. 후속 최종 fixture는 explorer test target만 strict clippy exit 0, 0.37초로 검사했으며 같은 제품 lib/bin 성공을 재사용했습니다.
- [x] app fmt --check·해당 QA Prettier·git diff --check는 exit 0입니다. PROCESS/HANDOFF는 현재 관련 상태만 갱신했습니다.

## 미완료 경계

- [ ] 브라우저 event.key와 egui-winit Text/Key의 OS dead-key·keyboard layout/physical fallback·key release/동시 modifier 경계, 같은 frame의 IME/다른 Key 혼합과 popup/다중 pane/window·modal·OS focus 순서는 실제 source/GUI 검증이 더 필요합니다. 현재 composition frame 전체 차단 보호를 IME event별 완전 동등성으로 계산하지 않습니다.
- [ ] clipboard/Cut/Copy/Paste·모든 context action/keyboard menu·전체 shortcut·표시/AX·실기·전체 M8 N1~N8/TS 제거/Rust99%는 미완료입니다.

검사는 headless egui의 합성 rows/events와 결정적 input.time입니다. 실제 사용자 파일·clipboard/Trash·OS 설정·실기 앱/bundle을 변경하지 않았고 commit/push는 하지 않았습니다.
