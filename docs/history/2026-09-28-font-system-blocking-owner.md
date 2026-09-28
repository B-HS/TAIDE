# 폰트·시스템 사용량 blocking 조회의 종료 소유

## 대상과 원인

- `src-tauri/src/domain/font/commands.rs::font_list`는 시스템 폰트 첫 스캔을 직접 `tauri::async_runtime::spawn_blocking`으로 실행했습니다.
- `src-tauri/src/domain/system/commands.rs::system_usage_get`·`system_usage_breakdown`은 앱 사용량/프로세스 레코드 수집을 같은 방식으로 실행했습니다. 요청 future가 취소되면 이미 시작한 작업의 JoinHandle이 버려져 정상 root의 TaskSupervisor 대기에서 빠질 수 있었습니다. 실제 폰트/사용자 프로세스 스캔은 실행하지 않았습니다.

## 수정

기존 등록 TaskSupervisor에 결과형 `run_blocking_result`를 추가해 세 호출이 실제 blocking worker의 완료를 추적합니다. worker 반환 뒤 기존 서비스 `AppResult`는 그대로 전달하고 JoinError는 기존 `Internal`로 변환합니다. 닫힌 감독자와 시작 전 취소된 worker는 `Forbidden`으로 거절합니다. 시스템 사용량 breakdown은 사전 PID/label/CPU 확인과 사후 결과 조립에도 operation lease를 보유합니다. Native font/system 명령은 같은 managed State를 받고 원격 gateway도 같은 State를 전달합니다. 서비스의 실제 OS 측정 함수, 공개 인수·응답 wire, 이름·등록 순서는 유지합니다.

## 재현과 검증

메모리 worker의 새 API 부재 E0599 세 건(exit 101)을 확인한 뒤, 요청 abort 후 held worker를 정상 root가 기다리는 검사, 종료 입장 거절, panic/Internal, 원래 서비스 오류, 정상 결과를 추가했습니다. 최종 `cargo test -p taide-runtime --lib task_supervisor::tests` 10건, `cargo test -p taide --test blocking_adapter_ownership` 1건, `cargo test -p taide --test rust_native_phase0_contract` 7건, 실제 TypeScript bindings 생성 1건으로 서로 다른 19건이 통과했습니다. bindings/manifest 작업 트리 diff는 없습니다. 관련 clippy·strict rustdoc·Rust fmt/diff·대상 MD 포맷도 exit 0이며 같은 입력의 성공 결과는 재사용했습니다.

## 남은 경계

이미 시작한 OS 폰트/프로세스 조회를 강제 중단하거나 종료 대기에 상한을 두지 않습니다. 직접 `RunEvent::Exit`는 정상 `ExitDrain`과 다르고, 실제 OS scan·사용자 PID·GUI는 실행하지 않았습니다. 다른 nested worker/서버·M6/M7/M8 전체는 미완료입니다.
