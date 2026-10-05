# M8 원격 명령 정책의 Rust 코어·native gate

## 대상과 완료 경계

`crates/taide-remote/src/command-policy.rs`, `lib.rs`, `src-tauri/src/remote_gateway.rs`, `native/taide-native-app/src/remote-gateway.rs`, `remote-gateway-tests.rs`, native `lib.rs`·manifest/lock이 대상입니다. 기존 N5-S1 remote command policy pending 범위입니다. 정책 코어 이동과 필수 backend 앞의 native gate는 구현했으며 전체 native command backend·앱 조립·M8은 미완료입니다.

원본의 enum/denial_error·허용/거부 목록을 코어로 이동했습니다. Tauri는 같은 공개 목록/오류 타입을 import하고 기존 routing과 조건부 handler를 유지합니다. 현재 실제 목록은 허용177·무조건 거부29개입니다. 원본 오래된 주석의176개를 현재 개수로 주장하지 않습니다. 기존 compiled 검사에서 bindings/구현 목록·허용/거부의 정확한 분할·실제 match arm 집합이 일치합니다.

## 구현 계약

- 코어 `command_policy::admit`는 무조건 거부를 먼저 적용하고, 허용 목록에 없으면 Unclassified/Localized Forbidden을 반환합니다. 11개 정책 variant의 locale key·fallback·command arg와 각 분류를 유지합니다. Tauri의 복사본은 제거해 단일 목록/오류 정의를 사용합니다.
- native `remote_gateway::with_policy(Dispatch)`는 필수 backend의 JSON/raw 앞에 gate를 연결합니다. default/no-op backend는 제공하지 않으며 미구현 handler를 성공으로 위장하지 않습니다. raw는 기존 file_read_raw만 backend로 전달하고 JSON에서 file_read_raw를 직접 부르는 경우도 원본 JSON match처럼 Unclassified입니다.
- admission 뒤 기존 root `enforce_remote_owner_label`로 중첩 객체/배열의 owner를 강제합니다. hook 설치는 원본처럼 projectId→agentName 입력 검사 후 실제 `taide_agent::service::hook_scope_for_agent`로 User scope만 거부합니다. Project는 통과, 알 수 없는 이름은 backend의 원래 InvalidArgument 판정으로 넘깁니다. hook uninstall이나 허용 pty_spawn을 임의로 제한하지 않습니다.
- settings_update의 typed patch는 기존 root strip 함수로 remotePasswordOnlyLogin/remoteAllowedHosts/shellOverride/aiOmlxBaseUrl 변경을 제거합니다. 원본처럼 remoteAccessEnabled의 자가 차단은 허용합니다. app_file_write는 target/content를 먼저 검사하고 Settings일 때 원본 settings parser와 전체-file strip 함수를 재사용해 현재 보호값을 보존한 본문을 backend에 전달합니다. Prompt 본문은 이 보호 처리로 다시 작성하지 않습니다.
- 전체 Settings 보호 snapshot은 원본처럼 실제 write mutation guard 이전의 current Settings 읽기입니다. 이 migration을 동시 mutation의 새 원자성 계약으로 확대하지 않습니다. 원본 JSON serializer/error shape와 기존 파일 persist/runtime 경로를 재사용합니다.

## 실행 증거

모든 Cargo는 `CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo`, `--locked --offline`, target `experiments/native-shell-spike/target`이며 직렬 실행입니다. 실제 사용자 파일·키체인·OS/GUI·hook 설치·보호 bundle을 사용하지 않았습니다. Tauri는 정확한 lib unit filter만 실행했고 앱이나 bundle을 빌드/실행하지 않았습니다.

