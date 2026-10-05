# 원격 Settings 재시도 후 과거 오류의 종료 재실패

## 대상·재현

`native/taide-remote-web/src/{preferences,browser-workbench,browser-editor,browser-application}.rs`입니다. 실제 원본 Settings 스위치→보류 쓰기→Close Pending→거절→Failed/Toast→취소→명시 재시도→성공 ack 뒤에도 Ready가 아니라 과거 `Synthetic preference refusal`로 다시 Failed였습니다. `settings-preferences` 첫 연속 검사의 10초 대기가 이를 재현했습니다.

## 원인·해결

종료 판정이 진단용 Finished 전체 이력에서 최초 오류를 매번 찾았습니다. 선행 fixture가 Finished를 직접 drain해서 숨겨졌지만 실제 Settings consumer는 보존된 결과를 사용합니다. PreferenceWrites에 matched 실패의 일회성 close 채널을 추가했습니다. BrowserApplication은 mandatory consumer 전후에서 새 오류를 소비하고 Pending이면 Failed로 전이합니다. 이미 표시한 오류는 새 close 시도에 다시 적용하지 않습니다. Finished와 Toast 오류는 별도 소비로 보존되며 consumer의 결과 drain이 close 실패를 숨기지 않습니다. 실패를 성공으로 바꾸거나 자동 재전송하지 않습니다.

## 결과

신규 순수 오류/Closed/중복 응답/진단 보존 검사를 새 close 채널에 맞춰 확장하여1 PASS(1.15초/.00초)입니다. 수정 후 실패한 실제 Chrome/Wasm 연속 검사만1회 재실행하여 GREEN(seq12/update2·실패 socket1→cancel/명시 retry→ack 뒤 Ready/socket0·quiet1.1초 불변)입니다. `docs/quality-assurance/2026-10-05-m8-settings-preference-consumer.md`가 최신 근거입니다.
