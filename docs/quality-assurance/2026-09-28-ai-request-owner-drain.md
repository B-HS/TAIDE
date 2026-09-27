# AI 요청 owner 종료 QA

## 대상 파일과 리포트

`ai_request_store.rs`, `exit_drain.rs`, Tauri root/state 등록과 AI action이 대상입니다. provider·keyring·사용자 자격증명/파일·실제 앱을 사용하지 않고 메모리 채널/token과 기존 자기 소유 child/worker만 검사합니다.

## 확인한 경계

- [x] request future Drop의 key 잔류 실패(exit 101)를 재현하고 수정했습니다. poll 전 future Drop와 poll된 Tokio task의 abort를 await한 뒤 같은 key 재시작을 확인합니다.
- [x] 같은 owner/requestId의 중복과 서로 다른 owner의 독립성, cancel 뒤 늦은 finish/Drop이 새 identity를 지우지 않음을 확인합니다.
- [x] shutdown 뒤 모든 신규 key가 거절되고 취소 receiver는 깨어나지만 살아 있는 token이 있으면 idle 대기는 완료하지 않습니다. 취소/수동 finish로 registry가 비어도 old owner를 기다립니다.
- [x] idle 대기 Drop 후 재대기·동시 waiter와 최종 token Drop의 알림을 확인합니다. sender 폐기/취소 알림은 mutex 밖이고 live 목록 해제는 sender 폐기 뒤입니다.
- [x] 정상 coordinator는 같은 AiRequestStore를 주입받고 AI owner Drop 전에 ready/exit callback을 열지 않으며 drain Drop 후 새 coordinator도 같은 owner를 기다립니다. store 12·coordinator 7건이 최종 exit 0입니다.
- [x] 최종 AI action 9·동일 타입 1·AppServices 2·감독/실제 Tauri 배선 source 22·IPC 7이 exit 0입니다. store/root를 포함해 서로 다른 60건이며 같은 재실행/0건은 중복 합산하지 않습니다. source 확인은 실제 native 이벤트 전달 실기와 구분합니다.
- [x] 최종 runtime all-target·Tauri lib/관련 integration clippy·fmt/diff와 strict runtime rustdoc가 exit 0입니다. 뒤의 내부 Drop 순서/테스트 추가는 문서 API 불변으로 strict 문서 성공을 재사용합니다. 새 history/QA MD만 별도 포맷합니다.

## 남은 gate

- [ ] 실제 native ExitRequested 전달/메뉴·GUI 이벤트 루프와 provider 요청 중 앱 종료 실기를 확인해야 합니다. 메모리 token 대기가 실제 native 이벤트 전달을 증명하지는 않습니다.
- [ ] 직접 Exit·런타임 소멸·OS 강제 종료의 owner Drop/완료는 미보장입니다. 로컬 provider future를 정리했다고 외부 provider의 원격 처리가 중단됐다고 판정하지 않습니다.
- [ ] 나머지 nested blocking·stage 임시 경로·비IPC callback/root와 M6/M7/M8 전체 gate를 완료해야 합니다.
