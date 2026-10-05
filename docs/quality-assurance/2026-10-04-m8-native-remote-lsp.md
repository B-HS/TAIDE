# M8 원격 LSP 실제 backend

## 대상과 결과

대상은 `native/taide-native-app/src/remote-lsp.rs`, `remote-lsp-tests.rs`, `lib.rs`입니다. 원본 허용 LSP10명령을 actual lsp_actions에 연결했습니다. 선행142+10=152/177 domain adapter·남은25이며 전체 M8 N1~N8은 미완료0/8·목표 active입니다.

## 구현 경계

- 필수 remaining JSON/channel/raw·바깥 with_policy·nested request.owner 강제와 default-deny/install 거절을 유지합니다. 원본 remote LspStore를 사용하며 native editor의 별도 SessionClient/DocumentMirror와 동일하게 조립했다고 주장하지 않습니다.
- process factory(services/id/epoch/spec/root)와 PATH 공급은 필수 typed Ports입니다. source runtime의 store process admission/owned resources·project/mutation/operation·owner 공유/root reference·status/재초기화 generation·세션 해제 순서를 유지합니다. production process exit recovery/재시작 backoff/로그 마스킹 factory는 아직 조립하지 않았습니다.
- onMessage prefix 제거·필수 인자의 localized 오류를 유지합니다. Channel<String> 원본처럼 프로토콜 문자열을 JSON 문자열로 직렬화하며 raw 프로토콜 객체로 바꾸지 않습니다. 거절한 sink는 원본 subscribers가 제거합니다.
- root/detect/install-cancel과 조회/confirm/report/send의 원본 shutdown 정책을 유지하고 새 blanket gate를 넣지 않습니다. discovery는 원본 detect_servers에 injected PATH를 전달하며 새 다운로드/설치를 허용하지 않습니다.
- manifest/lock/dependency/MSRV/root/Tauri/제품TS·보호bundle·사용자 앱/설정 변경은 없습니다.

## 검증

환경은 CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo, manifest native/taide-native-app/Cargo.toml, locked/offline, target experiments/native-shell-spike/target입니다. 기존 example `examples/native-lsp-mock`를 재사용했으며 보호 실기 앱을 실행하거나 교체하지 않았습니다.

- [x] 최초 `cargo test --lib remote_lsp::tests:: -- --nocapture`: compile10.45초/suite6.02초,2 PASS/1 FAIL입니다. 실제 child의 첫 initialize 응답 timeout을 확인했습니다. fixture source는 processId/rootUri가 명시적 null이어야 하지만 새 입력에 두 필드가 없었음을 확인했습니다. 제품 timeout/프로세스/parser를 바꾸지 않고 fixture 입력을 정정했습니다.
- [x] 실패한 `remote_lsp::tests::실제_합성프로세스`만 재실행: compile5.65초/suite0.12초,1 PASS입니다. 정상2검사를 재실행하지 않았습니다. 최초 unused import2개를 제거했으며 새 detect 함수의 uppercase 이름 경고도 suppression 없이 정정했습니다.
- [x] 독립 신규 `remote_lsp::tests::실제_detect는`: compile3.67초/suite0.01초,1 PASS입니다. 고유4검사 모두 PASS이며 선행 domain 성공은 재사용합니다.
- [x] catalog10/actual arm/allowlist·선행 domain 비중복·typed 인자/channel localized/prefix·project/server 거절 전 process 비실행·factory 실패 후 session 회수/미발행·없는 session5명령·remaining JSON/channel/raw/owner·shutdown 뒤 실제 spawn admission을 확인했습니다.
- [x] actual LspStore 공유는 incoming main owner를 remote로 강제하고 같은 owner만 재사용합니다. 새 root reference·JSON 문자열 전달·project별 목록·stale generation 무시/current 실패·확인 복구·sink 거절/제거·partial/full root release·Stopped와 종료 후 조회/cancel 원본 수명을 확인했습니다. 별도 다른 owner의 existing 세션은 기존 policy/store 검증의 성공 근거를 재사용합니다.
- [x] 실제 LspInstallStore의 cancel 뒤 token/slot 유지·guard Drop 뒤 재입장을 확인했습니다. 합성 go.mod/하위 파일의 원본 nearest root를 확인했습니다. 실제 toolchain install/download는 실행하지 않았습니다.
- [x] UUID 합성 project cwd의 실제 mock child2개를 순차 spawn하고 stdin JSON→원본 framed stdout→JSON 문자열 channel을 왕복했습니다. restart의 기존 child 종료/실제 completion·새 process/epoch·원본 status 순서·stop의 session 제거/두 child exit callback·is_exited/is_finished·Store wait_for_idle/task0을 확인했습니다. 다른 사용자 프로세스는 조회/종료하지 않았습니다.
- [x] discovery는 builtin command 이름의 새 비실행 합성 파일과 fixture PATH를 사용합니다. manifest row 개수/gopls의 actual resolvedPath/available/initializationOptions·process 비실행을 확인했습니다. 원본 SDK discovery가 고정 시스템 경로의 존재 metadata를 확인하는 정책은 유지하며 사용자 설정/환경 변수를 수정하거나 실제 SDK/LSP를 실행하지 않았습니다.
- [x] 최종 `cargo clippy --lib --bin taide-native-app --tests -- -D warnings` exit0,13.25초·authored3 exactfmt exit0입니다. 기존 Wry dependency17경고는 별도이며 authored 검사 억제는 없습니다.

## 미완료와 fixture 한계

- [ ] 기존 mock은 응답 ID를 unsigned로 제한합니다. root 서비스의 shutdown ID 문자열/미초기화 shutdown은 mock의 protocol 오류로 child를 끝낼 수 있습니다. 이 검사는 실제 종료/회수와 원본 action 순서의 근거이며 정상 initialized→shutdown→exit handshake 성공의 근거가 아닙니다. production recovery/process factory와 전체 AppExit를 조립할 때 정상 handshake와 auto-restart/backoff를 검증합니다. 이 제한을 숨기려고 mock/원본 서비스를 바꾸지 않았습니다.
- [ ] 남은25 backend·전체 dispatcher/Settings/assets/App/Exit/HostBridge·native editor와 remote 세션 소유권·full process recovery/log redaction·GUI/AX/성능/보안·keybinding RED/PTY remount·TS 제거/Rust99%·배포/rollback은 기존 M8 gate입니다. 전체 완료 뒤에만 commit/push합니다.

## 근거

원본 Tauri gateway LSP10/channel factory와 LSP commands/process callback/cleanup·actual root lsp_actions/LspStore/lifecycle/subscribers/roots/install/service/manifest를 확인했습니다. 기존 native mock example/frame/process API를 source에서 확인해 재사용했으며 새 API/의존성을 추측으로 추가하지 않았습니다.
