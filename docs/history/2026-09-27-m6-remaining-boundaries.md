# M6 잔여 경계 조사

상태: 전체 종료 조사 진행 중. M6 완료를 선언하지 않습니다.

## 기준과 확인 결과

`docs/roadmap-rust-native.md` Phase 2의 완료 조건은 core가 AppHandle 없이 compile되고 기존 Tauri 앱이 port 구현을 소비하는 것입니다. flush, capability attach/detach, watcher·PTY·LSP·remote lifecycle의 명시적 소유권도 요구합니다. AppServices·EventSink·WindowRegistry·TaskSupervisor의 도입만으로 잔여 application action과 자원 소유권을 생략하지 않습니다. M7의 전체·GUI 회귀 gate와 M8의 native UI는 현재 실행 목표가 아닙니다.

현재 `crates`의 Rust/Cargo 원문 검색에서 AppHandle·Tauri 참조는 문서 6곳뿐이며 실제 import·dependency는 없습니다. AppServices는 21개 상태·포트를 조립하고 setup은 기존 Tauri State로 20개를 등록합니다. file의 15개 action은 runtime에 있으며 창 flush 확인과 raw 응답 wrapper는 Tauri adapter에 남았습니다.

## 확인된 잔여 경계

| 경계 | 실제 파일·심볼 | 남은 확인·이전 |
| --- | --- | --- |
| 설정 application action | `src-tauri/src/domain/settings/commands.rs`: settings_get·settings_update·settings_set_theme·apply_and_broadcast | sanitize→persist→live state 적용→observer 순차 await→SettingsChanged, 추가 ThemeChanged 순서와 호출자의 mutation guard가 아직 Tauri 파일에 있습니다. 설정 정책 조립을 UI 비의존 runtime action으로 옮기고 실제 observer의 AppHandle만 adapter에 유지합니다. |
| 프로젝트·layout·검색 및 다른 action | project/commands·project/capability, layout/commands·layout/service, search/commands, app/commands, plugin_port | 창·OS·등록 adapter와 UI 비의존 상태·권한·영속·이벤트 조립을 나눠야 합니다. 실제 창·capability 구현을 무조건 core로 이동하지 않으며 203 command 전수 대응과 나머지 port 종류를 계속 대조합니다. |
| LSP 설치 수집·child 수명 | `src-tauri/src/domain/lsp/commands.rs`: capture_output_tail·run_toolchain_install | stdout/stderr 수집은 직접 tokio::spawn하고 핸들을 보유하지 않습니다. 설치 Child는 명시 취소에서만 process group 정리·wait하며 setup 종료의 LspStore.kill_all에는 설치 child가 포함되지 않습니다. 종료·수집 완료·등록 거절의 실제 회귀를 먼저 작성하고 감독/자원 소유권을 해결해야 합니다. |
| infra LSP 작업 | `crates/taide-infra/src/lsp_proc.rs`: spawn·LspProcHandle | stdout·stderr·child wait 작업이 직접 spawn됩니다. stderr 핸들은 child wait 작업이 드레인하지만 timeout 뒤 reader 취소와 stdout/wait 핸들의 소유권은 별도 검토 대상입니다. 기존 동기 kill·exit guard·stderr masking·drain timeout을 유지해야 합니다. |
| PTY와 요청형 pipe reader | `crates/taide-infra/src/pty.rs`: spawn·PtySession::drop, `crates/taide-git/src/service.rs`: read_pipe_on_thread | PTY reader/flusher/wait thread 핸들은 현재 보관하지 않지만 Drop은 pause 해제→child kill→temp 정리를 소유합니다. 명시적인 세션 작업 소유권과 종료를 검증해야 합니다. Git의 요청형 pipe reader는 bounded 수집·취소 불가능한 표준 Read의 기존 계약과 구분해 판정하며 이름만 보고 장수 작업으로 일괄 변환하지 않습니다. |

remote/commands·remote/ws·sync/github와 IDE server에 검색된 일부 직접 spawn은 cfg(test) 안의 fixture입니다. IDE 연결의 실제 child 작업은 JoinSet, 서버/연결과 remote accept/WS는 이미 TaskSupervisor 경계를 소비합니다. 소스 검색 결과만으로 모든 spawn을 미감독 제품 작업이라고 분류하지 않습니다.

## 다음 변경 단위

설정 action 3개와 공통 apply 경계를 먼저 이전합니다. 기존 app_file_write·apply_settings_file·sync_download는 같은 SettingsApplyPort를 통해 공통 apply adapter를 소비하므로 공개 경로와 재진입 금지 계약을 보존합니다. 실패 테스트와 관련 설정/공유 상태/이벤트·IPC 검증을 동반하고 소유권 조사에서 확인한 나머지 항목은 계속 미완료로 유지합니다.

