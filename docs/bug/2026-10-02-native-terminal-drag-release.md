# Native terminal 자동 스크롤 뒤 release 선택 축소

## 대상·재현

`native/taide-native-app/src/terminal_surface.rs`입니다. 합성 actual Core/headless egui에서 아래쪽 timer가 끝점을 Line1/col12로 확장한 뒤 같은 위치에서 버튼을 놓으면 Line0/col12로 축소됐습니다. 실제 assertion RED입니다. 수정 중 Ui::input 안에서 Response::drag_stopped_by를 호출한 경로는 egui의 10초 RwLock 실패로 재현됐습니다.

## 원인·수정

release마다 mousemove 없이 viewport pointer 끝점을 다시 계산했습니다. 원본 xterm mouseup은 drag timer/listener만 해제합니다. 실제 PointerMoved가 있을 때만 확장하고 mouseup만 있으면 timer가 만든 끝점을 보존합니다. Response::drag_stopped_by의 내부 Context::input 조회는 외부에서 수행해 입력 closure 중첩을 없앴습니다.

## 검증·잔여

최종 정책 1 PASS(0.01초)·변경된 release의 기존 pointer 영향 1 PASS(0.01초)·app strict exit 0(1.88초)입니다. 전체 증거와 headless/OS 구분은 `docs/quality-assurance/2026-10-02-m8-native-terminal-drag-scroll.md`에 있습니다. 실제 OS capture·숨은/다중 창·전체 선택/N4/M8는 미완료입니다.
