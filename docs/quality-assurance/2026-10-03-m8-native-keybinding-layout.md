# M8 keybindings 행·컨트롤·modal 배치 후속

## 대상 파일

- `native/taide-native-app/src/keybinding-editor.rs`
- 원본 읽기: `src/widgets/keybindings-editor/keybindings-editor.tsx`, `src/features/settings/keybinding-row.tsx`, `src/shared/ui/{dialog,button}.tsx`, `src/shared/styles/global.css`

## 리포트

원본의 네 열·버튼 그룹·검색과 필터·헤더·theme scrim/shadow·마지막 close focus 기본 배치를 연결했습니다. 기존 아이콘/capture/catalog/host/search 성공은 보존하며 변경된 UI 위험만 검사했습니다. 실제 전체 픽셀·접근성·일반 focus trap·toast·상위 M8 완료는 아닙니다. N1~N8은0/8이고 commit/push는 전체 완료 뒤에만 수행합니다.

## 상세

1. row의 binding/source/control 폭을 실제 monospace12·proportional12/10으로 구분해 측정합니다. 네 열 gap12, binding 내부 gap6, control 내부 gap4, label 세로 gap2와 when truncate를 적용했습니다. Change는 outline24px·radius6이고 Reset/Unbind는 ghost24px·glyph12입니다. 공통 버튼 style은 다중 소비에서만 공유하며 정확한 title font family·모든 narrow width의 CSS grid min-content·overflow 보존은 아래 gate에 남습니다.
2. chord/conflict badge는10px·line15·padding4x2·theme15% fill, row capture는12px/line18·padding8x4·border1·height28을 사용합니다. conflict 설명은 glyph와 해제 버튼 폭을 먼저 예약하고 나머지 영역에서 truncate합니다. Capture focus lock/confirm pointer-down 보호와 typed output/비낙관적 override는 유지합니다.
3. SearchByKey는 Keyboard14·text12·24px control·gap4·padding6을 사용하고 검색 행 gap8과 query font13·margin8x4를 연결했습니다. 필터는24px rounded pill·gap8·warning15% fill/40% border와 실제 count/기존 filter 상태입니다. context pill과 header command flex/key/source 우측 배치·대문자를 연결하고 세 header cell을 같은15px 높이로 배치했습니다. 실제 browser font와 tracking/완전한 query line-height·모든 hover transition은 아직 동등성 완료가 아닙니다.
4. `app.shadow`의50%를 scrim으로 쓰고 동일색 offset0/8·blur24·spread0으로 원본 전용 그림자를 구성합니다. Frame의 content+padding+stroke 합산을 반영해 정상1000x800 fixture의 modal 외곽을768x560으로 유지합니다. close glyph16은 외곽 top/right16이며 content 순서 마지막에 focusable interaction을 생성합니다. hover100%/기본70%와 theme focus ring2·offset2를 연결합니다. animation zoom/exit·전체 OS/GPU raster 차이는 남습니다.
5. query에서 ShiftTab/마지막 close에서 Tab일 때에만 순환 target을 지정합니다. popup/capture/IME/disabled 경우는 이 경계를 적용하지 않고 기존 popup와 캡처 보호를 유지합니다. 이것은 양끝 기본 순환 검사이지 모든 offscreen row/auxiliary/동일frame/multi-pass/AccessKit focus trap의 전체 증거가 아닙니다.

## 실제 검사

모든 Cargo는 app manifest·`CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo`·기존 target·`--locked --offline`·serial입니다.