실제 앱·설치기·LSP/PTY 외부 프로세스는 실행하지 않았습니다. 이 조사는 재현 전 종료 위험을 확인한 것이며 새로운 버그를 검증 완료로 기록하지 않습니다. GitHub B-HS/TAIDE의 to_rust_native 일반 push는 사용자 승인 범위대로 M6 전체 완료 후 수행합니다.

## 후속 파일 대조

현재 활성 목표는 남아 있는 M 전체 완료로 재개됐습니다. 앞서 적은 M7/M8 제외는 당시 목표의 기록이며 현재 상태 정본은 PROCESS입니다. 설정 action은 911d320, 검색 action은 cc2a10f로 분리·검증됐습니다. 각 실제 검증 근거는 별도 action 이력에 기록했습니다. command owner 전수 집계는 별도 census 문서에 기록했으며 전체 body 적합성 판정은 미완료입니다.

LSP 설치의 실제 setup 종료를 추가로 대조했습니다. lib.rs의 ExitRequested/Exit는 LspStore.kill_all을 호출하지만 LspInstallStore에는 종료 호출이 없습니다. install.rs의 guard는 슬롯과 Arc identity를 회수할 뿐 Child나 reader를 소유하지 않으며 Store에는 전체 취소·종료 후 신규 시작 거절 API가 없습니다. TaskSupervisor.stop_all도 기존 등록 작업만 중단하므로 설치 command의 Child를 정리한다는 근거가 아닙니다.

run_toolchain_install은 명시 cancel에서만 process group 신호·start_kill·wait를 실행합니다. success는 reader receiver를 기다리지 않고, failure는 stderr/stdout receiver를 순차 await합니다. capture_output_tail은 JoinHandle 없이 reader task를 직접 spawn하므로 receiver Drop이 reader 취소·join을 보장하지 않습니다. 이 정적 조사만으로 실제 자식 잔존을 재현했다고 기록하지 않습니다. 후속 변경은 synthetic fixture로 요청 취소·앱 shutdown·reader EOF 지연을 구분해 먼저 재현해야 합니다.

## 트리 이후 application body 대조

트리 5개 action과 helper·기존 unit 4개는 58acd6f로 runtime에 이전·검증됐습니다. 앱 파일 읽기·쓰기와 비IPC apply_settings_file의 guard→파싱/검증→적용 포트 await도 runtime 이전 대상으로 확인했습니다. app_get_info는 제품 패키지 버전 원천, perf_snapshot·perf_reset은 process-wide registry adapter이므로 파일 action 이전에 섞지 않습니다. SettingsApplyPort의 실제 AppHandle만 Tauri closure에 유지하며 remote_gateway의 gated 필드 strip과 sync 적용은 불변이어야 합니다.

layout/commands.rs의 제품 body 19개를 읽었습니다. 공통 run_layout_mutation 소비처는 현재 14개이며 기존 private 문서의 13개는 과거 설명입니다. 나머지는 조회·open·close·untitled 변환·경로 변경입니다. 공통 경로는 guard→layouts clone→locate→변경→finish_mutation 이벤트→state write 순서이고 이벤트가 state write보다 앞이라는 기존 계약을 이번 조사에서 수정하지 않습니다. 경로 변경은 프로젝트 root 검사 후 guard를 취득하며 closed-stack만 바뀐 경우 dirty/state만 갱신하고 LayoutChanged를 발행하지 않습니다. 실제 창 이동은 composition root의 별도 command로 현재 19개와 구분합니다.

layout/service.rs의 flush_dirty_layouts·finish_mutation·open_tab_and_finish·close_tab_and_finish는 UI 비의존 상태/영속/이벤트 조립과 AppHandle observer 조회가 섞여 있습니다. flush는 dirty drain→snapshot→save이고 shutdown은 동기 호출, 주기는 awaited blocking 호출입니다. close observer는 state write 이후지만 원본 mutation guard가 해제되기 전에 실행합니다. IDE와 command의 기존 open/close service 경로를 보존한 채 runtime으로 상태 조립을 옮기고 observer를 주입하면 실제 앱 없이 이 경계를 검증할 수 있습니다. source contract의 순서 검사와 기존 unit도 함께 이동해야 합니다.

작은 adapter 13개도 body를 읽었습니다. theme 5개 중 current는 follow_system_theme·설정 theme_id 선택, locale 3개 중 current는 설정 language snapshot·system language resolver를 조립합니다. 이 두 selector는 native 소비 시 같은 정책을 재사용하도록 분리 대상입니다. 나머지 theme 4개·locale 2개·snippet 3개는 이미 분리한 service에 경로/입력을 위임합니다. task 1개는 공통 project_root 검증 뒤 blocking 실행, font 1개는 process font cache의 blocking 실행 adapter입니다. 단순 위임을 새 도메인 정책으로 오인하거나 시스템 폰트 스캔 실기를 실행하지 않았습니다. 이 부분 대조만으로 전체 203개 body 판정을 완료하지 않습니다.

