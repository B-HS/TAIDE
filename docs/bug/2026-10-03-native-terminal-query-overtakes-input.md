# 터미널 응답이 기존 지연 입력을 추월함

## 대상·원인

Session의 인코딩된 pending 입력은 Views Outbox가 보유했지만, Dispatcher query는 Writer에 바로 들어갔습니다. 비동기 용량 대기를 추가해도 Writer에 아직 도착하지 않은 이전 입력은 보이지 않았습니다. 실제 child에서 이전 x보다 상태 응답이 먼저 전달되어 exit 1로 실패했습니다.

## 수정·결과

입력과 응답이 Writer별 bounded 순서 표식을 공유합니다. PendingInput은 최초 등록 표식을 유지하고 실제 enqueue 또는 취소 때 반납합니다. focus batch도 중간 query 표식을 건너뛰어 합치지 않으며, 별도 fragment로 같은 순서를 따릅니다. 닫기는 순서 대기도 깨웁니다.

actual PTY x→응답→z·epoch+3·exit 0와 focus 포화, 표식 adjacency/close, writer 종료·취소 검사가 통과했습니다. 정확한 명령·상한·범위는 `docs/quality-assurance/2026-10-03-m8-native-terminal-input-order-queue.md`가 정본입니다.

아직 Session에 들어오지 않은 raw UI event와 output parse 시점까지 전체 순서를 완료했다는 뜻은 아닙니다. 모든 실제 OS/다중 창·aggregate/M8 경계는 유지합니다.
