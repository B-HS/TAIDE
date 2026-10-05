# M8 native 프로젝트 hooks capability 연결

## 대상과 결과

대상은 `native/taide-native-app/src/projects.rs`, `projects-tests.rs`입니다. 기존 NativeProjects의 hooks-enabled 무조건 Forbidden을 제거하고 실제 `agent_hooks::reconcile_installed`를 원본처럼 감독된 transient task로 연결했습니다. 기존 HostCommand OpenProject/RestoreWatchers와 remote factory의 NativeProjects::new 호출을 그대로 사용합니다. 전체 capability/NativeApplication 완료는 아닙니다.

## 원본과 구현

- 원본 `src-tauri/src/lib.rs::project_capabilities`는 layout/file/Git watcher/terminal/Git cache/tree/IDE lockfile/agent hooks 순서입니다. 원본 agent capability는 IDE lockfile 이후 guard-free build에서 작업을 spawn하고 기다리지 않으며 root reconcile이 Settings enabled와 installed-file 조건을 판단합니다.
- new는 실제 native agent-hooks callback을 생성합니다. with_hook_reconcile의 필수 typed callback은 테스트/명시 조립용이며 제품 no-op default가 아닙니다.
- NativeProjects의 기존 watcher/layout build와 IDE lockfile refresh 뒤 agent-hooks-attach를 같은 AppServices TaskSupervisor에 등록합니다. callback은 await하지 않으며 build→guard 재취득→실제 live project 확인→commit 순서는 유지합니다. spawn 입장 거절은 원본과 같이 따로 제품 오류를 추가하지 않습니다.
- enabled 자체로 프로젝트를 거절하지 않고 실제 root reconcile의 기존 Settings gate를 사용합니다. duplicate open은 root action의 already_open 조건으로 새 attach/reconcile을 실행하지 않습니다.
- 원본 agent capability의 detach는 기존 no-op이며 새 hook uninstall 또는 LSP/Hub 정리 정책을 이 slice에 임의로 추가하지 않았습니다. 기존 IDE lockfile 오류 처리/flush/다른 capability 차이는 상위 미완료 경계입니다.

## 최소 재현과 검증

- [x] 필수 callback 주입과 최소 enabled open 테스트를 먼저 작성했습니다. 원본 check_hooks를 유지한 exact 실행은 compile17.69초/suite0.01초에서 실제 Forbidden(`native agent hook reconciliation is not connected`) RED입니다. 이 RED 준비 단계의 아직 사용하지 않은 callback field 경고는 수정 후 사라졌으며 suppression은 없습니다.
- [x] 거절 제거/실제 감독 spawn 뒤 `cargo test … --lib projects::tests -- --nocapture`: compile8.05초/suite0.49초, 고유5 PASS입니다. 필터는 직접 변경된 NativeProjects를 사용하는 remote_projects의 기존3도 포함했고, 변경된 코드 상태의 관련 회귀 결과입니다. 동일 성공 상태의 반복 검사로 추가 실행하지 않았습니다.
- [x] 신규 enabled1은 같은 services identity·실제 mutation guard 취득/해제·project snapshot·watcher/layout commit·pending callback 비대기·중복 open 비재실행·미완료 callback shutdown 취소·task1→0입니다. 합성 callback만 사용해 실제 home/hooks/server/provider는 접근하지 않습니다.
- [x] 신규 off1은 production new callback의 기존 root Settings gate·실제 open/restore watcher 중복 방지·server 없음·project close watcher/layout 폐기·감독 종료 후 새 open 거절/task0입니다. settings false이므로 root는 home lookup/서버 bind 전에 반환합니다. 새 합성 project/data만 사용했습니다.
- [x] 변경 뒤 native lib/bin/tests strict `-D warnings`: exit0·13.72초입니다. 기존 Wry dependency17 warnings와 authored strict를 구분하며 suppression은 없습니다.
- [x] authored2 edition2024 exact rustfmt·tracked diff check exit0입니다. 신규2 no-index check 빈 출력/exit1은 정상 신규 diff입니다.

Cargo 환경은 CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo, native manifest, --locked --offline --target-dir experiments/native-shell-spike/target입니다. 이번 slice는 manifest/lock/root/Tauri/제품TS/MSRV/TAIDE Git을 변경하지 않았습니다. 앞선 후속26의 공유 Gist 변경은 보존합니다. 보호 bundle/사용자 앱/프로세스/홈/자격 증명/OS 설정을 조작하지 않았습니다. 실제 enabled production callback의 사용자 설치 hooks 재조정은 실기 검증으로 남깁니다.

## 미완료

- [ ] production IDE server startup·필수 callbacks와 전체 project capability/Hub/flush·App shutdown/Settings reconcile 조립을 기존 pending에서 이어갑니다. 이 callback 연결은 전체 capability 동등성 완료가 아닙니다.
- [ ] 원본처럼 기존 설치 사용자 hooks만 조정하는 실제 enabled production 동작과 모든 provider의 OS/file 경계는 제품 실기 gate에서 확인합니다. 이번 합성 callback이 사용자 홈의 통합을 대신하지 않습니다.
- [ ] 전체 N1~N8 0/8·keybinding Tab RED/PTY remount·전체 view/cutover/Rust99%·IME/AX/perf/security/package/rollback 미완료이며 full M8 완료 전 commit/push하지 않습니다.
