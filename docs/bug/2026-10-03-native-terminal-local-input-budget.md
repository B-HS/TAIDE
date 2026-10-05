# native 터미널 포화 큐의 local action 차단

## 증상·원인

`native/taide-native-app/src/terminal_surface.rs`의 `Outbox::submit`이 인코딩 전에 receipt+pending 상한을 검사했습니다. 따라서 PTY 전송이 필요 없는 SelectAll·PageUp도 큐가 가득 차면 오류로 처리됐습니다.

## 재현·수정

`tests/terminal-host.rs`의 `headless_input_budget`에서 실제 writer count=1 상태로 64개 문자를 넣고 초과 입력·SelectAll·Copy를 같은 headless frame에 전달했습니다. 최초 검사는 `full writer queue blocked local selection`으로 실패했습니다.

전송 슬롯이 없으면 Session에 deferred encoding을 요청하고, local action은 그대로 반환하도록 수정했습니다. 실제 write만 count/byte 상한을 통과한 뒤 pending 큐에 보유하므로 포화 상태의 전송 승인·epoch 갱신은 발생하지 않습니다.

수정 뒤 해당 검사 1건 PASS(0.34초)입니다. 초과 오류 표시·headless CopyText·대기 취소 뒤 epoch+1 유지·실제 child join/task 0까지 같은 검사에서 확인했습니다. 실제 OS clipboard·전체 입력 parity 통과로 해석하지 않습니다.
