# M8 원격 preferences 실제 backend

## 대상과 현재 결과

대상은 `native/taide-native-app/src/remote-preferences.rs`, `remote-preferences-tests.rs`, `remote-gateway.rs`, `lib.rs`입니다. 기존 N5-S1 전체 remote backend pending 안에서 설정/테마/로케일/snippet/AppFile/remote 상태·폐기의19개 명령을 실제 runtime에 연결했습니다. 전체 허용177개 중 나머지158개와 생산용 앱 조립은 미완료이며 전체 M8 N1~N8은0/8입니다.

단순 echo가 아닌 실제 settings/theme/locale/snippet/app/remote runtime을 사용합니다. AppInfo와 Settings reconcile, 나머지 domain dispatch는 필수 포트입니다. fixture의 remaining JSON/raw/channel 전달 확인은 제품 handler가 아니며 미구현 handler를 성공으로 등록하지 않았습니다.

## 구현 경계

- `remote_preferences::extend_backend(Ports, remaining)`은 자신의19명령만 실제 handler로 처리합니다. 나머지 JSON은 필수 remaining callback에 channels까지 전달하고 raw는 그대로 유지합니다. 생산용 assembly는 domain backend 전체를 조립한 뒤 바깥에 `remote_gateway::with_policy`를 적용해야 합니다. fixture는 이 실제 합성을 사용해 정책 선행을 확인했습니다.
- 앱 정보는 root AppInfo 타입의 필수 값입니다. 버전/플랫폼을 native 후보 package의 값으로 임의 치환하지 않았습니다. fixture 메타데이터가 synthetic인 것은 실제 제품 artifact/version 검사와 별개입니다.
- 설정3진입점 settings_update/settings_set_theme/AppFile write는 원본 command처럼 `TaskSupervisor::run_nonabortable_result`로 admitted 작업을 유지합니다. patch·theme 입력은 worker admission 전에 좁히고 mutation guard→sanitize/persist/live state→필수 reconcile→SettingsChanged→응답 순서입니다. theme 선택은 원본의 추가 ThemeChanged 순서를 유지합니다.
- AppFile write는 root target/content·settings parse/apply·prompt3 validate/atomic write를 재사용합니다. 필수 reconcile이 실제 앱의 IDE→hooks→remote 조립을 대체하는 no-op은 아닙니다. callback 호출 순서·파일/state/event 일치는 합성 포트로 검증했으며 실제 observer 구현은 다음 단계입니다.
- theme5/locale3/snippet3명령은 기존 runtime의 identifier/root guard·builtin 보호·상속/해석·원본 snippet 서식 보존·오류·현재 system fallback을 사용합니다. theme/save/delete나 snippet에 원본에 없던 이벤트나 디자인을 추가하지 않았습니다. remote_status/revoke_sessions는 기존 store/service를 그대로 사용합니다.
- 공통 argument/error wire helper는 기존 gateway에서 crate-visible로 재사용합니다. DTO를 수동 중복하지 않았으며 공개 handler·포트는 Rust 타입 경계입니다. 새 dependency/package/lock/MSRV·제품TS·보호 bundle·OS·Git 변경은 없습니다.

## 검증

모든 Cargo 환경은 `CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo`, native manifest `native/taide-native-app/Cargo.toml`, `--locked --offline`, target `experiments/native-shell-spike/target`입니다. 정확한 새 test filter만 실행했고 실제 앱/OS/키체인·사용자 데이터는 사용하지 않았습니다. 이 단계는 network transport를 바꾸지 않아 이전 WS/HTTP 성공 검사를 반복하지 않았습니다.

