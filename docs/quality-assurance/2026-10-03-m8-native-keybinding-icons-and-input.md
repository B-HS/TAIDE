# M8 keybindings 원본 아이콘·popup/capture 입력 후속

이후 행/control·scrim/shadow·close/Tab 기본 배치는 `2026-10-03-m8-native-keybinding-layout.md`에서 이어집니다. 아래 잔여는 이 문서 작성 시점의 범위이며 전체 픽셀/focus/상위 gate는 여전히 미완료입니다.

## 대상 파일

- `native/taide-native-app/src/keybinding-editor.rs`, `keybinding-icons.rs`, `application.rs`
- `native/taide-native-app/resources/keybindings/{keyboard,rotate-ccw,triangle-alert,unlink,x}.svg`, `LICENSE-LUCIDE`

## 리포트

원본5개 Lucide 아이콘을 native 렌더링에 연결하고 popup Escape 우선권과 캡처 Tab/화살표 오류를 수정했습니다. 기존 model/host/search·theme 성공은 재사용했습니다. grid·전체 focus trap·toast·실제 픽셀/AX/OS와 상위 M8 완료는 아닙니다. N1~N8은0/8, 기본 shell31+terminal2 수는 유지합니다.

## 상세

설치 source는 `lucide-react`1.28.0(ISC)이며 CJS 파일 SHA-256은 `cf6ca1261384d8f603a9c3e8b362ca0dff4128dffa7de71f565623fab157e6ca`입니다.

1. 설치된 `node_modules/lucide-react/dist/cjs/lucide-react.js`의 Keyboard/RotateCcw/TriangleAlert/Unlink/X 정의를 기존 TypeScript AST로 읽고 string-valued node/attributes만 추출했습니다. React component나 원본 App runtime을 실행하지 않았습니다. key metadata만 제거해 source node9/2/3/6/2개를 SVG로 옮겼습니다. `viewBox=24`, stroke2·round cap/join을 보존하고 white raster를 theme foreground/warning으로 tint합니다.
2. parse한5개 Tree를 소유하고 현재 배율의5개 texture만 유지합니다. 배율이 같으면 raster/texture를 다시 만들지 않고 배율 교체/owner 폐기 시 이전 TextureHandle을 해제합니다. source는 compile-time 내장하며 external/data href resolver는 거절합니다. glyph12px·Keyboard14px·X16px와 reset/unbind24px 버튼·close top/right16px·기본 opacity70%를 연결했습니다. 실제 hover/focus/radius와 모든 small viewport의 CSS 동등성은 아래 gate에 남습니다.
3. egui0.36의 frame 시작 focus navigation 때문에 캡처 key event를 content draw 전에 제거하는 것만으로 Tab/화살표를 막을 수 없었습니다. 캡처 중 focus direction을 취소하고 해당 버튼 EventFilter의 Tab/가로·세로 화살표/Escape를 잠급니다. 포인터 blur를 강제로 되돌리지 않으며 실제 capture button focus가 소유할 때만 binding을 적용합니다.
4. popup이 열린 경우 keybindings가 Escape와 캡처 events를 먼저 소비하지 않습니다. App의 raw focused=false에서는 viewport chord/deferral/decision cache를 초기화합니다. 실제 OS blur·auxiliary 전체 수명 완료를 주장하지 않습니다.
5. 원본 X와 TriangleAlert의 AlertTriangle alias를 설치 source에서 확인해 기존 ISC·Feather MIT notice의 적용 아이콘 설명을 갱신했습니다. 새 dependency/root lock/MSRV/보호 bundle/OS 설정 변경은 없습니다. 최종 package notice 감사는 N7에 남습니다.

## 실제 검사

- [x] app `--lib keybinding_modal_input`:두 실패 RED를 먼저 확인했습니다. popup Escape가 사라졌고 Tab chord save가0건이었습니다. 수정 뒤2 PASS, compile2.19초/suite0.24초입니다. unrelated frame hook 위치에 들어간 blur patch의 compile 오류는 올바른 App::ui 경계로 이동해 제거했습니다.
- [x] 아이콘 연결 영향의 app `--lib keybinding_editor는`:1 PASS, compile2.36초/suite0.56초. 실제 rendered reset icon pointer click과 기존 capture/즉시 save/reset/key-search/Escape/IME/focus 흐름입니다. 최초 검사기의 Mesh-only assumption 때문에 image를 찾지 못했으며 설치 egui Image::paint의 Rect+Brush를 확인해 검사기를 수정했습니다. 이미 성공한 popup/model/host 검사는 반복하지 않았습니다.
- [x] 신규 app `--lib keybinding_icons`:1 PASS, compile3.09초/suite0.02초. 1/1.25/2배율의5 texture 크기·실제 nonzero/transparent pixel·logical point size·동일배율 texture ID와 update 없음·배율 교체의 이전 texture free·owner drop의 전체 free를 확인했습니다. 초기 test의 TexturesDelta를 구버전 list로 가정한 타입 오류는 설치0.36의 HashMap/SmallVec API로 수정했습니다.
- [x] app `cargo clippy ... --lib --bins --tests -- -D warnings`:exit0(11.37초). authored3개 exact fmt·추적diff whitespace 검사에 진단이 없었습니다. Wry17개의 inherited vendor warning은 authored strict와 구분합니다. Cargo는 serial·locked/offline·기존 target을 사용했습니다.

최초 icon 연결 library check exit0(2.03초)는 compile 근거이며 runtime/UI 성공을 중복 계산하지 않습니다. 배율별 raster pixel 검사는 OS/GPU 전체 screenshot 비교가 아닙니다. 사용자 clipboard·입력기·VoiceOver·보호 앱·실제 사용자 문서는 사용하지 않았습니다.

## 남은 동등성

- [ ] 정확한 CSS grid의 label/when/충돌 truncate·header column·small viewport·간격/line-height/button variant·pill/context badge·tooltip/focus ring/hover opacity·shadow/scrim/animation·overlay scrollbar를 연결합니다. 현재 행 폭 예약과 기본 native tooltip만으로 동일 화면 완료를 주장하지 않습니다.
- [ ] 일반 query 모드의 Tab/ShiftTab 순환·DOM 순서에 맞는 close focus·포인터→key 같은 frame·capture row/search 교체·blur·multi-pass exactly-once·실제 App/aux/AX/OS를 확인합니다. 이번 EventFilter 검사는 캡처 key navigation 경계만 덮습니다.
- [ ] 원본 AppToaster의 richColors/close/position/수명에 맞춰 warning/설정 저장 실패를 표시하고 빠른 연속 저장·late event·다른 창 반영을 확인합니다. 현재 App status 경로가 toast 동등성은 아닙니다.
- [ ] 나머지8 공통 action/전체 Monaco·213 view/성능·보안·배포/cutover/TS제거와 상위 M8를 완료합니다. remount A/B는 응답 대기이고 임의로 확정하지 않습니다. 전체 M8 완료 뒤만 commit/push합니다.
