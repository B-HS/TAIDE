# 정상 앱 종료의 감독 작업·설치 lease 드레인

## 대상 파일

- `crates/taide-lsp/src/install.rs`
- `crates/taide-runtime/src/task_supervisor.rs`, `exit_drain.rs`, `lib.rs`, toolchain 설치 fixture
- `src-tauri/src/lib.rs`, `src-tauri/tests/task_supervisor.rs`, 설치 store extraction 검사
- `docs/architecture.md`, `docs/PROCESS.md`, 연결된 종료 QA/toolchain QA

## 리포트

기존 TaskSupervisor.stop_all은 async 핸들을 즉시 목록에서 없애 아직 종료하지 않은 task를 0개로 보고했습니다. current-thread 최소 재현은 실제 취소 전 `0 != 1`로 실패(exit 101)했습니다. 실제 Tokio 완료 플래그를 확인한 뒤에만 추적에서 제거하고 정상 종료가 감독 task·설치 자원의 마지막 lease를 기다리도록 구현했습니다.

## 상세

1. TaskSupervisor는 등록·집계·종료 작업에서 AbortHandle::is_finished가 true인 항목만 정리합니다. 기존 TaskCleanup의 Drop을 실제 task 완료로 간주하지 않으며, abort 직후 async/이미 시작한 blocking 작업의 핸들을 잃지 않습니다. 완료 핸들 정리는 다음 감독자 API 호출 때 이뤄집니다.
2. shutdown은 입장을 닫고 취소를 요청한 뒤 snapshot 핸들의 실제 완료를 10ms 간격으로 관찰합니다. 대기 future가 Drop돼도 registry는 실제 완료 전 핸들을 보유해 다음 shutdown이 다시 기다립니다. 시작한 blocking 작업을 강제 abort하거나 전체 종료 시간을 보장하지 않습니다.
3. LspInstallStore.wait_for_idle은 Notify를 먼저 만들고 상태를 확인해 마지막 lease Drop/복수 대기자·빈 저장소 경계를 놓치지 않습니다. 모든 request/resource lease가 종료해야 반환하며 사용 전 shutdown으로 신규 입장을 닫습니다.
4. Tauri 메뉴 build는 run_main_thread→rx.recv로 native 이벤트 루프의 응답을 기다릴 수 있습니다. root Exit callback에서 전체 감독 작업을 동기 대기하는 방식은 교착 위험이 있어 사용하지 않습니다. 정상 ExitRequested는 prevent_exit 후 owned ExitDrain task에서 기다려 이벤트 루프를 유지합니다. 완료 뒤 원래 exit code로 exit를 다시 요청하며 재진입은 중복 drain/callback을 만들지 않습니다.
5. ExitDrain은 root callback이 JoinHandle을 보유하며 자신이 종료하는 TaskSupervisor에는 등록하지 않습니다. Drop은 coordinator를 취소하며 완료 전 exit callback을 실행하지 않습니다. 직접 Exit가 오면 기존 감독 취소 뒤 설치 lease만 동기 대기합니다. 이 fallback을 모든 다른 자원의 종료로 보고하지 않습니다.

## 검증과 공식 근거

자기 생성 main-reply 채널/worker·설치 lease와 child/pipe/localhost fixture만 사용했습니다. TERM을 무시하는 자기 process-group 자손도 store/supervisor 종료 뒤 PID가 사라지는지 확인했습니다. 검사와 실제 결과는 [종료 QA](../quality-assurance/2026-09-27-exit-drain-lifecycle.md)에 기록합니다.

설치된 Tauri 2.11.5 `app.rs`의 App::run·Exit callback→cleanup_before_exit, ExitRequestApi::prevent_exit, AppHandle::exit를 대조했습니다. Tauri `menu/mod.rs`·`lib.rs`의 main-thread macro와 실제 refresh_app_menu 본문에서 rx.recv 경계를 확인했습니다. Tokio 1.53.1 AbortHandle 문서/harness에서 abort 요청과 actual complete 플래그, future Drop 뒤 완료 기록을 대조했습니다. Tauri의 기본 runtime은 TokioRuntime::new이며 native callback을 막지 않는 coordinator를 소비합니다. 외부 앱 실행은 없습니다.

## 미완료 경계

정상 종료 coordinator와 설치 lease 대기는 전체 M6 완료가 아닙니다. 실제 native 이벤트 전달/메뉴 실기·직접 Exit의 비설치 작업·감독되지 않은 nested blocking worker·LSP wait worker/PTY·부모가 먼저 종료한 뒤 남은 자손·Windows process tree·OS 강제 종료는 남아 있습니다. AppHandle::exit의 runtime 요청 자체가 실패하면 Tauri가 callback을 우회해 std::process::exit하는 경로도 이번 fixture로 보장하지 않습니다. M7/M8·Phase 0 실기는 그대로 미완료입니다.
