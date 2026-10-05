# M8 원격 task·font·system 실제 backend

## 대상과 결과

대상은 `native/taide-native-app/src/remote-utilities.rs`, `remote-utilities-tests.rs`, `lib.rs`, native manifest/lock과 `crates/taide-system/src/store.rs`입니다. 원본 task1/font1/system2명령을 실제 task_actions·폰트 cache·system usage DTO/분류·blocking 감독자에 연결했습니다. 선행164+4=168/177 domain adapter·남은9이며 전체 M8 N1~N8은 미완료0/8·목표 active입니다.

## 구현 경계

- 필수 remaining JSON/channel/raw·바깥 with_policy/default-deny/owner 강제를 유지합니다. utility에는 전송 channel이 없으며 새로운 외부 URL/파일 열기 허용을 추가하지 않습니다.
- task는 기존 root project lookup→blocking task detection을 사용합니다. package/Bun lock·GNUmakefile 우선순위·Cargo 발견과 원본 quoting/순서를 재사용하고 발견한 명령은 실행하지 않습니다. root lookup은 mutation/shutdown blanket gate로 바꾸지 않습니다.
- font는 기존 taide-font::service::list_families의 프로세스 수명 OnceLock cache를 실제 production Ports::new에 연결합니다. native preview의 Database로 별도 font 목록 정책을 작성하지 않았습니다. cold scan·cache clone은 원본처럼 blocking worker에서 실행되며 metadata/DTO를 바꾸지 않습니다.
- SystemUsageStore는 typed UsageProvider를 실제 구현합니다. production Ports::new는 services의 동일 store를 clone하며 자체 별도 sampling store를 만들지 않습니다. root PID helper는 기존 sysinfo get_current_pid/as_u32/Internal 오류를 그대로 노출합니다. CPU count의 available_parallelism/fallback1을 유지합니다.
- breakdown은 원본 operation lease→root PID→등록 순서의 domain labels→CPU count→supervised records→기존 build_usage_processes 순서입니다. terminal→agent→LSP provider factory를 원본 store/프로젝트 label에 연결했고 같은 PID의 나중 label 우선·app label TAIDE·descendant 한정·메모리 역순·첫 CPU sample null을 유지합니다. 생산용 native App에서 실제 provider factory를 사용하도록 조립하는 작업은 남습니다.
- font/get은 run_blocking_result만, breakdown은 별도 operation lease도 보유합니다. 요청 waiter를 버려도 이미 시작된 blocking scan이 실제 끝날 때까지 추적됩니다. 상태 shutdown flag만으로 조회를 막지 않으며, TaskSupervisor가 stop되면 factory를 실행하지 않습니다.
- native에 기존 workspace taide-font/taide-system 직접 edge2개만 추가했습니다. taide-font path package1개와 이미 있던 fontdb0.24.0을 native lock에 연결했으며 registry package/버전/checksum·root manifest/lock/MSRV는 바꾸지 않았습니다. root store diff는 PID helper6줄뿐입니다.
- 사용자 앱/프로세스/OS 설정·Keychain·보호bundle·제품TS·TAIDE Git은 조작하지 않았습니다. 테스트는 UUID 합성 project·mock usage/font/PID/label provider를 사용했고 실제 시스템 font scan이나 process sampling을 실행하지 않았습니다.

## 검증

환경은 CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo, native manifest, locked/offline, target experiments/native-shell-spike/target입니다. Cargo는 직렬로 실행했습니다.

