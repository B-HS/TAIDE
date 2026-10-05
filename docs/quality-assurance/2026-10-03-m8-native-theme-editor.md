# M8 native ThemeEditor·Settings 연결 검증

## 대상 파일

- `native/taide-native-app/src/theme-editor.rs`, `theme-editor-tokens.rs`, `theme-editor-tests.rs`, `theme-live-preview.rs`
- `native/taide-native-app/src/theme-edit.rs`, `theme-color-picker.rs`, `settings-view.rs`, `settings-view-tests.rs`
- `native/taide-native-app/src/host.rs`, `application.rs`, `presentation-refresh.rs`, `toast.rs`
- `native/taide-native-app/src/keybinding-icons.rs`, `keybinding-editor.rs`, `lib.rs`, `theme-draft.rs`, `resources/themes/`

## 리포트

이전 draft·서비스·ColorPicker primitive를 실제 native Settings에 연결했습니다. 생성·복제·편집 진입, 원본 token 순서·검색·색상/reset·bold/italic, 이름/변경 수/저장, 미저장·삭제 확인, local preview와 창별 draft palette 연결이 구현됐습니다. 기본 UI/host 연결의 완료이며 원본 픽셀·모든 Settings·M8 전체 완료 판정은 아닙니다.

기존 TypeScript 제품 코드는 수정하지 않았습니다. 사용자 보호 bundle, OS/IME/VoiceOver·clipboard·사용자 문서와 Cargo manifest/lock/MSRV는 이 단위에서 변경하지 않았습니다. Git branch/HEAD는 `to_rust_native` / `2824005573ea1e1e6a70136499016b14aa06a2bf`이며 목표 active·전체 M8 완료 뒤만 commit/push합니다.

## 실제 연결

1. TS `theme-tokens.ts`와 정확히 같은 21개 namespace·134개 color/31개 syntax/20개 terminal 순서를 native 상수로 고정했습니다. 테스트에서 문자열 순서를 원본 소스와 비교합니다. runtime은 TS를 읽거나 실행하지 않습니다. 검색은 원본처럼 namespace 또는 token에 적용하며 label·reset·syntax style/toggle와 hex picker를 조립했습니다. 숨긴 행의 picker는 폐기하여 다시 나타날 때 새 popup 상태를 사용합니다.
2. Settings는 기존 4section 대신 ThemeEditor가 전체 콘텐츠를 차지하는 takeover를 사용합니다. 정상 Settings로 돌아오면 scroll을 초기화하고 TOC state는 유지합니다. TAIDE 기본 card에는 복제 버튼이 없고 Bundled/custom에는 별도 Copy 버튼이 있습니다. 선택 영역과 복제 영역을 분리했고, custom 목록에 복제·Pencil 편집과 생성/폴더 열기를 연결했습니다.
3. load/save/delete 각각 `Arc<()>` operation identity와 기존 Session 폐기 ticket을 함께 검증합니다. 이전 실패 응답이 새 재시도의 pending을 지우거나 새 editor에 들어갈 수 없습니다. 성공한 save/delete만 editor를 닫고 catalog를 무효화합니다. catalog request는 owner/mount 외 generation도 비교하여 같은 mount의 이전 refresh를 거절합니다. `ThemeChanged`만 별도 catalog revision을 올리며 일반 Settings 변경마다 목록을 다시 읽지 않습니다.
4. 실제 64-capacity HostBridge에 typed Command/Reply를 연결했습니다. 제출 오류는 해당 요청의 실패 응답으로 pending을 해제합니다. theme command는 editor dirty flush 실패에 의해 사라지는 일반 file action 배열에 두지 않습니다. 원본 ThemeEditor 오류는 `describeIpcError`에 대응하는 native 원본/번역 제목만 표시하며 Settings의 `saveFailed` 제목을 붙이지 않습니다.
5. preview 값은 활성 owner와 viewport로 제한하고 가장 최근 변경된 editor가 해당 창의 palette를 소유합니다. 저장된 테마는 별도로 보존하며 close/hidden·shutdown과 최신 saved-theme refresh 후 draft palette를 다시 선택합니다. 기존 shell/editor/terminal/Settings/keybinding/preview/toast appearance 경로를 재사용하고 terminal query palette도 같은 경계를 사용합니다. local preview는 원본 tab·syntax 예문/16ANSI·cursor를 그립니다. 실제 NativeApplication 창의 완전한 renderer/구문/픽셀 동등성을 이 코드 연결로 통과했다고 주장하지 않습니다.

## 검증 명령과 결과

공통 Cargo 인자는 아래와 같습니다. 모든 Cargo 검사는 직렬로 실행했습니다.

```sh
CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo cargo test \
  --manifest-path native/taide-native-app/Cargo.toml --locked --offline \
  --target-dir experiments/native-shell-spike/target --lib FILTER -- --test-threads=1
```

