# 원격 dirty 전송 전 Closed 오류 미공개

## 대상·증상

`native/taide-remote-web/src/dirty.rs`, `browser-editor.rs`, 신규 `tests/dirty.rs`의 unsent dirty 종료입니다. 관찰 가능한 failure가0이고 기대1인 RED를 실행했습니다. unsent desired는 전송이 멈추지만 take_dirty_failures에서 Closed가 나오지 않아 제품 UI가 실패를 알 수 없었습니다.

## 원인·해결

disconnected는 in-flight 요청만 failed를 통해 공개하고 unsent entries는 failure 필드만 바꿨습니다. unsent next_updates도 같은 Closed failed 경로로 통과시키고 이미 실패한 항목은 재공개하지 않습니다. 실제 browser poll은 renderer 사건을 socket 사건 전에 처리하고 같은 poll의 late 파일 사건도 disconnect 뒤 freeze합니다. dispose도 pending renderer 사건을 먼저 처리합니다. 새 공개 dirty_flush_state는 실제 전송 ack 경계를 읽습니다.

## 검증

새 exact 검사 RED(build0.98초/suite0.00초)→수정→GREEN(build0.49초/suite0.00초), 신규 실제 Chrome-Wasm dirty-close1 첫 실행 PASS입니다. 재연결 recovery1에서도 dirty/save 자동 서버 수신0·Closed1·draft 유지·명시 retry 뒤 현재 true1회/dirtyFlushed=true·최종 socket0/quiet1.1초/wake18/응답누출0입니다. 이전 dirty-owner/다른 성공 시나리오는 반복하지 않았습니다. source/artifact·seq 간격을 포함한 상세는 `docs/quality-assurance/2026-10-05-m8-rust-remote-dirty-owner.md` 후속 절입니다.
