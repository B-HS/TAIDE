# 원격 미러 복원이 pending 저장·선택을 침범하는 경계

## 대상·증상

`native/taide-remote-web/src/files.rs::restore_mirror`는 초기 clean/revision만 확인했습니다. 같은 문서의 save가 이미 전송됐지만 아직 완료되지 않은 동안 미러를 복원하는 신규 검사에서 Ok(true)/기대 Ok(false) RED를 확인했습니다.

## 해결·검증

동일 문서의 pending SaveSnapshot과 queued/in-flight ChoiceRequest가 있으면 복원을 보류합니다. 이후 저장 완료의 restore_finished나 선택 이후 변경된 revision으로 과거 미러를 적용하지 않습니다. 실패1만 다시 검사해 GREEN(build0.60초/suite0.00초)입니다. 원래 성공한 초기 browser 시나리오는 반복하지 않았고 최종 strict도 통과했습니다. 상세는 `docs/quality-assurance/2026-10-05-m8-rust-remote-mirror-restoration.md`이며 전체 write/epoch/flush·App 완료가 아닙니다.
