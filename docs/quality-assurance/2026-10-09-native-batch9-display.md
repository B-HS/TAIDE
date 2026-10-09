# 배치 9 — 편집기 표시 옵션 1 (2026-10-09)

상태: 옵션 공급·공백/rulers·캐럿·스크롤 구현, 변경 크레이트 전체 검사·실패 선별 재검사, 브라우저 host/Wasm·포맷·diff·디스크 확인을 마쳤습니다. 선별 커밋·일반 푸시 진행 중입니다. 서브에이전트·workflow 없이 메인이 직렬 수행합니다.

## 기준과 범위

`src/features/editor/code-editor.tsx`의 옵션 갱신과 `src/shared/lib/code-editor-settings.ts`·`crates/taide-model/src/settings.rs`의 공급 계약을 기준으로 합니다. `node_modules/monaco-editor/esm/vs`의 실제 0.56.0 소스와 vendored egui 0.36.2의 `Painter`, registry epaint 0.36.2의 `Mesh` API를 읽었습니다. 의존성을 추가하지 않습니다.

| 옵션 | 원본 규칙 | 이번 경계 |
| --- | --- | --- |
| 공백 | none/boundary/selection/all, 기본 selection. Monaco 기본 experimentalWhitespaceRendering은 svg. 공백 원은 spaceWidth/7 반지름, 탭은 한 문자 폭의 화살표. 선택 범위는 끝 제외이며 줄 투영의 가짜 들여쓰기는 생략합니다. boundary는 단일 내부 공백과 wrap 뒤로 이어지는 단일 마지막 공백을 생략합니다. | native-host 표시 모듈, 실제 row의 모델 바이트→galley 좌표 사용 |
| rulers | 열 × typicalHalfwidthCharacterWidth, 1px inset 선, 스크롤 높이(최대 1,000,000px). TS는 ruler 색을 매핑하지 않아 Monaco 기본 dark #5a5a5a/light #d3d3d3 사용 | native-host 설정 공급·테마 기본값, 문서 가로 스크롤과 텍스트 clip 적용 |
| 캐럿 | TS는 line/block/underline, blink/smooth/phase/expand/solid만 노출. line 기본 2px을 DPR의 정수 물리 픽셀에 맞추며, block은 grapheme 폭(탭/EOF는 반각 폭), underline은 2px. 선택 머리에도 표시. 포커스 없음·조합 중 숨김, 읽기 전용 solid | 뷰별 임시 상태·원본 타이밍·다중 커서·IME 경계 |
| 캐럿 시간 | flat blink 500ms, CSS 모드 500ms 지연 뒤 500ms ease-in-out alternate 20회. smooth 이동 80ms CSS ease, 커서 개수 변경 시 이동 애니메이션 중단. block 글자색은 editorCursor.background 미지정 시 cursor 색의 반전 | 기존 css_motion 보간 재사용, 필요한 시점만 repaint 요청 |
| 스크롤 | scrollBeyondLastLine 기본 true, 문서 높이에 max(0, viewportHeight-lineHeight-paddingBottom) 추가. smoothScrolling 기본 false; true이면 125ms ease-out cubic, 장거리 분할과 연속 입력의 목표 누적 | 기존 뷰 scroll·wrap/접기·드러내기·휠/스크롤바 경로 |

추가 모드·새 디자인은 넣지 않습니다. 기존 `EditorAppearance` 필수 필드와 `NativeEditor`·`EditorRequest` 공개 서명을 보존하며 기본 presentation의 호환 경로를 유지합니다. 브라우저는 native-host 분기로 기존 word-wrap 공급·렌더 동작을 유지합니다. remote-web 소스·manifest·lockfile과 editor/UI 의존 그래프는 동결입니다.

원본 경로: `viewParts/whitespace/whitespace.js`, `viewParts/rulers/rulers.js`, `viewParts/viewCursors/viewCursor.js`·`viewCursors.js`·`viewCursors.css`, `common/core/editorColorRegistry.js`, `common/viewLayout/viewLayout.js`, `base/common/scrollable.js`, `base/browser/dom.js`입니다. 직접 대응한 원본 라이선스는 기존 Monaco MIT 고지와 함께 최종 diff에서 확인합니다.

## 검증 계획과 보호

실제 row/shape·설정 변경·시간 프레임을 사용하는 메모리 egui 회귀로 공백/탭/Unicode/wrap/선택/clip, rulers 좌표, 캐럿 폭/스타일/깜빡임/읽기 전용/IME/뷰 수명, 스크롤 높이/보간/복원을 확인합니다. 변경 크레이트 전체 대상은 --no-fail-fast로 한 번씩 직접 실행하며 Cargo는 한 번에 하나만 사용합니다. 실패 영향만 선별 재검사합니다. host/Wasm 브라우저 동결 컴파일·포맷·diff·디스크를 확인합니다.

