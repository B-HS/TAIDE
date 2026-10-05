# native Problems 분할선 두께·입력 순서

## 대상·관찰

`native/taide-native-ui/src/shell.rs`, `tests/workbench.rs`, `native/taide-native-app/src/presentation.rs`입니다. 원본 editor-area는 resizerThickness를 PaneSeparator에 전달합니다. 기존 native egui Panel::bottom은 설정5px에도 editor/panel 간격1px이었으며 실제 headless 검사로 재현했습니다.

실제 pointer 드래그에서도 response interact pointer NaN/Flags(1), 최대·드래그 높이503.3125px 그대로를 관찰했습니다. 분할선 interaction이 자식 panel/pane UI보다 먼저 등록돼 후속 UI에 가로채였으며 렌더 후 상태와 hit rect가 한 프레임에서 엇갈렸습니다.

## 해결·증거

설정 두께를 실제 두 영역에서 빼고 해당 간격을 app.border/hover app.focusBorder로 그립니다. 자식 panel/pane을 먼저 렌더한 뒤 divider interaction을 등록하며 실제 상태 변경 때만 repaint를 요청합니다. 다음 프레임은 변경된 상대 비율로 영역과 hit rect를 함께 갱신합니다. 입력이 없는 추가 프레임은 이벤트 재발행이나 추가 계측이 아닙니다.

원본 라이브러리의 ArrowUp/Down5%·Home/End와 기본220px/최소120px/상단30%·상대 크기 유지·재열기 초기화를 연결했습니다. separator는 drag focusable, horizontal AX orientation·value/min/max이며 화살표가 egui focus 이동으로 유실되지 않도록 pinned Memory focus lock을 사용합니다.

최종 단일 workbench 검사1 PASS(compile1.36초/suite0.07초)와 UI lib/workbench strict exit0(8.32초)입니다. 앞선 keyboard fixture의35.95/35.9375px 차이는 공식 GUI_ROUNDING=1/32 단위로 정정했습니다. 자의적 허용 범위 확대나 제품 목표 면제가 아닙니다.

아래 후속 검사로 controls/F6·창 크기 변경·source 기반 낮은 창 제약을 확인했습니다. 전체 pane separator·GUI/VoiceOver는 미완료이며 native-problems QA에서 추적합니다. 사용자 앱·보호 bundle·OS는 조작하지 않았습니다.

## controls/F6·창 크기 변경 후속

후속 실제 AccessKit 검사에서 controls가0개로 관찰됐습니다(compile0.95초/suite0.04초). 실제 편집기 자식 UI를 Group으로 렌더하고 그 실제 node를 separator controls에 연결했습니다. Group의 bounds 누락은 공식 egui의 non-focusable Ui 경계 확인 뒤 실제 editor rect로 수정했습니다. 가상의 참조 node나 다른 슬롯의 ID를 쓰지 않습니다.

원본 F6 순환 범위는 같은 분할 Group이며 Problems Group에는 선이1개입니다. F6·Shift+F6를 소비하고 현재 포커스를 유지하며 다른 슬롯/pane 선으로 이동시키지 않습니다. 창 높이800→1100의 실제 editor/panel 치수로 상대 비율 유지도 확인했습니다.

신규1 PASS(compile1.30초/suite0.04초), 실제 editor UI 구조 변경의 영향을 받는 기존 pointer/keyboard/close/reopen1 PASS(compile0.10초/suite0.05초), 당시 UI strict exit0(0.92초)입니다.

## 낮은 창의 상충 최소 제약 후속

원본 react-resizable-panels4.12.2의 실제 정규화/크기 변경 함수와 flexBasis0/flexGrow를 대조했습니다. 최소 합이100%를 넘으면 두 논리 weight를 유지하며 실제 픽셀은 그 비율로 배분합니다. 유효99px에서 native69.3125px/기대76.15385px RED를 재현했습니다(compile0.79초/suite0.04초).

원본3자리 percentage·max100%·flex 배분을 연결하고 상충 구간의 geometry minimum/maximum을 동일하게 고정했습니다. 정상 높이 복귀는 기존 상대 비율을 정상 제약에 재적용하며 AX는 원본 논리 editor weight30을 유지합니다. 실제 낮은 창 두 높이/AX/keyboard 고정·정상 복귀와 기존 정상 pointer/resize/F6/닫기 영향을 합친3건 PASS(compile1.43초/suite0.07초)·UI lib/workbench strict exit0(0.87초)입니다. 상세 공식 근거와 fixture 정정은 native-problems QA에 기록했으며 원본 DOM 실측·전체 pane separator·GUI/VoiceOver는 남습니다.
