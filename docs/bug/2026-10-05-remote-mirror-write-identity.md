# 원격 미러 쓰기 receipt의 동일 값 식별 문제

## 대상 파일

`crates/taide-model/src/ids.rs`, `crates/taide-model/src/file.rs`, `crates/taide-file/src/service.rs`, `crates/taide-runtime/src/file_actions.rs`, `native/taide-native-app/src/remote-files.rs`.

## 리포트

`file_mirror_dirty`는 디스크 baseline만 반환합니다. 이를 쓰기 receipt로 쓰거나 완료 후 목록을 읽으면 다른 writer의 미러를 자신의 것으로 오인할 수 있습니다. full MirrorEntry 비교도 같은 내용·baseline·savedAtMs의 두 쓰기를 구별하지 못합니다. 저장과 미러 쓰기가 겹친 뒤 stale write를 정리하려면 실제 쓰기 식별자가 필요합니다.

## 원인·해결

1. 모델의 기존 UUID ID 패턴으로 서버가 모든 쓰기에 고유 MirrorWriteId를 발급합니다. 기존 JSON은 optional write_id 없이도 읽으며 목록 DTO는 바꾸지 않습니다.
2. 같은 service mutex 안에서 persist한 `{writeId,entry}`를 optional receipt 모드로 반환하고 raw 저장 값+ID를 conditional clear에서 비교합니다. 없어진 미러는 멱등 완료이고 다른 ID/legacy 덮어쓰기에는 false입니다. 저장 이후 원본 metadata 변경은 자신의 정확한 write 정리를 막지 않습니다.
3. runtime/remote 동일 root·canonical·감독 worker 경계, 기존 number/null 응답 및 expected/ordinary clear를 유지합니다. malformed·기대 모드 충돌을 ordinary clear로 바꾸지 않습니다.

## 검증·잔여

actual native receipt 없음 RED→해당1 GREEN, 실제 service 동시간·동내용 fixture1 PASS, 변경된 legacy runtime worker1 PASS입니다. 정확한 명령/시간/최종 static은 `docs/quality-assurance/2026-10-05-m8-remote-mirror-write-receipt.md`가 정본입니다. BrowserEditor의 실제 write/epoch/timer/flush는 다음 구현이고 기존 MirrorEntry 기반 native cleanup을 receipt 기반 전체 owner 완료로 주장하지 않습니다. 여러 프로세스/수동 파일 덮어쓰기는 현재 service mutex 보장 범위가 아닙니다.
