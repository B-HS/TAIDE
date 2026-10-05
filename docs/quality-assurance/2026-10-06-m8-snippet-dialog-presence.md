# M8 Snippets Dialog Presence·키 입력

## 대상·원본

대상은 공용 `snippet-editor.rs`, 기존 `tooltip-motion.rs`, 실제 `tests/snippet-editor.rs`입니다. [캐시·후보 QA](2026-10-06-m8-snippet-catalog-and-candidates.md)의 후속입니다.

원본 `dialog.tsx`/`alert-dialog.tsx`, 설치된 `tw-animate-css`의 enter/exit 변수, `@radix-ui/react-dialog`/`react-alert-dialog`의 autofocus/loop/trapFocus, `global.css`의 scrim/shadow를 읽었습니다. egui0.36.2의 실제 `Modal`, `Area`, `Ui.interact`/`response`, layer transform과 Focus/EventFilter 구현을 읽고 같은 API를 사용했습니다.

## 구현

1. 콘텐츠는200ms CSS ease·opacity·scale95↔100, scrim은 기본150ms opacity입니다. 기존 Motion에 duration을 명시할 수 있게 했으며 Tooltip/Picker의 `new` 기본150ms·기존 계산식은 그대로입니다. native Area의 별도 fade-in을 꺼서 같은 전환을 이중 적용하지 않습니다.
2. 닫힘 뒤 콘텐츠를200ms 유지하고 그동안 뒤쪽 편집기/닫힌 액션의 중복 입력을 차단합니다. `Modal::default_area`와 같은 foreground/center/modal memory/dismissal 등록·frame/sense 경계에 직접 전환을 조립했습니다. scrim의 도형·hit rect·clip을 inverse-transform 좌표로 만들므로 콘텐츠 확대/축소가 전체 viewport scrim을 축소하지 않습니다.
3. 새 파일은 Picker/새 global input이 먼저 포커스를 받고 Alert는 Cancel이 먼저입니다. 원본 DOM 순서인 Picker→global input(있을 때)→Cancel→enabled Confirm→Close와 Alert Cancel→Confirm의 Tab/Shift-Tab 순환을 구현했습니다. Popup이 열려 있으면 기존 Picker가 키를 소유합니다. IME Escape/disabled/Alert 바깥 클릭 유지도 보존합니다.
4. 처음 포커스를 줄 때 기존 SDK의 원자적 `request_focus_with_filter`를 사용합니다. Tab은 NONE/SHIFT만 명시적으로 처리하여 `consume_key(NONE)`의 추가 Shift/Alt 무시 규칙에 의존하지 않습니다. 콘텐츠 제거/Editor drop은 transform·dismissal을 회수하고 그 layer의 포커스만 해제합니다. 원본 스니펫 Dialog에는 Radix Trigger가 없으므로 임의의 버튼 포커스 복귀를 추가하지 않았습니다. 제목은 원본18px, New header gap8/Alert gap6을 맞췄습니다.

## 검증·실패 이력

- [x] 신규 실제 Settings 연속1 최종 PASS(build1.38초/suite.13초·filtered2): 최초 scale95·전체 backdrop rect·중복 filename Confirm 제외·첫 Tab/Close/Picker loop·Shift-Tab 역순·Escape 후 retained 콘텐츠/뒤쪽 입력 차단·200ms 뒤 transform/dismissal 제거·Alert Cancel autofocus/바깥 클릭 비닫힘·Editor drop입니다. 현재 시간은 fixture의 실제 egui frame time이며 대기를 추가하지 않았습니다.
- [x] 실제 paint-list 도형/clip 검사1 첫 PASS(1.22초/.11초·filtered3): 확대 중 반투명 full-viewport scrim 도형과 실제 clip을 대조했습니다. raster/Chrome 픽셀 검사가 아닙니다. 당시 함수 이름의 `픽셀` 단어를 실제 범위인 `도형`으로 정정했으며 본문은 그대로이고 성공을 재실행하지 않았습니다.
- [x] 변경된 renderer의 기존 실제 입력 영향2 PASS(.55초/.12초·filtered1): 저장 거절/retry/초안/삭제/unmount와 nested Picker/global input/IME/disabled/create입니다. 삭제 뒤 뒤쪽 Back을 누르는 fixture에200ms 닫힘 완료 경계를 추가했습니다. 초기화/forced click으로 생산 동작을 우회하지 않았습니다. 이 검사들은 새 전환 변경의 영향을 확인한 것이며 선행 동일 상태 성공을 반복하지 않았습니다.
- [x] 최종 normal Canvas Wasm check.55초 exit0·Rust3fmt/diff exit0입니다. native-host 공용 UI는 위 실제 UI 검사 target으로 컴파일했습니다. App 전체9.42초 check는 캐시 경계 당시 성공이며 이번 Dialog 뒤 새 전체 App check라고 표기하지 않습니다. 초기 production unused Id warning은 inspection/test 조건 import로 수정했으며 최종 새 경고/억제는 없습니다.

