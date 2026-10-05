# native presentation의 시작 값 고정과 terminal palette callback

## 대상 파일

`native/taide-native-app/src/{application.rs,presentation-refresh.rs,host.rs,keybinding-editor.rs,terminal_surface.rs}`, `crates/taide-runtime/src/{theme_actions.rs,locale_actions.rs}`

## 리포트

native App은 시작 시 theme/locale로 appearance를 만든 뒤 runtime Settings/Theme 이벤트에서 재조회하지 않았습니다. theme 필드를 갱신해도 기존 terminal의 fallback color callback은 attach-time palette를 읽는 별도 차이가 있었습니다.

## 원인과 수정

1. 원본 provider/toaster/terminal은 현재 query/theme를 반영합니다. native는 event revision과 Settings snapshot으로 한 pending request만 coalesce하고 기존 bounded host/blocking task에서 읽도록 연결했습니다.
2. snapshot resolver를 기존 current resolver와 공유해 IO 중 live Settings를 다시 읽는 혼합을 피합니다. 요청 sequence/revision/inputs가 현재 pending과 맞을 때만 적용합니다. theme/locale은 독립 결과여서 theme 실패가 정상 locale을 막지 않습니다.
3. 모든 appearance가 준비되기 전에 화면 필드를 바꾸지 않습니다. 열린 keybindings appearance만 교체하고 query/capture/focus·문서/terminal/toast identity를 유지합니다.
4. terminal actor의 color callback은 현재 App 소유 palette를 읽도록 연결합니다. geometry/Writer/dispatcher/Core override 경계는 유지합니다. 실제 PTY/hidden-marker/OS 전체 검증은 별도입니다.
5. strict에서 큰 Request로 queue enum이544/912bytes가 된 것을 검출해 boxed typed carrier로 수정했습니다. 검사 억제를 사용하지 않았습니다. renderer fixture의 Rect-only 진단과 기본 system language 가정 오류는 제품 bug 재현으로 계산하지 않습니다.

실제 결과와 미완료 경계는 [presentation-refresh QA](../quality-assurance/2026-10-03-m8-native-presentation-refresh.md)에 기록합니다. 제품 TS·vendor·OS 설정·보호 앱은 변경하지 않았고 M8 전체 완료가 아닙니다.
