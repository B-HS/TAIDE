# M8 공용 Tooltip provider와 실제 browser 소비

## 대상·보존

`native/taide-native-ui/src/{tooltips,tooltip-placement,tooltip-motion,css-motion}.rs`에 원본 provider·배치·이동·easing을 동일 이동했습니다. native App의 tooltips/css-motion 경로는 production에서 re-export하며 옛 placement/motion 사본은 삭제했습니다. 공유 provider의 이벤트 순서·raw event identity·400ms 지연/300ms skip·viewport/context 소유자·AX/focus·controlled 오류·modal/dismissal·scroll·layer 회수는 같은 코드입니다. raw_event_index wrapper 대신 동일 vendored Context 메서드를 직접 호출합니다.

native private integration 검사는 cfg(test) compatibility 모듈에서 공용 소스 하나를 include하여 기존 preview/terminal/explorer fixture와 private 관찰을 유지합니다. production에 별도 구현이나 private 검사 API를 노출하지 않습니다. include 경로/주변 scope는 공식 Rust include 문서와 실제 native 컴파일로 확인했습니다. shared crate 단독 build도 같은 기존 egui-input patch를 사용하도록 manifest/lock의 egui registry edge를 path edge로 바꿨고 버전0.36.2는 유지했습니다.

BrowserEditor는 같은 provider를 소유합니다. begin_ui_frame/finish_ui_frame으로 원본 frame 경계를 호출하고 실제 show_settings가 Output의 Trigger를 한 번 소비합니다. inspection feature의 읽기 전용 open/geometry는 probe/dev에만 켭니다. 다른 surface가 같은 provider를 사용할 API도 제공하되 전체 caller 이식을 완료로 세지 않습니다.

후속 정리: inspection_open은 inspection feature에서만 컴파일합니다. native App의 같은 소스 include도 feature를 명확하게 인식하도록 nondefault inspection forwarding을 추가했고 production 기본값은 꺼져 있습니다. native의 공용 ThemeEditor/tokens/preview/easing 테스트 shim root는 실제 사용되는 cfg(test)에 한정하여 미사용 경고를 제거했습니다. Toast 통합 뒤 lib/bins/tests 최종 strict5.37초 exit0이며 기존 Wry17 외 새 경고는 없습니다. 앞선 runtime 성공을 반복하지 않았습니다.

## 검증

- [x] native 실제 동일 소스·기존 timing1 PASS: build27.18초/suite.02초
- [x] shared portable crate 단독 timing1 PASS: build8.13초/.01초. 같은 행동의 재계측 목적이 아니라 registry→원본 patch compile graph와 native-host 없는 독립 crate 계약 확인입니다. 이후 성공은 재사용합니다.
- [x] 기존 native theme Tooltip/Picker/ANSI16 영향1 PASS(.15초), 같은 binary의 다른 검사이며 timing 성공은 반복하지 않았습니다.
- [x] 실제 Chrome/Wasm `settings-tooltip` 연속1 첫 PASS: Create 화면→원본 ColorPicker pointer hover→같은 provider open→Escape→unmount→Ready/socket0·quiet1.1초 추가 요청/상태0. seq11/쓰기0·frame48/pump50·page/panic/오류/consumer 누출0입니다.
- [x] 최신 probe canvas build5.04초/공식 wasm-bindgen 생성·TS strict/Prettier·Rust exact format exit0. native production lib/bins strict9.62초 exit0이며 기존 Wry17/unused theme test-shim 경고를 검사 억제로 숨기지 않았습니다.

실측 결과는 `/private/tmp/taide-m8-wasm-tools.h2xBQB/file-built/settings-tooltip-result.json`과 `settings-tooltip.png`입니다. 실제 화면에서 색상 버튼 위 `themeEditor.pickColor` Tooltip의 배경/테두리/문자와 provider 표시를 확인했습니다. 합성 locale fallback이고 전체 translation/pixel parity나 OS VoiceOver 검사라고 주장하지 않습니다. 선행 theme-operations 결과는 당시 source의 독립 성공이며 이번에는 반복하지 않았습니다.

## 남은 범위

Toast·live preview의 전체 application 적용·다른 Settings 명령·전체 surface/tab caller·원본 layout 문제·제품 bundle/최종 gate는 남습니다. provider2/4(50%)·전체363/433(83.83%)·최종0/8·ETA 보류·goal active·main 직접입니다. 보호 앱/OS/사용자 데이터/제품TS/기존 의존성 버전/MSRV/Git는 변경하지 않았습니다.
