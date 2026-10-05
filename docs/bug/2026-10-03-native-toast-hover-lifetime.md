# Native toast hover와 expanded/interacting 수명

## 대상 파일

`native/taide-native-app/src/toast.rs`의 tick·close hover, `toast-motion.rs`의 close color transition입니다.

## 리포트

close hover는 PointerGone 뒤에도 egui Response.hovered가 유지되어 기본색으로 돌아오지 않았습니다. parent는 원본 이벤트 state 대신 매 프레임 hover로 expanded를 켜고 drag outside release에서 접었습니다. 원본 Sonner는 enter/move에서만 펼치고 interacting 중 leave 및 pointer-up 뒤 펼침을 유지합니다.

## 상세와 검증

1. close는 실제 pointer.hover_pos와 변환된 close rect를 추가로 확인합니다. 두 theme의200ms 색 전환·leave 복귀 검사1 FAIL→1 PASS(suite0.02초)입니다.
2. parent에 interacting을 분리하고 raw 이벤트 순서의 enter/move/leave/down/up 및 frame 단일 처리를 연결했습니다. timer는 hidden/expanded/interacting에서 멈춥니다. modal/focus 취소는 native 보호 정책으로 구분합니다.
3. parent의 최초 실패 두 건은 공개 focus 요청 뒤 focus_within이 반영되기 전에 Escape를 보낸 fixture 문제였습니다. 제품의 stationary 재현이라고 잘못 설명한 내용을 정정하며 source 대비 차이와 실패 증거를 혼동하지 않습니다. 실제 focus 반영 assertion 뒤 parent1 PASS(suite0.02초), 변경된 stack/focus/swipe renderer 각1 PASS(suite각0.01초), 최종 strict exit0(12.53초)입니다.
4. OS pointer capture·전체 선택/touch/DragEnd·같은 frame/late/aux/WebView·전체 toast/M8 완료는 아닙니다. 정확한 source·실패·재사용 성공·남은 gate는 `docs/quality-assurance/2026-10-03-m8-native-toast-swipe-hover.md`가 정본입니다.
