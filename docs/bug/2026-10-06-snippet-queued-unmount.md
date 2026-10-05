# Snippets 승인된 저장의 큐 대기 중 unmount 취소

## 대상 파일·원인

`snippet-edit.rs` 초기 Request.execute는 모든 worker 진입에서 Weak editor/active owner를 확인했습니다. native Host queue에 Save를 승인한 직후 Back/탭 닫기가 일어나면 아직 시작하지 않은 저장까지 거절해 원본 nonabortable mutation 계약과 달랐습니다.

## 해결·증거

HostBridge.submit에서 현재 owner/lifetime을 검증하고 mutation admission을 기록합니다. 조회는 계속 현재 owner를 요구하고, UI의 Weak lifetime은 그대로 유지해 늦은 결과를 폐기합니다. admission을 다시 사용할 때도 처음부터 현재 권한을 검증합니다. actual HostBridge의 합성 gate→Save enqueue→Editor drop/owner 제거→늦은 조회 거절→gate 해제→rust.json 실제 저장1 PASS8.65초/.05초와 shared admission1 PASS가 근거입니다.

기존 native 앱 전체 shutdown/drain·OS hot-exit은 별도 미완료이며 승인된 저장의 ordinary unmount 수명만 검증했습니다. browser sent mutation도 정산하지만 actual Chrome 종료는 다음 검사입니다.