app 전체 검사에서는 기존 보호 Trash 3건을 정확히 제외합니다. 실제 앱 데이터·클립보드·Keychain·OS 설정·Trash·보호 spike 앱을 건드리지 않습니다. OS 합성 입력을 사용하지 않으며 실제 UI/OS IME·RTL·접근성·대형 파일 실기·ignored 성능 검사는 실행하지 않은 부채로 남깁니다.

## 실행 결과

1. UI 전체: `cargo test --manifest-path native/taide-native-ui/Cargo.toml --features inspection --no-fail-fast --locked --offline --target-dir experiments/native-shell-spike/target` exit 0, 286 통과·0 실패·0 ignored, 17 대상, 빌드 7.42초입니다. `/private/tmp/taide-batch9-ui-full-20261009.log`입니다. 표시 target의 14건과 기존 위젯·문서·키·접기·구문 회귀가 포함됩니다.
2. app 전체: `cargo test --manifest-path native/taide-native-app/Cargo.toml --no-fail-fast --locked --offline --target-dir experiments/native-shell-spike/target -- --skip 'remote_files::tests::실제_파일14명령과_raw는_plugin_overlay_저장_복사_이동_삭제_mirror를_보존한다' --skip '실제_탐색기_삭제는_확정한_dirty를_회수하고_선택프로젝트만_닫으며_공유초안을_보존한다' --skip '실제_workspace_delete는_dirty_mirror_root를_거절하고_모든_프로젝트_tab과_document를_회수한다'` exit 0, 612 통과·0 실패·보호 3 제외, 67 대상, 빌드 39.86초입니다. `/private/tmp/taide-batch9-app-full-20261009.log`입니다. 기존 loopback·감시·ImageIO 경계 때문에 승인된 샌드박스 밖에서 실행했으며 보호 3건을 실행하지 않았습니다. 테마 갱신 검사에서 모든 builtin 테마의 공백·스크롤바 공급을 확인했습니다.
3. vendored egui 전체: UI manifest에서 `-p egui --no-fail-fast --locked --offline --target-dir experiments/native-shell-spike/target`로 직접 한 번 실행했습니다. 단위 50 통과, 문서 157 통과·10 실패·1 ignored, 전체 exit 101입니다. `/private/tmp/taide-batch9-egui-full-20261009.log`입니다. 10건 모두 SDK 배포에서 누락된 ferris/icon 이미지의 include_bytes 경로 오류이며 수정한 입력 API의 오류가 아닙니다. 아래 복원 후 해당 문서 범위만 재검사합니다.
4. remote-web host: `cargo check --tests --manifest-path native/taide-remote-web/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target` exit 0, 2.92초입니다. `/private/tmp/taide-batch9-remote-check-20261009.log`입니다. manifest·lockfile·remote-web 소스 diff는 없습니다.

표시 옵션 전체 대응률이나 전체 기능 완료율로 해석하지 않습니다. bracket/guide·sticky scroll·minimap·진단/overview·주입/블록·LSP 등은 다음 범위입니다.

## 재현·원인·수정

