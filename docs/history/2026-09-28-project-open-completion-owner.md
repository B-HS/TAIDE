# 프로젝트 열기 완료 소유

## 대상 파일

- `src-tauri/src/domain/project/commands.rs`, `src-tauri/src/lib.rs`, `src-tauri/src/remote_gateway.rs`
- `src-tauri/tests/project_lifecycle_actions_runtime.rs`

## 리포트

Native `project_open`·`project_open_in_slot`·`project_group_open`을 TaskSupervisor의 취소되지 않는 완료 operation으로 감쌌습니다. 기존 runtime project action은 단일 정책 출처로 유지하고, menu와 원격 gateway에도 같은 supervisor State를 전달합니다. 상태 저장 뒤 capability attach 중 요청 future가 중단돼도 attach 결과에 따른 이벤트 또는 실패 rollback이 끝나며 정상 root 종료가 이를 기다립니다.

## 상세

1. 세 command는 AppState clone·AppHandle·입력을 operation에 소유시켜 요청 수명과 분리합니다. `project_open`과 `project_open_in_slot`의 perf span도 실제 작업 종료까지 유지합니다.
2. group open의 기존 멤버별 순서·오류·활성화 이월 정책을 바꾸지 않았습니다. 요청 취소 뒤에도 이미 시작된 그룹 열기는 나머지 멤버를 계속 처리합니다.
3. 기존 `project_close`의 guard 밖 flush·중복 close 재검사, 공개 IPC 인수/응답, 생성 binding은 변경하지 않았습니다.

## 검증과 잔여

배선 부재 source fixture의 RED(exit 101) 뒤 프로젝트 runtime 통합 15건, Native source 11건, binding 생성 1건이 통과했습니다. 자기 UUID 프로젝트와 대기 capability port에서 요청을 취소하고 root 종료 대기를 확인한 뒤 성공 이벤트와 실패 rollback을 각각 확인했습니다. Clippy·Rust fmt·diff도 통과했습니다. 자세한 명령과 미검증 항목은 [QA](../quality-assurance/2026-09-28-project-open-completion-owner.md)에 기록합니다.

실제 Tauri AppHandle·watcher·메뉴/UI 요청 중단과 OS 종료는 실행하지 않았습니다. 테스트는 runtime action과 같은 supervisor operation의 합성을 검증하고 Native 배선은 source/컴파일 검사로 확인했습니다. 전체 M6·M7·M8 gate와 push는 별도입니다.
