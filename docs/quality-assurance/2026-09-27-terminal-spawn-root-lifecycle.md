# 터미널 spawn/root 소유권 QA

## 대상 파일과 리포트

TerminalStore 입장/완료 목록과 runtime 실제 blocking spawn·동일 mutation guard·정상 ExitDrain, Tauri command/원격 State 주입을 검사합니다. 종료 요청·실제 worker join·native 실기를 구분합니다.

## 실제 결과

- [x] 새 store admission API 두 검사: 구현 전 E0599(exit 101), 구현 뒤 2건 통과(exit 0)입니다. 컴파일 API RED이며 실제 OS lifecycle 실패로 집계하지 않습니다.
- [x] `cargo test --offline -p taide-terminal --lib store::tests --quiet`: 최종 4건 통과(exit 0)입니다. 마지막 lease·복수 대기자/대기 Drop와 자기 child callback을 붙잡은 kill/project/replace 세 경로의 실제 idle 지연·재대기·목록 정리를 확인했습니다. 신규 fixture의 ProjectId 문자열 타입 오류(exit 101)는 수정 뒤 해당 검사가 통과했습니다. 앞 두 admission 성공과 중복 합산하지 않습니다.
- [x] `cargo test --offline -p taide-runtime --lib terminal_actions::tests --quiet`: 6건 통과(exit 0)입니다. 중지한 감독자/queued factory 거절·panic·요청 Drop 뒤 actual worker의 lease/guard 지속·성공 전달의 동일 guard·늦게 생산한 자기 PTY의 callback 종료 대기를 확인했습니다.
- [x] `cargo test --offline -p taide-runtime --lib exit_drain::tests --quiet`: 5건 통과(exit 0)입니다. 정상 root는 자기 PTY callback까지 기다리며 callback panic에는 준비/종료 callback을 실행하지 않습니다. 기존 LSP/설치/메인 응답 계약도 보존했습니다.
- [x] `cargo test --offline -p taide-runtime --lib state::tests --quiet`: 23건 통과(exit 0)입니다. 기존 borrowed/blocking mutation lock과 flush/종료 상태를 확인했습니다. core/runtime은 서로 다른 38건입니다.

## 연결·정적 검사

- [x] `cargo test --offline -p taide --test taide_terminal_store_extraction --test taide_lsp_install_store_extraction --test task_supervisor --test app_services_runtime --test app_state_runtime --quiet`: 3+2+22+2+2로 서로 다른 31건 통과(exit 0)입니다. 실제 두 project 세션의 회수/다른 세션 유지·replay·store clone과 AppServices의 동일 admission/idle, adapter의 supervisor/guard/등록/이벤트 순서 및 root 배선을 확인했습니다. 첫 실행의 원격 spawn State 인자 누락 E0061/E0308/E0277(exit 101)은 같은 원격 호출에 감독자를 추가한 뒤 해소됐습니다.
- [x] `cargo test --offline -p taide --lib collect_commands_매크로_출력과_dispatch_테이블은_커맨드_이름_집합이_일치한다 --quiet`: 현재 Specta builder 임시 생성/dispatch parity 1건, `cargo test --offline -p taide --test rust_native_phase0_contract --quiet`: IPC 기준선 7건 통과(exit 0)입니다. 생성 이름 확인을 전체 bindings payload 동일성으로 확대하지 않습니다. core/runtime 38·Tauri integration 31·생성 parity 1·IPC 7은 서로 다른 77건입니다.
- [x] `cargo test --offline -p taide --lib typescript_바인딩을_생성한다 --quiet`: 실제 bindings 생성 1건 통과(exit 0)입니다. 생성 후 bindings/IPC manifest diff는 없으며 SHA-256은 각각 `f874269e742c9ac204c0b35085a41815743211b18ee33aabd48384f353a5975a`/`343950f91338121d04edd36d5aa8d09491437cdb78e096fa0f519d3315a04bce`로 유지됩니다. 전체 출력 확인까지 이번 단위의 서로 다른 성공 검사는 78건입니다.
- [x] `cargo clippy --offline -p taide-infra -p taide-terminal -p taide-runtime -p taide --all-targets -- -D warnings`: exit 0입니다. `RUSTDOCFLAGS='-D warnings' cargo doc --offline --no-deps -p taide-infra -p taide-terminal -p taide-runtime` 및 공개 ExitDrain 설명을 좁힌 뒤 runtime strict rustdoc도 exit 0입니다.
- [x] `cargo fmt --all -- --check`, `git diff --check`: exit 0입니다. history/새 QA/연결된 worker QA 3개 MD는 기존 docs ignore를 명시적으로 제외한 `bun node_modules/prettier/bin/prettier.cjs --ignore-path /dev/null --write`와 `--check`에서 대상별 출력·exit 0을 확인했습니다. 기본 ignore의 무선택 성공은 대상 포맷 성공으로 집계하지 않습니다.

