# M8 원격 IDE 실제 backend

## 대상과 결과

대상은 `native/taide-native-app/src/remote-ide.rs`, `remote-ide-tests.rs`, `remote-gateway.rs`, `remote-preferences.rs`, `lib.rs`입니다. 원본 원격 IDE7명령을 `AppServices.ide`와 실제 runtime에 연결했습니다. preferences19와 함께26명령의 domain adapter이며 제품 전체 backend·IDE 화면·앱 조립 완료가 아닙니다. 같은 세션 후속14 file/tree20명령까지 합하면46/177이며 나머지131명령과 생산용 assembly는 미완료입니다. 전체 M8 N1~N8은0/8입니다.

## 구현 경계

- `remote_ide::extend_backend(remaining)`은 IDE7명령만 처리하고 다른 JSON은 channels까지, raw는 그대로 필수 remaining backend에 전달합니다. 생산용 assembly는 전체 domain backend를 연결한 뒤 바깥에 `remote_gateway::with_policy`를 적용해야 합니다. fixture는 이 실제 policy 합성을 사용합니다.
- status·selection·diagnostics·at-mention·diff/save 응답은 원본 `ide_actions`를 그대로 호출합니다. owner의 중첩 입력도 policy가 remote로 강제하므로 로컬 current/latest selection을 덮거나 지우지 않습니다. diagnostics와 at-mention notification에는 원본에 없는 application event를 추가하지 않았습니다.
- Saved diff는 bootstrap의 실제 `IdeSaveFile(save_file_within_open_projects)`로 원본 mutation guard·열린 프로젝트 경로 검증·원자적 저장·self-write mark·mirror 정리를 사용합니다. 새 무검증 저장 포트나 synthetic 성공 handler로 치환하지 않았습니다.
- 원본은 pending을 먼저 소비하고 mutation guard를 기다립니다. guard 대기 중 RPC future 취소는 responder를 버리고 디스크를 쓰지 않으며 재요청은 NotFound입니다. Settings3쓰기의 nonabortable 정책을 IDE에 임의 추가하지 않았습니다.
- 공통 `respond` serializer를 preferences에서 gateway로 이동해 두 실제 domain adapter가 재사용합니다. 본문/오류 변환은 동일하며 preferences 기존 성공을 반복하지 않았습니다. DTO·검증·신규 의존성·root/Tauri/manifest/lock/MSRV·제품TS·실기 bundle을 변경하지 않았습니다.

## 검증

Cargo는 `CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo`, native manifest `native/taide-native-app/Cargo.toml`, `--locked --offline`, target `experiments/native-shell-spike/target`이며 한 번에 하나만 실행했습니다. 합성 임시 프로젝트와 실제 bootstrap/store/runtime만 사용하고 앱·키체인·사용자 파일·OS 설정을 조작하지 않았습니다.

- [x] `cargo test --lib remote_ide::tests:: -- --nocapture`: compile6.72초/suite0.04초, 실제 guarded 저장/mirror 정리/save 응답1 PASS입니다. 다른2검사는 잘못된 기대값 때문에 실패했으며 성공으로 보고하지 않았습니다.
- [x] unknown 명령의 policy 오류는 `code=Localized`, `message.kind=Forbidden`입니다. 닫힌 프로젝트의 root guard도 localized Forbidden이며 원본 `ide_actions`의 `Err(AppError::Forbidden(_))` 분기에 해당하지 않아 응답 오류·responder 폐기입니다. 이전 Tauri 주석의 “닫힌 프로젝트에도 Saved” 설명을 실제 동작으로 가정했던 기대값을 수정했습니다. 제품 동작을 바꾸거나 오류를 삼키지 않았습니다.
- [x] 실패한2검사만 `--skip diff_saved는_실제_guarded_저장과_mirror_정리를_거치고_save_응답을_해소한다`로 재실행해 compile2.98초/suite0.04초,2 PASS입니다. 이미 성공한 저장 검사는 제외했습니다. 고유3검사 모두 PASS이며 중복 성공3회가 아닙니다.
- [x] 목록/actual match arm·허용 집합/다른 domain과 비중복·mandatory remaining/channel/raw 전달·owner 보호/current/latest 무변경·diagnostics·실제 at-mention notification·typed argument 거절을 확인했습니다. diff Saved/Rejected/TabClosed·content 누락·directory 저장 오류·취소·닫힌 프로젝트·save true/false·중복 NotFound 및 종료 tracked0을 확인했습니다.
- [x] 후속14와 공유 최종 native `cargo clippy --lib --bin taide-native-app --tests -- -D warnings` exit0,12.75초입니다. 기존 Wry 의존성 경고17개는 별도이며 authored 경고 억제를 추가하지 않았습니다. authored7파일 edition2024 exactfmt exit0, 신규 source4 no-index whitespace는 빈 출력입니다.
- [x] PROCESS/HANDOFF/재개/QA5문서의 Bun Prettier 결과는 모두 unchanged입니다. tracked2문서 whitespace exit0, 신규3문서 no-index whitespace는 빈 출력/exit1(새 파일 diff 존재)입니다. 마지막 결과 표기 갱신은 QA2문서 단독 포맷으로 확인합니다.

## 미완료와 부채

- [ ] 남은131명령과 생산용 mandatory backend·AppInfo/Settings IDE→hooks→remote·assets/state/event bridge·NativeApplication startup/Exit·HostBridge 저장을 연결합니다. 실제 socket을 통한 전체 backend, 화면 pending 처리기, 전체 동시성/수명/메모리·보안·성능은 이 typed domain 검사와 별개입니다.
- [ ] 기존 plain Forbidden만 삼키는 분기와 localized root guard의 차이는 원본 코드/주석 불일치입니다. 현재 정확한 구현 parity를 유지했으며 원본 정책 변경을 합의한다면 root와 Tauri/native를 함께 변경하고 관련 검사를 추가합니다. 지금 원격 adapter에서만 다른 성공으로 바꾸지 않습니다.
- [ ] keybinding Tab RED·PTY remount 결정·전체213view/41action/Monaco21·TS 제거/Rust99%·픽셀/AX·배포/rollback은 기존 미완료입니다. 목표active·전체 M8 완료 전 commit/push 없음입니다.

## 원본과 공식 API

`src-tauri/src/remote_gateway.rs` IDE7 routing arm, `src-tauri/src/domain/ide/commands.rs`, root `ide_actions.rs`·`IdeStore`·`file_actions.rs`, native `bootstrap.rs`를 대조했습니다. [Tokio1.53.1 oneshot](https://docs.rs/tokio/1.53.1/tokio/sync/oneshot/index.html)·[serde_json1.0.151 from_value](https://docs.rs/serde_json/1.0.151/serde_json/fn.from_value.html)의 sender Drop과 typed input 경계를 확인했습니다. 설치 버전 문서를 사용했고 의존성을 업그레이드하지 않았습니다.
