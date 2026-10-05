# native LSP 재초기화 첫 오류의 조기 종료

## 대상·관찰

`crates/taide-lsp/src/native.rs`, `native/session.rs`, `native/taide-native-app/src/lsp.rs`, `lsp-recovery.rs`입니다. 원본 registry는 재시작된 generation에서 같은 child에 initialize를 최대3회·2초 간격으로 보내지만 native actor는 첫 initialize 오류에서 child를 회수했습니다. 실제 합성 child의 첫 오류 `ServerError(-32002)`로 Running 대기가 실패했습니다(exit101, compile7.68초/suite0.63초).

## 해결

재시작 generation의 실패 handshake만 동일 child에서 재시도합니다. 내부 Degraded backoff는 외부 Initializing으로 표시해 process crash recovery가 별도 child를 재시작하지 않도록 합니다. 각 시도의 기존15초 deadline·요청 ID를 새로 만들고 live 문서와 roots는 유지합니다. 초기 generation과 transport exit는 이 재시도와 구분합니다.

3회 소진을 별도 failure로 표시하며 process recovery는 종료합니다. worker는 현재 channel·generation·phase·failure를 확인해 session과 marker owner를 폐기합니다. 같은 visible sync의 매 프레임 자동 재생성은 하지 않으며 다음 close/open에서 새 owner를 만듭니다. Stop/restart/reap은 예약 timer를 취소합니다.

별도 counter overflow 검사에서 retry 생성 실패가 Degraded를 Detected로 바꾸는 RED를 확인했습니다(exit101, compile0.11초/suite0.00초). begin 실패 시 원래 phase를 복원해 pending/document 상태를 보존합니다.

## 검증·남은 범위

실제 child 성공1 PASS(compile17.71초/suite4.13초), 실제 worker 소진·owner/marker 회수·다음 열기 재연결1 PASS(4.85초/4.15초), coordinator latest/deadline/late 응답/원자적 실패1 PASS(0.91초/0.00초), actor Stop1 PASS(1.25초/0.00초)입니다. root LSP strict1.06초·native app strict18.24초·Rust6 exactfmt/check입니다. 정확한 명령·fixture 오류·0tests·테스트명 warning은 native-problems QA에 기록했습니다. 같은 성공 검사를 반복하지 않았습니다.

실제 언어 도구·NativeApplication GUI·동시 문서 편집/전체 provider/dynamic 등록·누적 heap와 전체 M8 완료를 주장하지 않습니다. manifest/lock/MSRV·제품 TS·보호 bundle·사용자 앱·OS·Git은 변경하지 않았습니다.