## 생략과 필수 잔여 gate

2026-09-28 현재 Tauri 배선 재검증에서 `cargo test --offline -p taide --test terminal_spawn_application_runtime --quiet` 7건과 `cargo test --offline -p taide --test terminal_actions_runtime --quiet` 10건이 모두 통과했습니다. 두 대상은 직접 만든 `/bin/sh`·UUID 임시 경로를 사용하며 사용자 profile·프로세스에는 접근하지 않습니다. 앞선 infra PTY 33건과 runtime/core 성공은 입력이 같아 재사용합니다. 이 배선 검사를 SIGHUP 무시 child·Windows·native Exit 실기로 확대하지 않습니다.

### 실앱 격리 사전 대조

대상 파일은 `src-tauri/src/lib.rs`의 setup, `src-tauri/tauri.conf.json`, `src-tauri/tauri.dev.conf.json`, `scripts/tauri.ts`, `crates/taide-infra/src/secret.rs`입니다. setup은 `app.path().app_data_dir()`를 상태 루트로, `app.config().identifier`를 키링 서비스명으로 사용합니다. 배포 identifier `net.gumyo.taide`와 개발 identifier `net.gumyo.taide.dev`는 서로 다르지만 개발 identifier도 고정값입니다. 따라서 실제 GUI/Exit fixture를 기존 배포·개발 데이터와 분리하려면 전용 실행 identifier로 두 경계를 모두 분리했음을 먼저 증명해야 합니다. 이 확인은 코드와 설정의 읽기 전용 대조이며 앱 시작·프로필/키링 조회·사용자 파일 접근·설정 변경은 하지 않았습니다.

- [x] 후속 [PTY 부분 시작 QA](./2026-09-27-pty-partial-startup-lifecycle.md)에서 master reader/writer·세 thread factory 오류/언와인드의 child wait·시작한 worker join·생성 경로 정리를 확인했습니다. 성공-result owner 검사만으로 대체하지 않았으며 실제 OS 오류 회복/abort panic은 별도 gate입니다.
- [x] 자기 `/bin/sh` PTY의 HUP 무시 상태에서 완료 핸들이 100ms 동안 대기하고 fixture 전용 SIGKILL 뒤 실제 join되는 검사 1건을 통과했습니다. [실측 이력](../history/2026-09-28-pty-sighup-ignore-probe.md)의 관찰 시간을 제품 종료 상한으로 해석하지 않습니다.
- [ ] OS wait 오류·pipe 보유/그룹 이탈 자손·non-yield callback/Read의 bounded 종료·runtime join 실패와 SIGHUP 무시 자식의 제품 종료 정책은 미검증·미결정입니다. 종료 요청/실패 cache를 성공한 회수로 해석하지 않습니다. 사용자 프로세스 없이 OS 정책/fixture를 준비한 뒤 수행합니다.
- [ ] Windows/다른 Unix·직접 native Exit·실제 앱 시작/종료는 미실행입니다. native UI 착수나 앱 재시작 권한을 추정하지 않습니다.
- [ ] M6-JP/JQ 전체·OS/실앱 자원·M7/M8·Phase 0은 미완료입니다. command body 정적 대조 M6-HK는 별도 완료했으며 승인받은 일반 push는 M6 전체 완료 이후입니다.

당시 infra의 27건 PTY 및 출력 producer 성공은 생산/완료 본문이 같아 재사용했습니다. 후속 infra 검사는 33건까지 통과했습니다. 이번 infra 수정은 같은 worker identity 비교뿐입니다. frontend 코드와 wire payload가 없어 frontend 전체 실행을 추가하지 않습니다. 추가 source 검사·bindings/원격 contract로 State 주입 위험을 직접 확인합니다.
