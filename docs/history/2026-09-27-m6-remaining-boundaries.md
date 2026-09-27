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
