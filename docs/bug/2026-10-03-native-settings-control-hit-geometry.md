# Settings control 정렬·headless hit geometry

## 대상 파일

`native/taide-native-app/src/settings-view.rs`, `settings-view-tests.rs`

## 리포트

원본 NumericField/SwitchField의 우측 control 정렬을 native에 연결할 때 `allocate_ui_with_layout`의 최대 크기가 실제 사용 폭을 예약한다고 가정했습니다. 짧은 label은 사용 폭만 소비해 숫자 입력이 x363~~443으로 줄어든 행 중간에 표시됐습니다. card의32px body 간격을 하위 control의 가로 간격까지 상속한 곳도12px 원본 계약으로 변경했습니다. label child의 min width를 명시해 숫자 입력을 x883~~963의 오른쪽80px에 배치합니다.

headless 검사는 별도로 스크롤 직후 이전 drawing의 좌표와 우측 정렬 galley의 origin을 glyph 위치로 오인했습니다. 숫자 Shape 위치[[439.1,593.4]–[459.1,607.4]]를 클릭했지만 해당 프레임 입력 rect는[[363.1,243.4]–[443.1,261.4]], focus=false였습니다. position 클릭은 성공해도 숫자는 focus를 얻지 못해 blur commit이 없었습니다.

## source 기반 정정

- 고정 egui0.36.2 `containers/scroll_area.rs`는 content UI를 만들고 뒤에서 target offset을 갱신합니다. 시간 점프 한 프레임을 최종 그려진 위치로 가정하지 않습니다. 실제 입력 전 후속 안정화 프레임을 그립니다. animation은 그대로 유지합니다.
- `widgets/text_edit/builder.rs`의 alignment와 galley local rect를 확인했습니다. glyph rectangle은 `galley.rect.translate(text.pos.to_vec2())`이며, shape origin과 `galley.size()`만 조합하면 right alignment에서 rectangle 밖을 클릭합니다.
- language popup의 toggle 이후 focus=None과 처리되지 않은 ArrowDown도 별도 관찰했습니다. raw clickable widget에는 원본 browser trigger의 focus를 명시하고 공개 Memory의 vertical arrow filter를 사용합니다. 열린 popup의 Enter는 trigger의 기본 click 소비보다 먼저 처리해야 language selection이 toggle로 사라지지 않습니다.
- test-only Trace는 실제 widget id/rect/focus/lost-focus/click을 보존합니다. 실제 사용자 화면·OS 설정이나 clipboard는 계측하지 않았습니다.

## 검증과 남은 범위

numeric/position1 PASS(suite0.07초), language1 PASS(0.07초), 영향 theme/TOC/번역1 PASS(0.10초)와 최종 lib/bin/test strict exit0(12.48초)입니다. 각 최종 성공은 재사용하며 모든 실패/명령은 [Settings surface QA](../quality-assurance/2026-10-03-m8-native-settings-surface.md)에 있습니다. 이 결과는 실제 앱의 모든 popup/Tab·OS AX·pixel·성능 검증이나 전체 Settings/M8 완료가 아닙니다.