- 최초 표시 target은 테스트에서 구형 IME 입력을 가정해 컴파일 실패했습니다. 실제 `ImeEvent::Preedit { text, active_range_chars }`를 읽어 수정했습니다. 그 다음 5 통과·3 실패는 테스트가 갤리 커서의 정수 좌표와 독립 문자 측정의 소수 폭을 같다고 가정한 문제였습니다. 실제 폰트 측정과 Monaco의 n/space 규칙에 맞춘 뒤 8건 통과했습니다. `/private/tmp/taide-batch9-display-first-20261009.log`·`display-second`·`display-metrics`입니다. 제품의 좌표/폭을 잘못된 기대값에 맞춰 바꾸지 않았습니다.
- 실제 Line 휠 40px이 첫 프레임에 12.32676px만 반영된 실패를 `/private/tmp/taide-batch9-wheel-before-20261009.log`(exit 101)에 남겼습니다. egui의 미소비 휠 보간 잔량을 native 렌더가 함께 회수하는 `InputState::take_scroll_delta_immediate`를 추가했습니다. 기존 입력 처리와 브라우저 호출 경로는 바꾸지 않습니다. native 옵션이 false이면 즉시, true이면 한 번의 125ms 보간을 적용하며 touch phase/정밀 델타는 즉시 반영합니다. Start에 실린 첫 델타도 잃지 않습니다. 후속 프레임 잔류와 연속 목표 누적을 검사했습니다.
- 아래 여백을 끄고 가로 스크롤바가 있는 11줄 문서에서 마지막 12px이 가려졌습니다. 예상 112px 대신 100px에 멈춘 실패는 `/private/tmp/taide-batch9-horizontal-before-20261009.log`(exit 101)입니다. 원본의 가로 바 높이 반영·콘텐츠 폭 갱신에 맞춰 native content height/슬라이더/row 좌표를 함께 보정한 뒤 UI 전체 대상의 회귀가 통과했습니다. 기본 호환 presentation은 기존 화면을 유지합니다.
- 10만 줄을 160만 px 스크롤하면 원본의 ruler DOM 높이 상한 때문에 선이 현재 화면을 덮지 못했습니다. `/private/tmp/taide-batch9-ruler-large-before-20261009.log`에서 실패(exit 101)했습니다. 사용자의 원본 버그 재현 불필요 지시에 따라 native에서는 전체 콘텐츠 높이와 viewport clip을 사용합니다. 새 회귀 1건만 재검사해 통과(exit 0, 빌드 1.45초)했습니다. `/private/tmp/taide-batch9-ruler-large-after-20261009.log`입니다. UI의 서로 다른 최종 통과는 287건이며 전체 대상을 다시 실행한 수치가 아닙니다.

## SDK 문서 이미지 복원

egui/eframe registry의 `.cargo_vcs_info.json`이 가리키는 원본 커밋 `49682f8baa058bf49e011035cfbd6e825f88a5ef`를 확인했습니다. 원본 [Ferris](https://github.com/emilk/egui/blob/49682f8baa058bf49e011035cfbd6e825f88a5ef/crates/egui/assets/ferris.png) 46,286 bytes·blob SHA-1 `8741baa19d02a003db73c33bd779a324bc1a57fa`, [icon](https://github.com/emilk/egui/blob/49682f8baa058bf49e011035cfbd6e825f88a5ef/crates/eframe/data/icon.png) 12,052 bytes·`4ce7cc588ecd1b3a0bf81a79dc04677c21e9bf98`와 다운로드/registry 사본을 각각 대조했습니다. MIT 원문과 각 provenance README를 보존했습니다. 새 eframe fixture 폴더는 Cargo 패키지가 아니며 기존 eframe patch·의존 그래프·앱 런타임을 바꾸지 않습니다.

선별 문서 재검사는 `-p egui --doc` 뒤 image 필터 7 통과·기존 1 ignored(4.32초), atomics::atom 필터 6 통과(3.34초), 모두 exit 0입니다. 최초 실패 10건 전부 해소했으며 성공 재사용 후 서로 다른 최종 결과는 단위 50·문서 167 통과·문서 1 ignored입니다. `../bug/2026-10-09-vendored-egui-doc-images.md`와 `/private/tmp/taide-batch9-egui-doc-image-after-20261009.log`·`taide-batch9-egui-doc-atom-after-20261009.log`입니다.

## 최종 동결·포맷·디스크

- remote-web Wasm canvas: `cargo check --manifest-path native/taide-remote-web/Cargo.toml --target wasm32-unknown-unknown --features canvas --locked --offline --target-dir experiments/native-shell-spike/target` exit 0, 1.60초입니다. `/private/tmp/taide-batch9-remote-wasm-check-20261009.log`입니다.
- 대형 ruler 변경 뒤 앱 --tests 최종 컴파일은 exit 0, 12.41초입니다. `/private/tmp/taide-batch9-app-final-check-20261009.log`입니다. UI 전체 대상 성공은 재사용하고 새 대형 ruler 회귀만 선별 검사했습니다.
- UI/app package 포맷 검사와 수정한 SDK 입력 파일의 rustfmt --check는 각각 exit 0입니다. 늦게 추가한 대형 ruler 회귀의 포맷을 정리한 뒤 개별 검사로 확인했습니다. 소스 의미는 바꾸지 않았습니다.
- `git diff --exit-code 6f7294f5`로 remote-web 소스와 UI/editor/app manifest·lockfile에 변경 없음(exit 0), 이번 native 소스와 고지의 whitespace diff도 exit 0입니다. 디스크 여유 679GiB·사용률 63%이며 산출물을 지우지 않았습니다.
- 보호 Trash 3건·ignored SDK 문서 1건·기존 ignored 성능·실제 UI/OS 입력·IME/RTL/접근성/대형 실기·soak는 완료로 표시하지 않습니다. OS 합성 입력·실제 사용자 데이터 접근은 하지 않았습니다.