신규 연속 검사는 통과 전 네 번 실패했습니다. 첫 실패는 `Ui.response`의 첫 pass Rect::NOTHING을 backdrop 현재 geometry로 기대한 것이며 실제 `interact` rect를 사용했습니다. 다음 두 번은 첫 Tab에서 Picker에 포커스가 남았습니다. 첫 Tab assertion을 초기 autofocus ID assertion으로 잘못 해석해 불필요한 trigger ID capture를 한 번 시도했고 효과가 없어 제거했습니다. 실제 오류 줄과 SDK를 다시 읽어 `set_focus_lock_filter`가 이전부터 포커스가 있던 위젯에만 적용된다는 원인을 확인하고 초기 포커스/filter를 원자적으로 설정했습니다. 다음 실패는 Shift-Tab이 일반 Tab으로 소비된 것이며 정확한 modifiers 경계로 수정했습니다. 마지막 성공 이후 같은 검사는 반복하지 않았습니다.

```sh
CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo /Users/hyunseokbyun/development/rust/cargo/bin/cargo test --manifest-path native/taide-native-ui/Cargo.toml --locked --offline --features inspection --test snippet-editor snippet_dialog은_200ms_presence_전체_scrim_tab순환_alert취소_포커스와_drop을_보존한다 --target-dir /private/tmp/taide-m8-menu-build.j6Efnw -- --exact
CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo /Users/hyunseokbyun/development/rust/cargo/bin/cargo test --manifest-path native/taide-native-ui/Cargo.toml --locked --offline --features inspection --test snippet-editor 실제 --target-dir /private/tmp/taide-m8-menu-build.j6Efnw
```

## 남은 범위

후속 [반응형·목록 QA](2026-10-06-m8-snippet-responsive-layout.md)에640px 배치·부분 token/geometry의 구현과 새 검증을 기록했습니다. 아래 목록은 이 Presence 경계 당시 미완료 기록이며 전체 AX/theme/DPI/포커스·Chrome/종료는 계속 남습니다.

- 원본 전체 시각 토큰/반응형 header·footer/카드/입력/AX bounds/theme/DPI, 빠른 재열기·상위 화면 unmount/다중 modal의 모든 포커스 그래프는 아직 완료하지 않았습니다. 닫힘 중 중복 액션 차단은 현재 수명 보호이며 모든 원본 closed DOM의 pointer 상호작용 동등성을 증명하지 않습니다.
- 신규 global cache·이번 Dialog가 포함된 실제 Chrome Snippets 연속 화면/입력/Toast/쓰기 거절·명시 retry/close drain과 native 전체 shutdown은 남습니다. 새 probe는 전역 `snippet_list` 추가 조회를 반영해야 합니다. 최신 bindings는 여전히 선행 키바인딩 source입니다.
- 실제 자동완성 추천 UI/삽입·placeholder, 나머지 Settings/App/assets/최종 N1~N8/TS 제거·Rust99%, 사용자 담당 CJK/VoiceOver 마지막 검증은 남습니다.

이번 전환·기본 키 입력 경계3/3(100%)·Snippets 전체1/4(25%, UI 부분 진행)·provider2/4(50%)·M8 363/433(83.83%)·최종0/8·전체 ETA 산정 보류입니다. goal active·main 직접·workflow/서브에이전트 없음·전체 M8 완료 전 Git 없음·live 검사 없음입니다. OS/Keychain/보호 앱/제품 TS/사용자 데이터/의존성·lock·MSRV는 변경하지 않았습니다.
