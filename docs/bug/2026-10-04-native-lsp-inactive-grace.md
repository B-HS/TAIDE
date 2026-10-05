# native 비활성 모델 진단과 세션 유예 수명

## 대상·관찰

`native/taide-native-app/src/{application,lsp,lsp-idle,lsp-diagnostics-tests}.rs`와 `tests/lsp-recovery.rs`, mock server입니다. 원본은 모델 marker를 session 폐기까지 보존하고 마지막 LSP 구독 뒤5000ms 유예를 둡니다. 이전 native는 didClose에 binding/marker를 제거하고 마지막 mirror가 사라지면 session도 즉시 stop했습니다. 실제 child 검사에 unbind 후 기존 모델의 marker owner 기대를 추가해 false/기대true RED(compile6.73초/suite0.32초)를 확인했습니다.

## 해결

전체 live 모델 catalogue·active mirror·session marker 모델을 분리했습니다. App은 실제 모델 차이만 큐에 입장시키고 실패에는 cache를 보존합니다. 알림은 never-bound/inactive 모델에도 전달하며 모델 폐기/owner 폐기에만 marker 수명을 정리합니다. 마지막 close는5초 deadline, acquire는 취소이며 project stop/Exit는 유예 없이 폐기합니다. 기존 worker select loop에서 deadline을 처리해 별도 timer task를 누적하지 않습니다.

이 구현 중 catalogue가 revision1인데 아직 server mirror0인 알림을1로 보내는 별도 오류를 실제 child에서 재현했습니다. 기대0/실제1 RED(6.84초/0.51초) 뒤 활성 문서는 mirror snapshot revision, inactive 모델은 catalogue revision을 사용하도록 수정했습니다. server protocol version gate·App의 기존 stale revision gate는 유지합니다.

## 검증·남은 범위

큐/child/Idle 정책/recovery의 고유5 PASS입니다. 변경된 revision 입력은 해당1건만 다시 확인해5.12초 suite에서 actual child/같은 owner·PID 재사용/비활성 marker/5초 만료/child·task0을 확인했습니다. 최종 strict17.19초이며 초기 `?` 정리 지적과 전체 명령은 `docs/quality-assurance/2026-10-04-m8-native-problems.md`가 정본입니다. 다중 provider/retarget·full root/handshake·전체 consumer·누적 heap·실제 App GUI/OS/M8은 미완료입니다. manifest/lock·제품 TS·보호 bundle·사용자 앱·OS·Git은 변경하지 않았습니다.
