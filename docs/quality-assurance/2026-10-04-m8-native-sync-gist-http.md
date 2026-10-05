# M8 공유 Gist HTTP 전송 구현

## 대상과 결과

대상은 `crates/taide-sync/src/github.rs`, `Cargo.toml`, `src/lib.rs`, `crates/taide-runtime/src/sync_gist_http.rs`, runtime lib, `src-tauri/src/domain/sync/github.rs`, root/native Cargo.lock입니다. 기존 Tauri HTTP 구현을 공유 sync crate로 이동했고 runtime의 local SyncGistPort trait adapter와 기존 Tauri import 경로 re-export를 연결했습니다.

## 원본 계약 보존

- 원본 source와 모든 외부 caller를 대조했습니다. GistClient의 외부 caller는 없고 기존 commands는 같은 SyncGistHttpPort::new 경로를 유지합니다. native는 이제 Tauri 없이 runtime 공유 타입을 필수 Gist factory로 전달할 수 있습니다. 아직 NativeApplication 생산용 factory 호출을 연결한 것은 아닙니다.
- GitHub API/version/UA/header/auth·페이지100/preferred ID/최신 updated+ID 비교·비공개 지정파일 payload·POST/PATCH/GET·missing-content 오류·provider redaction을 유지합니다.
- 기존 API singleton의 connection10초/whole request60초·lazy factory·reqwest 공유 pool을 그대로 씁니다. credential store나 새로운 client 정책/외부 base override는 추가하지 않았습니다.
- 기존 owned 포트는 GistHttpClient inherent4메서드로 옮기고 runtime local trait impl4는 명시적인 fully qualified forwarding입니다. sync가 runtime을 import하지 않아 dependency cycle이 없습니다.
- 원본 body를 이동본과 역변환/공백 정규화 대조한 결과 동등입니다. 의도한 차이는 소유 import/타입 이름·public inherent 메서드와 test abort 후 cancelled join 확인입니다.
- 기존 reqwest0.12.28/serde/taide-infra 직접 production edge3, 기존 Axum0.8.9/Tokio test edge2만 추가합니다. root/native lock의 sync 항목만 관련 edge를 추가했으며 새 registry package/version/checksum/MSRV는 없습니다. 기존 다른 M8 lock 변경은 보존합니다.

## 공식 근거

[Rust Reference trait coherence](https://doc.rust-lang.org/reference/items/implementations.html#trait-implementation-coherence)는 local trait의 외부 타입 구현을 허용합니다. [Axum0.8.9 serve](https://docs.rs/axum/0.8.9/axum/fn.serve.html)의 http1/tokio 요구를 기존 dev edge에 사용합니다. reqwest docs.rs 조회는 실패했으므로 실제 pinned registry reqwest-0.12.28/src/async_impl/client.rs의 공식 Client 문서/구현을 확인해 singleton clone/pool을 보존했습니다.

## 최소 검증

- [x] `cargo test -p taide-sync --lib github::tests --locked --offline …`: compile6.32초/suite0.00초, payload 순수2 PASS·loopback2는 bind에서 sandbox PermissionDenied입니다. 제품 실패나 실제 HTTP 응답 실패가 아닙니다.
- [x] 실패한 discovery pagination/preferred exact1만 승격 실행해 compile0.11초/suite0.00초 PASS입니다. 실패한 empty/auth exact1만 승격 실행해 compile0.10초/suite0.00초 PASS입니다. 성공한 payload2는 재사용했으며 총 고유4 PASS입니다.
- [x] 공유 `cargo clippy -p taide-sync -p taide-runtime --lib --tests … -- -D warnings`: exit0·11.87초입니다.
- [x] 변경된 dependency 연결을 포함한 native lib/bin/tests strict: exit0·16.99초입니다. 기존 Wry dependency17 warnings와 authored strict를 구분하며 suppression은 없습니다.
- [x] 기존 Tauri caller `cargo check -p taide --lib --locked --offline …`: exit0·1분07초입니다. 기존 import/factory4가 공유 타입으로 컴파일되며 앱 실행이나 bundle 교체는 하지 않았습니다.
- [x] authored5 exact edition2021 rustfmt·tracked diff check exit0입니다. 신규2 no-index check는 빈 출력/exit1 정상 신규 diff입니다.

모든 Cargo는 CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo, --locked --offline --target-dir experiments/native-shell-spike/target입니다. 합성 localhost만 실행했으며 실제 GitHub/사용자 token/Keychain/앱/OS 설정/보호 bundle/TAIDE Git은 접근하거나 변경하지 않았습니다.

## 미완료

- [ ] 생산용 필수 Gist factory와 다른 OS/capabilities/AppInfo/Settings reconcile·assets/state/events·socket_action·NativeApplication startup/Exit를 기존 pending에서 연결합니다. 공유 타입 제공은 full App 조립 완료가 아닙니다.
- [ ] create/update/fetch 외부 GitHub 실제 계정 전송은 이번 source 이동 범위를 넘어 실행하지 않았습니다. 기존 sync runtime 합성6 PASS를 재사용하며 실제 credentials/provider 통합은 제품 실기 gate에 남습니다.
- [ ] N1~N8 0/8·keybinding RED/PTY remount·view/cutover/Rust99%·IME/AX/perf/security/package/rollback은 미완료입니다. 전체 M8 완료 전 commit/push하지 않습니다.