- [x] 신규 `--lib keybinding_modal_frame`:theme scrim과 ShiftTab 순환2 RED(suite0.21초)를 재현했습니다. 수정 뒤 focus 순환·close click1 PASS와 theme 검사1 FAIL(suite0.25초)을 구분했습니다. 남은 theme 실패는 installed Area의 fade-in 중간 색을 최종색으로 가정한 검사기 오류였으며 `RawInput.time=1.0`의 결정적 settled frame으로 수정했습니다. theme/그림자·768x560·close inset1 PASS(suite0.19초)이고 후속 실제 layout 변경 영향의 같은 검사1 PASS(suite0.16초)를 기록합니다. animation을 제품에서 비활성화하지 않았습니다.
- [x] 신규 `--lib keybinding_row_grid`:실제 원본 버튼 중심 간격28px 대비 native36px RED(suite0.07초)를 재현했습니다. 그룹만 분리한 뒤 ghost 기본 패딩으로32px이 남았고(suite0.06초), 고정 icon 버튼의 패딩을 제거해1 PASS(suite0.05초)입니다. 실제 Reset/Unbind 중심 간격·동일 y와 binding/source 열 gap12를 확인했습니다.
- [x] 신규 `--lib keybinding_controls`:Keyboard14와 pill·헤더·filter 입력 검사에서 command/key y가318.5/314.0인 RED(suite0.06초)를 재현했습니다. uniform15px 셀로 수정 후1 PASS(suite0.08초)입니다. 실제 pill24px·둥근 radius·클릭 후 unassigned 상태와 no-results도 확인했습니다.
- [x] 변경된 row/control/검색 입력 영향의 `--lib keybinding_editor는`:1 PASS(suite0.58초). 캡처·confirm/save·실제 reset icon click·key search·Escape/IME/focus/context 흐름입니다. capture 버튼 ID/레이아웃 변경 영향의 `--lib keybinding_modal_input은_캡처`:1 PASS(suite0.21초), Tab chord 저장과 search ArrowDown focus 유지입니다. popup Escape·model/host/search/DPR 성공은 반복하지 않았습니다.
- [x] 최종 `cargo clippy ... --lib --bins --tests -- -D warnings`:exit0(9.98초). exact authored rustfmt exit0입니다. Wry의 inherited17 warnings는 authored strict와 구분합니다. 새 검사기 TextureId Option E0308과 filter closure의 E0500 compile 실패는 실행 통과로 세지 않았습니다. filter state 결정은 local로 분리해 단일 ConflictIndex를 재사용합니다.

## 남은 gate

후속 toast 기본은 `docs/quality-assurance/2026-10-03-m8-native-toasts.md`가 정본입니다. 아래 toast 잔여는 전체 animation/AX/전 mutation/late/full App/aux를 뜻하며 기본 연결 완료를 취소하지 않습니다.

- [ ] 실제 원본 font family/CJK·narrow viewport·row min-content와 고정 modal overflow·title/query/inspector line-height·tracking·scrollbar/pr8·tooltip bottom·모든 hover/focus/button shadow/zoom/exit를 재현하고 실제 픽셀/AX/OS로 확인합니다.
- [ ] 모든 focus 이동·offscreen row/스크롤·같은frame pointer→key·capture 전환/blur·multi-pass exactly-once·full App/aux/disabled/실제 IME를 확인합니다.
- [ ] warning/Settings 실패의 원본 toast·연속 저장/late event/창 반영·나머지8 action/Monaco/213view와 상위 N1~N8/cutover/TS 제거·성능/보안/배포를 마무리합니다. remount A/B는 응답 대기입니다.

이 후속에서 새 dependency·root manifest/lock·MSRV·보호 bundle·OS 설정·사용자 데이터·clipboard를 변경하거나 사용하지 않았습니다. 이 문서의 정상 viewport shape 검사만으로 전체 화면 동일성을 주장하지 않습니다.

최종 기록의 대상 QA/bug4개 Prettier exit0(모두 unchanged), 추적 whitespace exit0입니다. authored editor/icons/license·SVG5개·관련 QA/bug4개 no-index whitespace에는 진단이 없었습니다. `/dev/null`과 새 파일의 내용 차이 exit1을 검사 실패로 세지 않았습니다.

## 다음 toast 연결의 확인된 입력

`AppToaster`는 설정의 `toastPosition` 또는 bottom-right, theme.type, richColors와 closeButton을 사용합니다. `settings.query.ts`는 쓰기 실패 시 settings.saveFailed title과 describeIpcError description을 표시합니다. 설치 `node_modules/sonner/dist/index.js`의 기본값은 lifetime4000ms·width356·gap14·visible3입니다. middle 위치는 원본 global.css의 top50%/bottom auto/translateY(-50%) override를 별도로 대조해야 하며 아직 native toast 구현/실기 성공이 아닙니다. 원본 queue·hover/pause·close·접근성·theme 색상/애니메이션과 실제 Host 실패 provenance는 다음 구현에서 source를 추가로 읽습니다.
