# M8 Snippets 반응형 배치·목록 스타일

## 대상·원본

대상은 `native/taide-native-ui/src/snippet-editor.rs`와 실제 `tests/snippet-editor.rs`입니다. [Dialog Presence QA](2026-10-06-m8-snippet-dialog-presence.md)의 후속이며 기존 Snippets UI 두 번째 단계의 일부입니다.

원본 `snippet-editor.tsx`, `new-snippet-file-dialog.tsx`, `snippet-entry-editor.tsx`, `snippet-file-list.tsx`, `button.tsx`, `dialog.tsx`, `alert-dialog.tsx`, `global.css`를 대조했습니다. 실제 radius 매핑은 기본6px, sm4px, lg8px입니다. 다른 shadcn 프로젝트의 radius 공식을 적용하지 않았습니다. 설치된 egui0.36.2 Button/Atom/Ui/Frame/Style 원천과 [공식 Layout 문서](https://docs.rs/egui/0.36.2/egui/struct.Layout.html)를 확인했습니다.

## 구현

1. 새 파일/기본 Alert는640px 미만에서 viewport-32px 너비,640px 이상에서 최대512px입니다. 모바일 footer는 Confirm 위/Cancel 아래로 전체 너비를 쓰며 desktop은 오른쪽 행입니다. 작은 폐기 Alert는320px·Cancel 왼쪽/Confirm 오른쪽의 동일 너비 두 열입니다. 작은 viewport의 폐기 Dialog 최대 너비 세부는 아직 최종 대조하지 않았습니다.
2. 모바일/작은 Alert의 제목을 중앙 정렬하고 일반 desktop 제목은 왼쪽입니다. New 제목18px line-height18, Alert 제목18px line-height28, 설명14px line-height20과 원본 header/form/footer 간격을 적용했습니다. Close는 절대 배치 `Ui.place`와16px 크기이며 더 이상 부모 layout을 진행시키지 않습니다.
3. 잘못된 `snippetEditor.listTitle`을 실제 `snippetEditor.snippetListTitle`로 수정했습니다. 빈 파일 목록 안내, muted/medium 제목,4px 파일 간격·좌측 정렬·30px 행과4px 입력 반경을 적용했습니다. sidebar256px 안에 우측1px border를 포함시켜 실제 파일 content223px을 보존하고 카드의 border 이중 차감도 제거했습니다.
4. 공용 outline background/hover는 app.background/list.hoverBackground/list.foreground, primary/destructive hover는90% alpha, destructive 글자는 white입니다. medium·disabled50%와 원본 xs icon14px/padding6px/gap4를 연결했습니다. hover 판단은 설치된 SDK Button과 같이 동일 자동 ID의 실제 이전 response를 읽으며 강제 hover/클릭은 없습니다. 모든 focus ring/shadow/transition을 완료했다고 주장하지 않습니다.

## 검증·실패

- [x] 실제 responsive 연속 검사1 RED→GREEN:600/640/1000px의 최종 Dialog width568/512/512, 모바일 footer full-width/세로 순서·desktop 가로 순서·버튼36px·폐기320px/동일 두 열을 실제 Settings 클릭으로 확인했습니다. 최초 viewport600의 렌더 rect520px이 원본568px과 달라 실패했고, responsive 분기/Close layout/footer를 수정한 뒤 build1.57초/suite.35초·1PASS/filtered4입니다. 동일 geometry 성공을 다시 실행하지 않았습니다.
- [x] 원본 목록/style 실제 렌더 검사1 PASS: 빈 안내2곳, 실제 locale 제목,4px 간격·파일명 좌측 padding12px·입력 frame radius4를 확인했습니다(build.59초/suite.14초). 최초 존재하지 않는 `Editor.invalidate`를 fixture에 사용해 E0599(exit101)이었고, 생산 API를 추가하거나 inspection 상태를 주입하지 않고 실제 Back→Settings→Manage 재진입으로 수정했습니다.
- [x] 이후 실제 sidebar/card geometry 변경 위험에만 위 style 검사를 확장하여 파일 content223px·카드가 container-304px 전체 폭을 쓰는 것을 확인했습니다(build1.25초/suite.15초·1PASS/filtered5). 이전 입력 상태를 같은 코드로 반복한 것이 아니라 새로운 width 변경의 직접 검증입니다. 성공한 responsive/기존 입력 suite는 재실행하지 않았습니다.
- [x] form/Close/footer와 버튼 변경 영향의 기존 실제 검사4건 PASS(build1.47초/suite.15초·filtered1): Tab/Shift-Tab·Cancel autofocus/drop, scrim 도형/clip, Picker/global input/합성 IME/disabled, 저장 거절/retry/초안/삭제/unmount입니다. 이 실행은 이전 코드의 결과와 구별하며 후속 카드 width만 바꾼 뒤 같은 suite를 반복하지 않았습니다.
- [x] normal Canvas Wasm 최초 style 상태 check.57초, 후속 sidebar/card geometry 최종 check.54초 exit0·경고0입니다. Rust2fmt/대상 diff와 문서 포맷은 exit0입니다. native-host UI는 실제 검사 target으로 컴파일했으며 이번 전체 App check/실제 Chrome 완료로 대신 표기하지 않습니다. 검사 본문의 동일 기대값을 이름 있는 상수로 옮긴 뒤에는 의미가 같으므로 성공을 반복하지 않았습니다.

```sh
CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo /Users/hyunseokbyun/development/rust/cargo/bin/cargo test --manifest-path native/taide-native-ui/Cargo.toml --locked --offline --features inspection --target-dir /private/tmp/taide-m8-menu-build.j6Efnw --test snippet-editor snippet_dialog의_원본_640px -- --nocapture
CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo /Users/hyunseokbyun/development/rust/cargo/bin/cargo test --manifest-path native/taide-native-ui/Cargo.toml --locked --offline --features inspection --target-dir /private/tmp/taide-m8-menu-build.j6Efnw --test snippet-editor snippet_목록의 -- --nocapture
CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo /Users/hyunseokbyun/development/rust/cargo/bin/cargo test --manifest-path native/taide-native-ui/Cargo.toml --locked --offline --features inspection --target-dir /private/tmp/taide-m8-menu-build.j6Efnw --test snippet-editor -- --skip snippet_dialog의_원본_640px
```

## 남은 범위·현재 상태

후속 [outline·작은 Dialog QA](2026-10-06-m8-snippet-outline-and-small-dialog.md)에 Close 일반 focus 링·outline shadow와300px 폐기 Dialog 너비를 구현·검증했습니다. 아래 남은 범위는 이 경계 당시 기록이며 최신 실제 범위는 후속 QA를 따릅니다.

원본 focus-visible ring/outline shadow/버튼 padding·전환/글꼴 semibold·global input line-height·작은 viewport 폐기 너비·전체 AX bounds/theme/DPI/픽셀·전체 포커스 그래프는 남습니다. 실제 Chrome Snippets 입력/Toast/쓰기·종료 연속 검사는 아직 실행하지 않았으며 다음 harness에 전역 snippet_list fixture를 반영해야 합니다. 자동완성 UI/삽입·전체 native shutdown, 나머지 Settings/App/assets/최종 gate와 사용자 담당 마지막 CJK/VoiceOver 실기도 남습니다.

## 후속: 실제 Close·Trash 아이콘 tint

- [x] Close는 원본 foreground70%, pointer hover 시100%입니다. 실제16px Button response 위에 원본 SVG를 그리고 Button 키/AX label/절대 배치는 유지합니다.
- [x] Trash는 원본 muted→hover error이며24px Button/14px SVG·기존 Tooltip wrapper와 label을 유지합니다. 버튼 밖 테스트 상태/강제 hover 대신 실제 response의 hover로 현재 프레임 색상을 그립니다.
- [x] 신규 실제 Settings 클릭/PointerMoved 연속1 최종 PASS(build.59초/suite.12초·filtered6)입니다. 실제 paint-list의 icon tint를 resolved theme와 비교했습니다. raster 픽셀 검사와 다릅니다. fixture가 회전 없는 Image도 Mesh라고 가정하여 두 번 missing icon으로 실패했습니다. 설치된 SDK Image.paint_texture_at는 이 경우 textured Rect를 내보내므로 brush가 있는 Rect.fill을 읽도록 수정했습니다. 같은 가정의 세 번째 반복/검사 억제/성공 재실행은 없습니다. 생산 동작 수정 전의 첫 실패는 tint 불일치 재현으로 해석하지 않습니다.
- [x] 변경된 최종 Canvas Wasm.55초 exit0·경고0입니다. 선행 responsive/style/기존 입력 성공은 당시 소스 근거로 재사용하며 같은 전체 suite를 또 실행하지 않았습니다. 이 변경은 전체 AX/포커스·Chrome 완료를 뜻하지 않습니다.

```sh
CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo /Users/hyunseokbyun/development/rust/cargo/bin/cargo test --manifest-path native/taide-native-ui/Cargo.toml --locked --offline --features inspection --target-dir /private/tmp/taide-m8-menu-build.j6Efnw --test snippet-editor snippet_close와 -- --nocapture
```

이번 responsive·목록 경계의 구현/직접 검사/기록3/3(100%)은 전체 UI 완료가 아닙니다. Snippets1/4(25%)·provider2/4(50%)·M8 363/433(83.83%)·최종0/8·전체 ETA 산정 보류입니다. goal active·main 직접·workflow/서브에이전트 없음·전체 M8 완료 전 Git 없음입니다. 제품 TypeScript/OS/Keychain/보호 앱/사용자 데이터/의존성·lock·MSRV는 변경하지 않았습니다.