추가 확인: `taide-infra::lsp_proc::spawn`의 stdout reader·wait worker는 JoinHandle을 저장하지 않습니다. stderr reader는 wait worker가 소유하지만 timeout에 handle을 값으로 넘기므로 시간 초과 시 reader를 abort/join하는 경로는 없습니다. reader/wait의 소유권·종료를 synthetic child로 검증하기 전에는 detached 작업의 종료를 완료했다고 기록하지 않습니다. 기존 wait의 kill request/exit select와 PID 재사용 방지·stderr tail 정책은 보존 대상입니다. PTY의 product flusher/reader/wait 세 thread도 handle 없이 spawn한다는 정적 관찰을 유지합니다.

`system/commands.rs` 제품 command 7개와 `notification/commands.rs` 2개도 전체 body를 읽었습니다. system의 경로 3개는 열린 프로젝트 snapshot의 strict owning-root 검증→기존 platform open/reveal/browser 호출이며 external URL은 기존 validate_external_url→platform입니다. app-data는 허용 enum의 디렉터리 선택·생성→reveal 조립입니다. usage get은 독립 store의 awaited blocking 호출이고 breakdown은 PID·CPU 수·AppHandle label provider 수집 후 blocking records→이미 독립한 service::build_usage_processes 조립입니다. notification은 기존 자격증명 masking→settings snapshot→모든 실제 창의 focus 조회→순수 decide_delivery→platform 전송입니다. 실제 창 focus와 platform 호출은 OS adapter, snapshot/마스킹/호출 조건은 application 경계의 분리 대상입니다. 설정 열기는 macOS의 고정 URL만 허용하는 기존 cfg 정책을 유지해야 합니다. 실제 OS·사용자 프로세스·알림·외부 URL 실행은 하지 않았습니다.

composition root의 layout_move_tab_to_window와 auxiliary tab return body도 대조했습니다. 전자는 guard→clone→target 선택→새 창 선생성→move 실패 시 창 close rollback→빈 보조 창 정리→finish 이벤트→state write입니다. 후자는 이미 TaskSupervisor를 경유하지만 guard 안의 mirror snapshot·phantom dirty 정리·탭 복귀·state write→dirty/event 정책이 AppHandle adapter와 섞입니다. 두 경로의 event/state 순서가 다르므로 공통화로 순서를 바꾸지 않아야 합니다. 실제 창 생성/close는 adapter에 유지하며 현재 source unit도 함께 이전/수정할 대상입니다.

메뉴 click dispatch의 transient 감독과 별개로 listen_for_app_menu_refresh의 recent/settings 두 listener는 spawn_blocking 결과를 저장·await하지 않습니다. inline listener에서 디스크 I/O를 수행하지 않는 현재 조건은 유지하되, 이 blocking 작업의 등록·중복/종료와 실제 OS 메뉴 갱신 adapter의 경계를 추가 확인해야 합니다. 등록 이름만으로 모든 메뉴 작업이 감독된다고 해석하지 않습니다. 이번 추가 body 대조로 전체 203개 판정을 완료하지 않습니다.

## OS·알림 정책 이후 감독 경계

system 정책 5개와 notification_notify는 5c684ab로 runtime 이전·단위 검증됐습니다. usage/실제 focus/설정 cfg adapter는 유지하며 전체 종료 판정은 아닙니다. 위 표와 body 대조는 당시 조사 기록이고 현재 완료 근거는 PROCESS와 개별 이력입니다.

TaskSupervisor의 기존 stop_all은 async AbortHandle을 꺼내 취소하고 추적 목록을 비웁니다. 이 방식을 이미 실행 중인 blocking worker에 그대로 적용하면 worker는 살아 있는데 tracked_count가 0이 될 수 있습니다. 현재 Cargo.lock의 Tokio 1.53.1과 같은 버전의 [Handle::spawn_blocking](https://docs.rs/tokio/1.53.1/tokio/runtime/struct.Handle.html#method.spawn_blocking)·[AbortHandle::abort](https://docs.rs/tokio/1.53.1/tokio/task/struct.AbortHandle.html#method.abort) 계약을 확인했습니다. 이미 시작한 worker는 abort할 수 없고 아직 queued인 경우만 시작을 막을 수 있으므로 worker 자체의 등록과 실제 완료까지의 추적이 필요합니다. async waiter를 취소한 것을 OS 작업 종료로 기록하지 않습니다. listener inline IO와 설정 언어 gate를 유지한 채 synthetic worker로 먼저 검증하며 실제 메뉴 실행/OS 종료 대기는 이번 자동 검사의 범위가 아닙니다.