| FILTER                               | 실제 결과와 범위                                                                                                                                                                                                |
| ------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `native_theme_editor`                | 최초 fixture E0308 `Arc<AppState>`/실제 cloneable `AppState` 오류를 정정했습니다. 신규 3 PASS, suite 0.22초/컴파일 8.10초: 원본 순서, dirty/요청별 실패·재시도·폐기, 실제 검색/bold/reset/취소/save 입력입니다. |
| `native_theme_editor는_실제_입력`    | 실제 trigger 폭·picker 재마운트 처리와 스타일 변경 뒤 영향 UI 1 PASS, suite 0.17초/0.21초입니다. 변경 없는 모델·순서 성공은 재사용합니다.                                                                       |
| `native_settings_theme`              | 생성 → 실제 HostBridge load/save → custom 목록 갱신/preview clear 1 PASS(0.24초/4.48초)입니다. 이후 custom 행 spacing·편집·확인 삭제 경계를 추가해 관련 1 PASS(0.26초/2.96초)이며 별개의 2건으로 세지 않습니다. |
| `native_settings_preview`            | 1 PASS, suite 0.18초/3.14초: viewport/활성 owner/hidden 제거와 미저장 3자리 hex·transparent의 aggregate palette, backend theme/settings 불변입니다.                                                             |
| `keybinding_icons는`                 | 변경된 공통 8SVG raster·배율/texture cache 교체·해제 1 PASS, suite 0.01초/0.21초입니다. 기존 keybinding focus RED를 재실행하거나 green으로 표시하지 않습니다.                                                   |
| `native_toast는_theme_오류`          | 1 PASS, suite 0.00초/0.21초: localized/raw theme 오류를 Settings 제목 없이 원본 제목으로 표시합니다. 나머지 toast 22건은 재사용합니다.                                                                          |
| `native_theme_editor는_picker`       | 실제 trigger → hex 입력 → reset에서 기본 `#89b4fa` 대신 입력 `#00ff00`이 남는 RED(exit101, suite0.10초/3.47초)를 재현했습니다. blur commit 뒤 reset을 적용하는 수정 후 동일 검사 1 PASS(0.10초/2.42초)입니다.   |
| `native_presentation_theme_revision` | 1 PASS, suite0.00초/3.64초: 일반 Settings와 ThemeChanged의 catalog revision 분리입니다.                                                                                                                         |
| `native_settings_view는_테마입력`    | 변경된 Settings 선택/목차/scroll/번역의 영향 1 PASS, suite0.08초/0.22초입니다.                                                                                                                                  |
| `native_settings_palette`            | 새 ThemeEditor/picker appearance가 포함된 모든 builtin palette 영향 1 PASS, suite0.04초/0.21초입니다.                                                                                                           |

서로 다른 관련 검사는 11건입니다. 성공 후 같은 입력·환경을 반복하는 추가 계측은 하지 않았고, 실제 코드/검사 범위가 바뀐 관련 검사만 다시 실행했습니다. 앞선 draft2·mutation3·ColorPicker2의 변경 없는 동작 성공을 재사용합니다.

- [x] 초기 authored strict의 cfg(test) `Id` 미사용 import 오류를 검사 억제 없이 조건부 import로 수정했습니다.
- [x] 중간 `cargo clippy … --lib --bins --tests -- -D warnings` exit0(12.79초)입니다. 최종 결과는 아래 최신 정적 검증 절에 기록합니다.
- [x] authored 16개 정확한 파일의 `rustfmt --check --edition 2024 --config skip_children=true` exit0입니다. vendor/전체 workspace 포맷은 하지 않았습니다.

### 최신 정적 검증

최종 `CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo cargo clippy --manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target --lib --bins --tests -- -D warnings` exit0, 11.56초입니다. authored 경고·검사 억제는 없으며 기존 Wry17 경고는 별도입니다. 전체 test suite는 실행하지 않았고 기존 keybinding Tab RED 때문에 전체 green이라고 표시하지 않습니다.

## SVG 출처

설치된 `lucide-react`1.28.0 `dist/esm/icons/copy.mjs`, `pencil.mjs`의 원본 node/속성에서 React의 `key` metadata만 제외했습니다. 24viewBox·stroke2·round cap/join과 실제 벡터 모양을 보존합니다. 기존 RotateCcw 자산을 14px 별도 슬롯으로 재사용하며 keybinding의 12px Reset은 유지합니다. 공통 raster/cache는 두 실제 사용처가 생겨 기존 모듈을 crate-level로 공유한 것이며 외부 image resolver는 계속 차단됩니다. Copy/Pencil의 ISC 고지는 `resources/themes/LICENSE.txt`에 보존했습니다.

## 남은 게이트

- [ ] 실제 NativeApplication GUI의 저장/삭제·원복/최신 base palette·aux 창·모든 renderer/구문 스타일과 terminal query 실기. 현재 preview 선택/DTO·palette 및 HostBridge headless 성공만 근거입니다. app 전체 성공 또는 모든 token의 최종 픽셀 영향을 주장하지 않습니다.
- [ ] 실제 host 포화·disconnect의 NativeApplication pending/toast, 완료된 worker와 close/이동·save/delete 동시 요청의 최종 UI 실기. 서비스 ticket/직렬 host와 요청별 admission의 검사로 OS race 완료를 대신하지 않습니다.
- [ ] popup 최초 keyboard/Tab/invalid hex 실제 blur·hue/drag Escape/window loss, syntax reset과 hex blur 동시 입력, discard 확인 버튼/반응형/locale/DPI/hover/focus/font/shadow/gradient·AX range/orientation/value text/action. 최소 재현 가능 상태를 유지하고 다음 실제 연결 위험에서만 검사합니다.
- [ ] 임의 전수 반복 대신 settings.json AppFile·나머지 Settings section/domain·전체213view/41action/Monaco21/palette·성능/보안/cutover·TS 제거/Rust99%·패키징/서명/notary/rollback을 계속 구현합니다. keybinding Tab RED·PTY remount 선택도 남아 있고 N1~N8은 0/8입니다.
