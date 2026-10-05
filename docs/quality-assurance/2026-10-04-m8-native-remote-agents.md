# M8 원격 agent 실제 backend

## 대상과 결과

대상은 `native/taide-native-app/src/remote-agents.rs`, `remote-agents-tests.rs`, `lib.rs`입니다. agent6명령을 actual runtime에 연결했습니다. 선행136+6=142/177 domain adapter·남은35이며 전체 M8 N1~N8은 미완료0/8·목표 active입니다.

## 구현 경계

- 자기 명령만 처리하고 필수 remaining JSON/channel/raw와 바깥 with_policy를 유지합니다. typed 원본 project/agentName/marker와 목록→프로젝트 검증→operation→foreground→probe 순서를 유지합니다.
- foreground/probe/CLI 상태/home/emitter를 필수 Ports로 받으며 기본 no-op을 제공하지 않습니다. hooks의 start_server는 이미 검증한 실제 native agent_hooks::ensure_started입니다. production OS foreground/probe/CLI/home/emitter factory와 App assembly는 아직 조립하지 않았습니다. fixture의 가짜 PID/합성 CLI 상태를 production 기능으로 세지 않습니다.
- 목록은 실제 AgentStore/AgentHooksStore·감독 probe/cache·활동/dialog과 원본 marker 검증/삭제/forget를 사용합니다. hook status/install/uninstall은 실제 atomic hook_files와 소유권 정책을 사용합니다.
- 원격 user-scope 설치는 기존 localized DesktopCliInterception으로 거절하되 status/uninstall의 원본 허용을 유지합니다. 프로젝트 설치의 settings gate/emitter/file 오류와 hook 액션 감독자 종료의 Internal 오류를 유지하며 list/release의 Forbidden이나 CLI 상태의 shutdown 무관 동작으로 통일하지 않습니다.
- 모든 사용자 home·CLI/프로세스/OS 설정·앱/보호bundle·manifest/lock/dependency/MSRV/root/Tauri/제품TS는 변경하지 않았습니다.

## 검증

환경은 CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo, manifest native/taide-native-app/Cargo.toml, locked/offline, target experiments/native-shell-spike/target입니다. Git 성공4건과 이전 domain 검사는 관련 source가 같아 재사용했습니다.

- [x] 최초 컴파일은 remote_files의 실제 JSON_COMMANDS 이름과 projectId의 역직렬화 추론에 실패했습니다. 실제 상수와 필요한 ProjectId 경계 타입으로 수정했으며 실행/통과했다고 보고하지 않았습니다.
- [x] 최초 실제3검사: compile9.51초/suite0.04초,3 FAIL입니다. 원격 거절을 plain Forbidden으로 잘못 기대했고, input 자체를 working 신호로 잘못 기대했습니다. 실제 localized 오류 전체값과 input 뒤 unknown·실제 dialog/own 상태 우선으로 fixture를 정정했습니다.
- [x] 정정 후 compile6.66초/suite0.07초:1 PASS/2 FAIL입니다. probe/활동/marker 성공1을 재사용합니다. 남은 실패는 빈 raw 인자에 owner가 새로 생성된다고 잘못 기대한 것과 다른 agent의 codex hook을 제거해야 한다고 잘못 만든 fixture입니다. 원본 owner 치환과 agent-specific 소유권을 그대로 두고 기존 owner 필드와 각 agent의 실제 in-band/HTTP shape로 fixture만 정정했습니다.
- [x] 실패2만 `cargo test --lib remote_agents::tests:: -- --skip 실제_probe_cache --nocapture`: compile6.69초/suite0.10초,2 PASS입니다. 통과한1은 제외했습니다.
- [x] catalog6/실제 arm/allowlist/선행 domain 비중복·denied/default-deny·typed/없는 프로젝트·거절 전 foreground/probe 비실행·remaining JSON/channel/raw/owner 전달·종료 후 원본 오류와 비실행을 확인했습니다.
- [x] 실제 supervised probe에 합성 PID/프로세스 이름 resolver를 주입해 Unix cache1회·활동/dialog/blockedReason·own state를 확인했습니다. marker는 시스템 임시 디렉터리 바로 아래 새 UUID 이름1개만 만들고 원본대로 삭제/forget·missing 재해제·잘못된 파일 이름 거절/외부 파일 불변을 확인했습니다. 사용자 프로세스를 탐지하지 않았습니다.
- [x] 새 합성 프로젝트의 claude hook gate·TerminalSequence 설치·foreign JSON/row 보존·해제·invalid JSON 불변을 확인했습니다. 새 합성 home의 codex/gemini/opencode/pi status/uninstall과 자기 agent-owned JSON/파일 회수·foreign 파일 유지·원격 install 비실행을 확인했습니다. HTTP hook command는 새 fixture의 실행하지 않는 문자열이며 서버/사용자 CLI를 실행하지 않았습니다.
- [x] 독립 신규 `remote_agents::tests::요청취소와`: compile9.68초/suite0.02초,1 PASS입니다. 실제 agent_list→감독 blocking probe를 진입 신호 후 차단하고 caller abort·stop_all·shutdown에도 worker가 tracked 상태로 남는지 확인했습니다. gate 해제 뒤 실제 완료1/tracked0·미발행 상태를 확인했습니다. 정상3검사는 재실행하지 않았습니다.
- [x] 최종 native lib/bin/tests strict `cargo clippy --lib --bin taide-native-app --tests -- -D warnings` exit0,16.65초·authored3 exactfmt exit0입니다. 기존 Wry dependency17경고와 authored 검사는 구분하며 억제를 추가하지 않았습니다.

## 격리와 미완료

모든 hook settings/owned plugin/marker는 새 합성 fixture에서 생성하고 테스트한 것입니다. 설정된 실제 사용자 home·CLI·프로세스·환경 변수는 조회/변경하지 않았으며 cleanup은 자기 임시 디렉터리와 자기 UUID marker만 회수했습니다. 실제 사용자 hook을 지우지 않았습니다.

- [ ] native production OS 포트·나머지35 backend/전체 socket dispatcher·App/Settings/assets/Exit/HostBridge를 연결합니다.
- [ ] 실제 agent/OS ps/HTTP 사용자 hook/버전 emitter·폴링/perf·전체 AppExit/보안·keybinding RED/PTY remount·TS 제거/Rust99%·배포/rollback은 기존 M8 gate이며 이4검사를 전체 완료 근거로 세지 않습니다.

## 근거

원본 Tauri agent commands/gateway, actual agent_actions/agent_hook_actions/agent_probe/store/service/hook_files, 선행 native agent_hooks 실제 서버를 확인했습니다. 사용한 Future/Arc/serde/Tokio supervisor 경계는 기존 구현의 API를 재사용했으며 새 의존성/API 가정을 추가하지 않았습니다.
