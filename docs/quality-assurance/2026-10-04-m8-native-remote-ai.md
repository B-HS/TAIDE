# M8 원격 AI 실제 backend

## 대상과 결과

대상은 `native/taide-native-app/src/remote-ai.rs`, `remote-ai-tests.rs`, `lib.rs`입니다. 원본 허용 AI6명령을 실제 `ai_actions`와 `AppServices.secrets`의 필수 SecretStoreState, 동일 AiRequestStore에 연결했습니다. 선행168+6=174/177 domain adapter·남은 sync3이며 production App 전체 조립 완료가 아닙니다. root AI/runtime·manifest/lock·MSRV·제품 TS·보호 bundle·OS·TAIDE Git은 이번 변경에서 불변입니다.

## 원본 계약과 구현

- 원본 `src-tauri/src/remote_gateway.rs`의 AI6 arm과 `crates/taide-runtime/src/ai_actions.rs`, `ai_request_store.rs`, `crates/taide-ai/src/service.rs`, `providers/omlx.rs`, 실제 bundled prompt3을 확인했습니다. 별도 provider·prompt·HTTP 정책을 복제하지 않고 기존 API client와 모델 DTO를 그대로 호출합니다.
- 외부 `remote_gateway::with_policy`의 명령 admit·deep owner 강제를 유지합니다. 자격 증명 읽기는 기존 SecretStoreState 포트이며 테스트는 None만 반환하는 mock을 AppServices::new에 직접 넣습니다. KeyringSecretStore를 테스트에서 생성하거나 접근하지 않습니다. 원격 token set/clear와 sync connect는 outer policy에서 차단합니다.
- 생성3경로의 원본 byte gate, edit/commit의 provider/model Settings fallback, 요청 중복의 InvalidArgument, `{requestId,text}`와 취소/빈 응답의 null을 유지합니다. 요청은 실제 AiRequestToken RAII로 추적되므로 future Drop·cancel·shutdown은 원본 identity/idle 수명입니다. 새로운 blanket AppState shutdown gate나 TaskSupervisor worker를 추가하지 않습니다.
- 모델 조회와 status는 원본 query입니다. Ollama/Codex 미설정 credential과 OMLX base URL 미설정은 네트워크 전에 원본 오류입니다. 합성 OMLX에는 API key가 없어 Authorization header를 보내지 않았습니다.
- fixture HTTP는 기존 Axum0.8.9의 pinned 공식 소스 문서 `serve`, `with_graceful_shutdown`, `extract::State`, `body::to_bytes`를 확인한 뒤 사용했습니다. 127.0.0.1 임시 port·bounded8 capture·256KiB fixture body·명시 release/shutdown과 실제 server join/active0입니다. fixture의 body 상한은 제품 WebSocket byte cap 추가가 아닙니다.

## 단일 검증과 성공 재사용

- [x] `cargo test … --lib remote_ai::tests -- --nocapture`: compile10.53초/suite0.01초에서 catalog·입력2 PASS, socket3은 listener bind의 sandbox PermissionDenied로 HTTP 시작 전 중단됐습니다. 성공2는 재실행하지 않았습니다.
- [x] 실패 socket3만 `cargo test … --lib remote_ai::tests::실제_ -- --nocapture`로 권한 승격 실행: 준비0.21초/suite0.01초, 3 PASS. 코드 수정이나 같은 성공 검사 반복은 없었습니다.
- [x] native `cargo clippy … --lib --bin taide-native-app --tests -- -D warnings`: exit0·13.25초입니다. 기존 Wry dependency17 warnings와 authored strict 결과를 구분합니다. 검사기 suppression은 없습니다.
- [x] authored3 exact rustfmt와 tracked `git diff --check`: exit0입니다. 신규 파일 no-index diff의 빈 출력/exit1은 신규 diff의 정상 exit이며 공백 오류가 아닙니다.

Cargo 환경은 `CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo`, native manifest, `--locked --offline --target-dir experiments/native-shell-spike/target`입니다. full suite·실기 GUI·외부 AI·사용자 Keychain은 실행하지 않았습니다.

## 실제 고유5 검사

1. catalog6/실제 dispatch arm/허용 목록·선행 domain 비중복/typed args/remaining JSON·raw·status DTO·원본 query shutdown 허용·credential 부재·외부 접근 전 오류·token mutation 거절입니다.
2. prefix32KiB/suffix16KiB/selection100KiB/instruction4KiB/diff64KiB/recent8KiB를 각각 UTF-8 byte 초과로 거절하고 credential 조회·요청 입장 전에 검증했습니다. prefix 정확한 상한은 통과 후 실제 missing credential 오류·Settings fallback 오류·합성 store Internal을 보존했습니다.
3. 실제 loopback GET models1·생성 complete/edit/commit 각1로 model/displayName wire·max_tokens256/4096·stream false·bundled prompt 렌더·Settings 기본 provider/model·공백을 포함한 정상 응답 text 보존을 확인했습니다.
4. 실제 대기 HTTP complete/edit/commit의 owner 강제·동일 remote ID 중복·main 동일 ID 비간섭·cancel null·future Drop 뒤 같은 key 재입장·shutdown의 입장 폐쇄/identity Drop까지 idle 대기·실제 server join/active0을 확인했습니다.
5. 실제 provider의 whitespace-only null·edit length truncation Internal·commit HTTP401 localized unauthorized를 확인했습니다. provider 전체 HTTP 상태표/FIM family 검사 반복은 하지 않았습니다.

## 미완료와 실행 시점

- [ ] 전체 dispatch chain·Settings/AppServices 생산용 자격 증명 서비스 선택·App startup/Exit의 AiRequestStore shutdown/idle·HTTP/WS 연결 수명 조립은 production assembly 시 확인합니다. fixture 성공은 이를 대체하지 않습니다.
- [ ] 실제 Ollama/Codex/OMLX 설정·자격 증명·외부 AI·FIM 모델·사용자 prompt override 실기는 제품 실기 단계입니다. 이번 어댑터는 실제 root provider/prompt를 재사용했으며 live credential 검증을 주장하지 않습니다.
- [ ] 전체 M8 N1~N8은0/8입니다. 기존 keybinding Tab RED/PTY remount 선택과 전체 view/cutover/Rust99%/성능/AX/보안/배포/rollback gate는 남습니다. 전체 완료 전 commit/push하지 않습니다.
