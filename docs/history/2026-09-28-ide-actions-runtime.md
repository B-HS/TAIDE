# IDE 공개 action의 runtime 이전

## 대상 파일

- `crates/taide-runtime/src/ide_actions.rs`, `src/lib.rs`: 공개 action 7개
- `src-tauri/src/domain/ide/commands.rs`, `src-tauri/tests/ide_actions_runtime.rs`: 기존 IPC adapter와 메모리 검증 10개
- `docs/architecture.md`, `docs/PROCESS.md`: 현재 배치·잔여 수명주기

## 리포트

IDE의 selection/diagnostics/status·diff/save 응답·at-mention application 조립을 Tauri 미의존 runtime으로 이전했습니다. 기존 IDE protocol/store와 공유 IdeSaveFile port를 그대로 사용하며 실제 MCP 서버·lockfile·사용자 파일은 실행하지 않았습니다.

## 상세

1. 7개 action body는 State→참조와 save port의 동일 참조 치환 외 차이가 없습니다. Tauri 공개 시그니처 7개·공개 Rustdoc 14줄도 불변입니다. 기존 비IPC toggle/start/stop·bind·lockfile refresh·stale pending reconcile 등 함수 7개 body는 원본과 같습니다.
2. selection은 remote owner를 먼저 무시하고, 입력을 snapshot으로 투영→notification 생성→store 갱신→broadcast합니다. clear는 remote owner를 무시하며 desktop current만 비우고 latest와 기존 무발행 정책을 유지합니다. diagnostics의 ready 전환과 at-mention protocol도 그대로입니다.
3. diff는 pending을 먼저 소비하고 Saved content 존재를 검사한 다음 mutation guard 아래 같은 저장 port를 호출합니다. primitive Forbidden만 warning 뒤 Saved 응답으로 유지하고 다른 저장 오류는 반환합니다. Rejected/TabClosed는 저장하지 않고 None content를 응답하며 responder send 실패를 기존처럼 무시합니다.
4. 누락 content/저장 오류 때 이미 소비한 pending과 닫힌 responder 처리도 기존 계약입니다. 요청 Drop나 서버 종료의 새로운 회수 보장을 추가한 단위가 아닙니다.
5. 신규 테스트는 메모리 store/channel과 자기 UUID 미생성 경로만 사용합니다. 저장 port는 guard 보유·정확한 대상/content를 확인하고 메모리 flag만 변경합니다. 실제 파일 쓰기·keyring·네트워크·앱/GUI는 수행하지 않습니다. Cargo.toml/lock과 새 외부 의존은 없습니다.

## 검증 기록

runtime module 부재 E0432(exit 101)로 신규 integration의 RED를 확인한 뒤 이전했습니다. 이후 서로 다른 검사 23건이 통과했습니다.

| 명령                                                                                                                                                                                  | 실제 결과                                                     |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------- |
| `cargo test -p taide --test ide_actions_runtime --test taide_ide_service_extraction --test taide_ide_protocol_extraction --test rust_native_phase0_contract --test domain_boundaries` | 새 action 10·기존 service 1·protocol 2·IPC 7·도메인 3, exit 0 |
| `cargo clippy -p taide-runtime --all-targets -- -D warnings`                                                                                                                          | exit 0                                                        |
| `cargo clippy -p taide --lib --test ide_actions_runtime -- -D warnings`                                                                                                               | exit 0                                                        |
| `RUSTDOCFLAGS='-D warnings' cargo doc -p taide-runtime --no-deps`                                                                                                                     | exit 0                                                        |
| `cargo fmt --all -- --check`, `git diff --check`                                                                                                                                      | exit 0                                                        |

새 MD만 docs ignore를 해제한 Prettier로 검사합니다. unchanged service/store/protocol·서버/lockfile·TaskSupervisor의 이전 성공을 재사용하며 이번 새 통과 수에 합산하지 않습니다. AppEvent를 발행하는 IDE start/stop/MCP body와 platform event 검사 입력도 불변입니다. dependency 입력이 같아 직전 runtime normal graph의 Tauri 패키지 0개 결과를 재사용합니다.

공개 문서/시그니처·model/registry 생성 입력이 불변이므로 이전 Specta 생성 성공을 재사용합니다. 실제 bindings digest `f874269e742c9ac204c0b35085a41815743211b18ee33aabd48384f353a5975a`, manifest `343950f91338121d04edd36d5aa8d09491437cdb78e096fa0f519d3315a04bce`도 같습니다.

## 공식 근거와 남은 경계

[FutureExt::now_or_never](https://docs.rs/futures-util/0.3.33/futures_util/future/trait.FutureExt.html#method.now_or_never)의 첫 poll 결과를 사용해 메모리 save port 안의 mutation guard 보유를 검사합니다. Tokio 1.53.1 oneshot Sender 문서는 웹 접근 실패 뒤 설치된 공식 `src/sync/oneshot.rs`에서 확인했습니다. send는 동기·일회 소비이며 receiver가 이미 닫히면 Err이고, send 성공이 실제 수신을 보장하지 않습니다. 기존 무시 정책을 새 성공 보장으로 해석하지 않습니다.

프로젝트 이전 뒤 F127/S27/A13/P39에 IDE 7개(3 S + 4 P)를 반영하면 F134/S24/A13/P35입니다. 정적 entry 배치이며 실제 MCP 서버/클라이언트·저장·취소/shutdown 동등성 합격 수가 아닙니다.

비IPC IDE 서버/연결·lockfile·pending reconcile callback의 application 분리와 자원 admission/drain, 나머지 M6 entry/nested worker/root, M7 전체/실기·M8 native UI와 Phase 0 gate는 미완료입니다. M6 전체 완료 뒤 일반 push 조건을 유지합니다.
