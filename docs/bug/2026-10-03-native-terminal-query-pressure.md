# 일시적 writer 포화가 query 실패로 전파됨

## 대상·재현

`native/taide-native-app/src/terminal_dispatch.rs`가 `Writer::submit`의 full 오류를 effects 실패로 반환했습니다. actual Core의 색 query를, 첫 write가 정지되어 count=1이 가득 찬 Writer로 전달하자 Dispatcher가 Pending 대신 즉시 오류를 반환했습니다.

## 수정·결과

Writer의 byte/count admission을 비동기로 기다리는 API에 query 응답을 연결했습니다. 영구적인 oversized payload는 먼저 거절하고 actor receiver 종료도 함께 관찰합니다. 취소·close·root-stop은 부분 permit을 반환하며, 실제 실행 중인 write의 감독은 유지합니다.

신규 query 압력 1건과 변경 writer 4건이 통과했습니다. 정확한 명령·시간·범위는 `docs/quality-assurance/2026-10-03-m8-native-terminal-query-pressure.md`가 정본입니다.

Views에 남은 UI pending과 query가 아직 공통 순서를 공유하지 않는 문제는 미완료입니다. 이 수정은 이미 writer에 도착한 전송의 포화를 대기로 처리한 것이며 전체 FIFO·aggregate/RSS·실제 OS 완료 선언이 아닙니다.
