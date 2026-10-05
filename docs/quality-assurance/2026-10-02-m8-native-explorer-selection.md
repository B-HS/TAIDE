# M8 native 탐색기의 선택·타이핑 검색

## 원본과 변경

- 원본은 `src/shared/lib/{list-selection,typeahead}.ts`와 해당 테스트, `src/features/explorer/file-tree.tsx`, `src/features/explorer/explorer-shortcuts.ts`입니다. 대상은 `native/taide-native-app/src/explorer.rs`, `tests/explorer.rs`입니다.
- primary path와 selected set·anchor를 구분했습니다. 일반 클릭은 단일 선택, Shift는 anchor부터 clicked까지 범위, Command/Ctrl은 추가/해제, Command/Ctrl+Shift는 기존 선택에 범위 추가입니다. 추가 선택 해제의 primary는 display order에서 마지막 남은 항목입니다. 다음 클릭 때 숨긴 행을 제거하고 숨긴 anchor의 Shift는 일반 클릭 규칙으로 돌아갑니다. 선택 수정키 클릭은 열기/Toggle을 호출하지 않습니다.
- 전체 행 배경·WidgetInfo는 selected set을 사용합니다. 우클릭은 해당 행 하나를 선택하고 blank 우클릭/double click은 set·anchor·primary를 모두 비웁니다. 기존 public selected의 외부 설정은 다음 show/start_create/moved에서 단일 선택 상태와 동기화합니다. 생성/rename 성공·rename 시작은 단일 선택, moved는 선택한 모든 display path와 anchor를 함께 전환합니다. 원본의 row action이 primary/context row만 사용하므로 임의 batch delete/open은 추가하지 않습니다.
- 방향키는 정확한 shortcut 우선 판정을 유지한 뒤 추가 modifier와 관계없이 이동합니다. 이동할 행이 있으면 단일 선택하고 경계를 넘으면 기존 multi-selection을 유지합니다. Right는 펼치기/직접 첫 child, Left는 접기/parent를 사용합니다. 원본과 같은 select/reveal 경로를 사용하지만 nearest scroll/픽셀 동등성을 확인한 것은 아닙니다.
- 타이핑 검색은 현재 primary부터 순환해 case-insensitive prefix를 찾습니다. no-match도 버퍼와 700ms deadline을 갱신합니다. deadline은 egui input.time으로 결정하며 화면이 다시 처리될 때 경과한 버퍼를 비웁니다. 별도 timer worker/repaint/sleep은 없습니다. 원본의 JS key.length===1에 맞춰 UTF-16 한 unit Text만 받으며 Shift는 허용, Alt/Control/Command·Paste·여러 글자 Text·surrogate pair는 제외합니다. printable key의 실제 Text와 브라우저 keydown이 모든 OS/keyboard layout에서 동일하다고 주장하지 않습니다.
- 트리 focus/활성 UI·popup/inline 입력 gate를 유지합니다. preedit를 다음 frame까지 기억해 composition 중의 방향키/검색/생성 shortcut을 차단하고 commit frame도 검색하지 않습니다. 이는 headless event 경계이며 실제 CJK IME gate 통과가 아닙니다.

## 확인한 공식 source

고정 egui 0.36.2의 `data/input/{event,event_filter,ime_event}.rs`, `memory/mod.rs`와 egui-winit 0.36.2의 `on_keyboard_input`을 실제 registry에서 읽었습니다. Text와 Key의 중복 가능성, modifier event, UTF-16과 logical/physical fallback의 차이, focus-lock 적용의 previous-frame 조건을 기준으로 합니다. 새 의존성·제품 MSRV 변경은 없습니다.

## 실제 검증과 실패

Cargo 공통 옵션은 `--manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target`입니다.

- 최초 compile은 fixture의 TreeRowPage.total을 usize로 설정해 실패했습니다. 실제 u32 DTO에 checked conversion을 사용했으며 실행 성공으로 세지 않습니다.
- [x] 최초 `cargo test ... --test explorer 탐색기_선택검색`는 신규 3건 중 포인터 1건 PASS, 방향키 1건 FAIL, 타이핑 1건 FAIL이며 전체 10.03초였습니다. 포인터는 range/add/remove/anchor/hidden row/secondary·blank/moved/rename/public primary 동기화를 확인했습니다.
- 방향키 실패 원인은 기본 egui 방향키 focus navigation이 tree 선택 변경과 함께 실행되는 것이었습니다. tree의 horizontal/vertical focus-lock과 처음 focus를 받은 frame의 pending navigation 취소를 연결했습니다. Tab은 가두지 않습니다.
- IME 실패는 ui.input closure 안에서 tree.has_focus를 다시 조회해 egui context lock에 재진입한 것입니다. focus를 closure 밖에서 먼저 읽도록 수정했습니다. 10초 lock timeout을 성공으로 세지 않습니다.
- 수정 뒤 첫 방향키 명령은 잘못된 filter로 0건이 실행됐습니다. 이를 PASS로 세지 않고 실제 filter `방향키는_수정키`로 관련 1건만 실행해 PASS, 0.01초였습니다. boundary의 multi-selection 보존·Shift/Control 이동·Command+Down 열기·선택 없음의 마지막 행·directory child/parent/Toggle을 확인했습니다.
- [x] `cargo test ... --test explorer 타이핑은_대소문자` 관련 1건 PASS, 0.01초입니다. prefix 누적/대소문자/순환/현재 행 우선/no-match/결정적 deadline, modifier·Paste·다중 Text/surrogate 제외, focus 상실/복귀, preedit 지속/commit frame·생성 shortcut 보호, primary 없는 Text를 확인했습니다.
- [x] hidden selection filter의 전행 반복 검색을 source와 같은 display-row 순회/set lookup으로 정리했습니다. 이 후속 변경의 포인터 관련 1건만 PASS, 0.04초입니다. 방향키/타이핑·이전 생성/삭제/실제 Trash·GUI startup 성공은 반복하지 않았습니다.
- [x] 최종 lib/bin/explorer strict clippy exit 0, 0.76초입니다.
- [x] app cargo fmt --check·해당 QA Prettier·git diff --check는 exit 0입니다. PROCESS/HANDOFF는 활성 관련 상태만 갱신했습니다.

## 남은 동등성 게이트

- [x] 후속 같은 frame의 방향키/shortcut/Text 순서·반복/Space 중복을 별도로 재현·수정했습니다. 실제 결과·fixture의 source 기대값 오류·추가 생성/rename 전환 검사는 `2026-10-02-m8-native-explorer-key-order.md`에 있습니다. composition 혼합/전체 keyboard 동등성이 완료된 것은 아닙니다.
- [ ] 실제 empty/긴 virtual tree·nearest scroll·여러 pane/window의 selection/typeahead focus scope·OS dead-key/keyboard layout/CJK IME·VoiceOver·tree AX/multi-selection semantics·픽셀 동등성은 남습니다. 실제 시스템 설정과 사용자 실기 bundle을 조작하지 않았습니다.
- [ ] 모든 context/clipboard/shortcut·Git·auto reveal/icon/theme·full Explorer와 전체 213 view/editor/LSP/terminal·성능/보안/beta/rollback/배포·TS 제거/Rust99%/M8 N1~N8은 미완료입니다. 이 좁은 입력 구현 완료를 전체 M8 완료로 세지 않습니다.

실제 사용자 파일/clipboard/Trash·OS 입력기/VoiceOver·실기 앱/bundle을 변경하지 않았으며 commit/push는 하지 않았습니다.