- [x] `cargo test --lib remote_preferences::tests:: -- --nocapture`: compile8.26초/suite0.12초, 신규3검사 PASS입니다. 목록/중복/허용 집합·실제 match arm의 정확한 일치와 mandatory remaining/channel/raw 전달,19명령의 실제 합성 파일/state/이벤트 왕복, 쓰기3진입점의 mutation 대기 중 RPC waiter 취소·감독자 종료와 admitted worker 완료를 확인했습니다.
- [x] 실제19명령 검사는 custom theme 저장/조회/목록/현재/삭제·실제 locale catalog/get/current·snippet 본문 그대로 저장/조회/삭제·Settings patch/theme 선택·AppFile Settings canonical 및 prompt 기본/override read/write·상태 조회/세션 폐기를 모두 호출합니다. 보호 설정값 유지, invalid theme/snippet 경로·잘못된 argument 거절도 확인했습니다. 메타데이터는 필수 synthetic AppInfo 포트의 전달 검사입니다.
- [x] 마지막 쓰기 수명 검사의 null 재요청은 입력 오류만으로도 실패할 수 있어 종료 뒤 admission 거절 근거로 쓰지 않았습니다. 같은 유효 입력을 다시 전송하고 Forbidden을 확인하도록 증거를 강화했습니다. 영향1건만 `--exact` 재실행해 compile4.17초/suite0.07초 PASS이며 나머지 성공2건은 재사용했습니다. 최초 test 이름의 한글 표기 정정도 이 관련 compile에 포함됩니다. 같은 상태의 성공3회를 반복한 증거가 아닙니다.
- [x] native lib/bin/tests `cargo clippy --lib --bin taide-native-app --tests -- -D warnings` exit0,13.63초입니다. 기존 Wry 의존성 경고17개는 authored 검사와 구분했으며 경고 억제는 추가하지 않았습니다.
- [x] authored4파일 edition2024 exactfmt exit0, 신규 source4파일 no-index whitespace는 빈 출력/exit1(새 파일 diff 존재)입니다.
- [x] PROCESS/HANDOFF/재개/QA4문서의 Bun Prettier 결과는 모두 unchanged입니다. tracked2문서 whitespace exit0, 신규2문서 no-index whitespace는 빈 출력/exit1(새 파일 diff 존재)입니다. 마지막 결과 표기 갱신은 QA 단독 포맷으로 확인합니다.

취소 검사는 실제 settings mutation guard를 held한 상태에서 worker operation을 확인하고 RPC waiter를 abort합니다. supervisor shutdown이 해당 guard 대기 중 끝나지 않음을 확인한 뒤 release하며, 디스크·live Settings 일치와 reconcile→SettingsChanged·theme 추가 이벤트·tracked0을 검사합니다. 원래 admitted 쓰기를 임의 취소하거나 저장을 성공으로 위장하지 않았습니다.

## 미완료와 부채

- [ ] 나머지158허용 명령의 실제 project/layout/file/tree/agent/IDE/Git/PTY/LSP/search/AI/sync/system backend와 native channel adapter를 연결합니다. 이19명령과 단일 coordinator만으로 full remote gateway를 완료로 세지 않습니다.
- [ ] 제품 AppInfo/version·실제 Settings IDE→hooks→remote 순차 observer·remote assets/개발 proxy·state/event bridge·NativeApplication startup/Exit·HostBridge AppFile write/저장 키를 조립합니다. 필수 synthetic observer의 호출 검사를 제품 서버 조립 성공으로 확대하지 않습니다.
- [ ] 실제 remote socket을 통한 전체 backend·여러 session/동시 mutation·현재 protected Settings snapshot의 동시성·오래 살아 있는 domain channel·전체 memory/보안/성능을 확인합니다. 재현 조건은 production assembly와 실제 병렬 요청이며 지금 직접 typed-port/runtime 검증으로 대신하지 않습니다.
- [ ] 원본 remote handler의 동기 theme/locale/snippet IO를 async worker에서 그대로 사용합니다. UI/네트워크 전체 latency와 thread-pool pressure는 측정하지 않았습니다. 변경 없이 시간 정책·worker/byte quota를 임의 추가하지 않으며 M8 성능 gate에서 관찰합니다.
- [ ] TS 제거/Rust99%·213view/41action/Monaco21·전체 픽셀/AX·성능/배포/rollback·keybinding Tab RED/PTY remount 결정은 기존 미완료입니다. goal active·전체 완료 전 commit/push 없음입니다.

## 원본·API 근거

`src-tauri/src/remote_gateway.rs`의 실제19 routing arm과 domain settings/theme/locale/snippet/app commands, root runtime/action/store를 대조했습니다. Settings write의 nonabortable supervision과 observer registration-order/이벤트 순서는 원본과 같습니다. [serde_json from_value](https://docs.rs/serde_json/latest/serde_json/fn.from_value.html)와 [Tokio JoinHandle](https://docs.rs/tokio/latest/tokio/task/struct.JoinHandle.html) 공식 문서, 실제 설치된 Tokio1.53.1을 사용하는 root TaskSupervisor 구현을 확인했습니다. latest 문서는 Tokio1.53.2이며 제품/lock을 그 버전으로 업그레이드하지 않았습니다.
