# M8 원격 project/group/session 실제 backend

## 대상과 구현

대상은 `native/taide-native-app/src/remote-projects.rs`, `remote-projects-tests.rs`, `lib.rs`입니다. 프로젝트8·그룹9·open-in-slot1·slot-close1·session4의 원본23명령을 실제 runtime에 연결했습니다. layout19를 포함한 domain adapter는88/177이며 남은89와 생산용 전체 assembly는 미완료입니다. N1~N8은0/8·목표 active입니다.

- `extend_backend(create_lifecycle, remaining)`은 기존 `ProjectLifecyclePort` factory를 필수로 받습니다. 별도 수기 callback 타입이나 제품 no-op/default lifecycle을 만들지 않습니다. JSON/channel/raw의 remaining 전달과 바깥 `with_policy`를 유지합니다.
- project open/open-in-slot/group-open의 원본 `run_nonabortable_result`를 유지합니다. typed argument를 검사한 뒤 admission을 받고 worker 내부에서 lifecycle을 생성합니다. open/slot의 ProjectOpen perf span도 원본과 같습니다.
- close는 원본대로 abortable이며 flush→mutation→persist/state 제거→detach→Closed/Activated/ShellSlots/List 이벤트를 유지합니다. slot-close는 project-close가 아니며 마지막 slot 거절과 열린 프로젝트/그룹 멤버 보존은 기존 runtime 정책입니다.
- 그룹 순서·이름·색상·접힘·멤버와 project display·프로젝트 순서/활성·shell focus/size/window chrome는 실제 persistence/runtime을 사용합니다. 새 dependency/package/lock/MSRV·root/Tauri/제품TS·보호 bundle·OS 설정 변경은 없습니다.

## 검증

Cargo 환경은 `CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo`, manifest `native/taide-native-app/Cargo.toml`, `--locked --offline`, target `experiments/native-shell-spike/target`입니다. 모든 프로젝트/data는 새 고유 임시 디렉터리이며 hooks/IDE integration을 fixture에서 명시 비활성화했습니다. 사용자 home/키체인/클립보드/앱/OS 설정을 사용하지 않았습니다.

- [x] 최초 컴파일에서 fixture의 Settings 필드 `ide_enabled`가 없어 실패했습니다. 실제 `ide_integration_enabled`로 정정했습니다. 이때 실행된 검사나 통과 결과는 없었습니다.
- [x] `cargo test --lib remote_projects::tests:: -- --nocapture`: compile7.66초/suite0.46초, 원본 open3진입점의 취소/감독 종료 및 close flush 취소1 PASS·fixture 입력/기대값2 FAIL입니다. owner 보호는 존재하는 필드만 교체하고 색상 wire 값은 `graph.lane1`이 아닌 `lane1`입니다. 원본 코드/정책을 바꾸지 않고 fixture만 정정했습니다.
- [x] 실패2개만 `--skip 열기3진입점의_취소와_감독종료는_승인된_작업을_남기고_닫기_flush는_취소된다`로 재실행했습니다. compile3.18초/suite0.26초,2 PASS입니다. 고유3검사 모두 PASS이며 성공한 취소 검사는 반복하지 않았습니다.
- [x] catalog와 actual23 arm·다른 domain 비중복·default-deny/잘못된 인자·remaining channel/raw/owner 전달·stopped supervisor의3입장 거절/factory 비실행/tracked0을 확인했습니다.
- [x] actual NativeProjects를 test-only 기록/gate wrapper로 감싸 실제 layout/watcher attach·project/session 파일·display mirror·그룹 reopen/skip·slot size/focus/close와 chrome·detach/flush 순서·원본 open/close 이벤트 배열을 확인했습니다. watcher 종료 tracker와 supervisor를 실제로 기다린 뒤 합성 파일을 정리했습니다.
- [x] 3개의 독립 open 경로마다 실제 native attach 뒤 caller를 abort하고 supervisor stop/shutdown이 worker 완료까지 기다림을 확인했습니다. release 뒤 ProjectOpened와 실제 project/watcher가 유지되고 tracked0입니다. 여기서 검사한 것은 TaskSupervisor 종료이며 실제 앱 Exit 전체/프로젝트 복원 전체를 완료로 주장하지 않습니다. close flush 중 caller abort는 project/watcher를 남기고 detach하지 않으며 다음 실제 close가 정리합니다.
- [x] 후속15와 공유 native lib/bin/tests strict clippy exit0,12.93초·authored5 exactfmt exit0입니다. dependency Wry17경고는 별도이며 authored 검사 억제는 없습니다.

## 미완료

- [ ] 생산용 lifecycle factory는 hooks/LSP/native Hub/문서/draft/전체 capability와 원본 hot-exit flush를 모두 연결해야 합니다. 현재 NativeProjects의 flush는 layout 중심이고 UI open의 `check_hooks`는 trait 직접 호출에 자동 적용되지 않습니다. fixture는 hooks 비활성 상태만 검증했으며 이 adapter를 incomplete NativeProjects factory로 제품 조립하지 않았습니다.
- [ ] 남은89명령·전체 dispatcher/Settings/assets/App startup/Exit·remote socket 전체 기능/장기 channel·GUI/AX/성능/메모리/보안·TS 제거/Rust99%·배포/rollback·keybinding RED/PTY remount 결정은 미완료입니다. 전체 M8 완료 뒤만 commit/push합니다.

## 근거

원본 `src-tauri/src/remote_gateway.rs` project/session23 arm, project commands의 lifecycle/nonabortable wrapper, root `project_actions`/service·model·native `projects.rs`를 대조했습니다. 동기화 API는 Tokio Notify/JoinHandle 공식 문서와 현재1.53.1 기존 supervisor 구현을 확인했습니다. 정확한1.53.1 웹 문서는 접근 실패해 latest1.53.2 문서와 설치된1.53.1 코드로 대조했고 버전 변경은 없습니다. serde_json1.0.151 from_value 문서도 확인했습니다.