- [x] 최초 compile은 존재하지 않는 fixture events::Inbox를 상정해 실패했습니다. 실제 EventSink 계약을 확인한 뒤 utility의 이벤트 발행을 거절하는 test sink로 정정했습니다. 본문 실행 전 source posix_quote를 확인해 task fixture의 quoted command 기대값도 맞췄습니다. 이 시점은 실행 검사 성공으로 세지 않습니다.
- [x] 최초 실제 `cargo test --lib remote_utilities::tests -- --test-threads=1`: compile12.98초/suite0.06초,3 PASS/1 FAIL입니다. task의 없는 project 오류가 plain NotFound인데 fixture가 localized 오류를 기대했습니다. uppercase OS를 포함한 새 검사 이름 경고도 확인했습니다.
- [x] 오류 기대값을 원본 root_guard에 맞추고 실패한 `remote_utilities::tests::catalog_`만 확인: compile3.67초/suite0.02초,1 PASS입니다. 정상3건은 재사용하며 검사 이름만 snake case로 정정했습니다. 신규 고유4검사 모두 PASS입니다.
- [x] catalog4/actual routing arm/allowlist·선행 domain 비중복·typed projectId·없는 project·JSON/raw remaining·remote owner·금지된 system_open_path의 backend 미실행을 확인했습니다. 새 합성 package/Bun/GNUmakefile/Makefile/Cargo에서9 task를 실제 발견하고 우선순위·quoted commands·cwd·미실행 marker를 확인했습니다.
- [x] mock font의 camelCase DTO·monospaced와 usage CPU null/memory DTO를 확인했습니다. mock ProcessRecord를 실제 기존 build_usage_processes에 전달해 label collision의 terminal→agent→LSP 우선순위·root/descendant/외부 PID 제외·gopls fallback 분류·CPU50%·메모리 정렬·첫 sample null을 확인했습니다. native app state shutdown flag 이후 조회 허용과 실제 supervisor stop 이후 factory 비실행을 확인했습니다.
- [x] font/get/breakdown3개의 각각 시작된 blocking mock worker를 실제 gate로 붙잡아 request waiter를 폐기했습니다. worker1 추적·shutdown 미완료→gate release→worker completion/task0을 확인했습니다. 이3경로는 같은 동작을3번 계측하는 것이 아니라 각 command의 독립된 closure/lease/worker 수명을 덮는 한 검사입니다.
- [x] root PID port 실패 시 label/CPU/sample 비실행·operation lease 회수, font scan panic의 원본 Internal 오류를 확인했습니다. 실제 production constructor/provider3개의 조립은 확인했지만 빈 session store였으며 OS provider 자체를 실행한 것으로 세지 않습니다.
- [x] native `cargo clippy --lib --bin taide-native-app --tests -- -D warnings` exit0,16.46초·native authored3/root1 exactfmt exit0입니다. 기존 Wry dependency17경고는 별도이며 새 authored 경고/suppression은 없습니다.
- [x] root `cargo clippy --manifest-path Cargo.toml --locked --offline -p taide-system --lib --tests -- -D warnings` exit0,1분55초입니다. native 성공을 반복한 검사가 아니라 별도 root workspace의 PID helper/dependency feature·strict 경계 검사입니다.

## 미완료와 다음 경계

- [ ] 나머지9 backend는 AI6/sync3입니다. 실제 secret/HTTP/설정 apply·request owner/cancel·sync writeback/conflict·원본 취소/종료 수명을 연결하고 외부 서비스가 아닌 합성 fixture만 검증합니다.
- [ ] 전체 App와 production OS usage/font/provider·전체 dispatcher/Settings/assets/Exit·HostBridge·TS 제거/Rust99%·배포/rollback은 미완료입니다. 이번 mock 결과를 시스템 실측/전체 App 조립으로 세지 않습니다.
- [ ] 기존 keybinding focus-scroll RED·PTY remount 결정·LSP 정상 종료 handshake/recovery·실기 GUI/AX/IME/성능은 기존 M8 gate입니다. 전체 완료 뒤에만 commit/push합니다.

## 근거

원본 Tauri gateway/font/system commands·system_usage_label_providers·root task_actions/task service·taide-font cache·SystemUsageStore/ProcessRecord/build_usage_processes와 actual TaskSupervisor를 확인했습니다. 설치된 official sysinfo0.39.6 source에서 지원 플랫폼 PID/unsupported 오류 계약을 확인했으며 기존 native fontdb0.24.0 package/version/checksum과 root taide-font path package를 대조했습니다. 새 registry 의존성을 추가하거나 OS provider를 기억만으로 작성하지 않았습니다.