- [x] `cargo test -p taide --lib remote_gateway::tests:: -- --nocapture`: compile1분42초/suite0.01초, 기존 관련37검사 37 PASS입니다. 이동된 코어 목록·정책 오류와 기존 Tauri routing/bindings/조건부 scope·owner·gated Settings 계약을 확인했습니다. 전체 Tauri/FE/GUI 검사는 아닙니다.
- [x] `cargo test --manifest-path native/taide-native-app/Cargo.toml --lib remote_gateway::tests:: -- --nocapture`: compile13.70초/suite0.05초, 신규2검사 PASS입니다. 29개 무조건 거부의 JSON/raw backend 비실행·unknown/wrong mode·4종 User hook scope·Project/unknown handler 위임·projectId 검사 순서·깊은 owner 강제와 실제 patch/전체 Settings persist·이벤트·reconcile 호출·보호4필드 유지·잘못된 입력의 backend 비실행을 확인했습니다.
- [x] native lib/bin/tests strict clippy는 테스트 MutexGuard의 명시적 drop만으로 await 범위 판정이 해소되지 않아 첫 실행에서 실패했습니다. test guard를 실제 lexical block으로 제한해 관련 정적 검사만 다시 실행했고 exit0/11.52초입니다. guard는 수정 전에도 await 전에 drop됐으며 입력/정책/제품 동작이 바뀌지 않아 성공2검사는 재실행하지 않았습니다. 기존 Wry dependency 경고17개를 억제하지 않았습니다.
- [x] root `cargo clippy -p taide-remote --lib --tests -- -D warnings` exit0/0.64초입니다. 작성 파일 이름을 kebab-case command-policy.rs로 정리하고 public Rust module name은 path attribute로 유지한 최종 선언 검사만 exit0/0.57초로 확인했습니다. API·목록·행동이 같아 기존 성공37/2검사를 반복하지 않았습니다.
- [x] native authored3 edition2024/root·Tauri authored3 edition2021 exact rustfmt check exit0, tracked diff check exit0, 신규 코드3파일 no-index whitespace는 빈 출력입니다. native manifest에는 DeserializeOwned/Serialize 입력·출력 경계와 기존 Settings parser 사용을 위한 이미 transitive인 serde/taide-settings 직접 edge2개만 추가했습니다. offline metadata exit0·새 package/version 없음·root lock/MSRV 불변입니다.
- [x] PROCESS/HANDOFF/재개/QA4문서 Prettier는 전부 unchanged입니다. tracked 문서·Tauri/root lib whitespace는 빈 출력/exit0, 신규 core/QA no-index whitespace는 빈 출력/exit1(새 파일 diff 존재)입니다. 최종 QA 체크 갱신은 이 문서만 포맷하며 제품 검사를 반복하지 않습니다.

## 남은 작업과 부채

- [ ] 허용177명령의 실제 native backend와 channel adapter·production IDE layout/diff/save·PTY/LSP/search/AI·프로젝트/파일 수명·Settings IDE→hooks→remote를 원본 전체 명령 계약에 연결합니다. 지금 fixture의 echo/scope 확인은 제품 handler가 아니며 remote gateway 전체 완료로 세지 않습니다.
- [ ] 제품 remote HTTP assets/개발 proxy·state/event bridge·NativeApplication startup/Exit·HostBridge AppFile write/저장 키와 실제 원격 UI를 조립합니다. with_policy가 생산용 socket_action 앞에 반드시 적용되도록 composition root에서 확인합니다.
- [ ] 전체 live backend의 정책 적용·동시 Settings write·permit 대기/큰 raw/channel·사용자 session 여러 개/OS gate·공유 domain owner 수명과 보안/메모리/성능을 검증합니다. 재현 조건은 실제 backend assembly 이후 다중 요청/시스템 동작이며 지금 pure gate/합성 persist 증거로 대신하지 않습니다.
- [ ] 원본의 terminal RCE 허용, 이미 permit을 기다린 요청의 session 재검증 미수행,256-frame만의 queue 상한은 유지합니다. 정책 분류는 dedicated command surface의 축소이지 전체 confidentiality/실행 권한 봉쇄가 아닙니다. 별도 보안 계약 없이 상한/거절 정책을 바꾸지 않습니다.
- [ ] 전체 N1~N8 0/8·TS 제거/Rust99%·213view/41action/Monaco21·실제 픽셀/AX·성능/배포/rollback·keybinding Tab RED·PTY remount 결정은 여전히 미완료입니다. goal active이며 전체 M8 완료 전 commit/push하지 않습니다.

## 근거 파일

원본 remote_gateway의 gate/argument/schema/handler와 실제 root policy·Settings parser/runtime·agent hook_scope_for_agent를 대조했습니다. 변경 전 목록/enum/오류 본문을 정확한 source 경계에서 추출해 apply_patch로 이동했으며 원본의 약430줄 전체를 새 정책으로 재설계하지 않았습니다. 금지 코드 주석은 새 코어에 추가하지 않았고 이유와 실행 근거를 이 문서에 저장합니다. Rust fn/enum·typed 공개 callback 경계는 기존 M8 Rust 계약을 따릅니다.
