# native 공유 provider의 root별 child 중복 생성

## 대상·관찰

`native/taide-native-app/src/lsp.rs`와 `crates/taide-lsp/src/native/session.rs`입니다. 원본은 sharesSessions spec의 동일 project/server에서 여러 root가 한 연결을 공유하지만 native는 root별 SessionKey마다 child를 생성했습니다. 실제 Python 두 pyproject root의 합성 child 검사에서2개/기대1개 RED(exit101, compile7.68초/suite0.02초)를 확인했습니다.

## 해결

기존 Rust should_reuse_session 계약으로 canonical key를 선택하고 session의 전체 roots를 보존합니다. root 추가는 초기 Running 이후 actor에서 중복·aggregate/wire budget과 outgoing 입장을 확인해 알림과 다음 initialize 목록을 함께 반영합니다. 서버 workspaceFolders 응답/applyEdit scope에도 전체 roots를 전달합니다. nonshare spec과 다른 project, 종료된 session은 합류하지 않습니다.

## 검증·남은 범위

공유 실제 child1 PASS(compile7.24초/suite0.13초), nonshare/project 격리 실제 child1 PASS(9.11초/0.41초), actor 원자적 실패1 PASS(2.99초/0.00초)입니다. root LSP strict1.21초·app strict18.09초입니다. fixture cleanup API 컴파일2회 실패·모델 retain 누락의 marker0·별도 workspace dependency test 거절과 정확한 명령은 `docs/quality-assurance/2026-10-04-m8-native-problems.md`에 구분해 기록했습니다. 실제 언어 도구/full dispatcher/GUI/applyEdit 왕복 검증이 아니며 같은 generation handshake 재시도·전체 provider/consumer/heap/M8은 미완료입니다. 성공 검사는 반복하지 않았고 manifest/lock/MSRV·제품 TS·보호 bundle·사용자 앱·OS·Git은 변경하지 않았습니다.
