# Native editor LSP 종료 후 자동 복구 누락

## 대상과 관찰

`native/taide-native-app/src/lsp.rs`의 실제 LspBridge는 SessionClient를 만들었지만 watch 상태를 관찰하거나 restart를 호출하지 않았습니다. native coordinator는 process 종료 후 Degraded로 전이하고 mirror를 유지하지만 외부 호출 없이 새 process를 만들지 않습니다. 원격 production LSP의 별도 restart 포트는 이 actor를 복구하지 않습니다.

합성 `--native-actions` fixture에서 초기 진단 이후 자신이 소유한 child만 kill했을 때 crash recovery 단계의5초 timeout을 확인했습니다. compile10.97초/suite5.38초 RED입니다. 그 이전 두 timeout은 잘못된 fixture의 초기 진단 대기 실패였으며 결함 증거와 구분합니다.

## 수정과 결과

실제 sync_document가 감독된 weak-owner monitor를 연결합니다. 기존 restart3회/backoff/healthy30초 정책과 SessionClient/coordinator의 generation·initialize·최신 replay를 재사용합니다. shutdown·project/root·generation/phase 재검사, stop/channel 종료의 monitor 회수와 notice/status source identity를 추가했습니다.

실제 bridge 신규1 PASS(compile8.38초/suite0.63초)로 crash 대기 중 live edit·generation1/Running·최신 formatting·두 번째 crash backoff 중 마지막 문서 close·worker/process join/task0/owner 해제를 확인했습니다. 별도 timer policy1 PASS(compile10.52초/suite0.00초)입니다. 전체 status UI·원본 same-generation handshake 재시도·provider/성능/OS parity가 완료된 것은 아닙니다. 상세와 미완료는 `docs/quality-assurance/2026-10-04-m8-native-editor-lsp-recovery.md`가 정본입니다.
