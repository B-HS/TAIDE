# LSP send와 재초기화 action의 runtime 이전

## 대상 파일

- `crates/taide-runtime/src/lsp_actions.rs`
- `src-tauri/src/domain/lsp/commands.rs`, `tests/lsp_actions_runtime.rs`
- `docs/architecture.md`, `docs/PROCESS.md`

## 리포트

LSP send·재초기화 확인/실패 기록 3개를 기존 runtime 모듈로 이전했습니다. IPC counter는 Tauri에 남기고 재초기화 status event는 같은 EventSink adapter로 전달합니다.

## 상세

1. 전체 공개 command 11개의 signature/문서가 byte 동일하며 spawn/stop/restart/install/OS 탐지 본문도 byte 동일합니다. 원격 호출·의존·상태 등록을 변경하지 않았습니다.
2. send는 같은 session 조회→proc clone→기존 write await를 실행합니다. 기존 missing/not-ready 오류와 proc writer 내부 직렬화는 유지합니다. Tauri는 매 IPC 호출의 LspSend counter를 이전과 같이 증가시킵니다.
3. 재초기화는 같은 lifecycle의 현재 generation/Crashed 조건을 적용하고 동일 status·last_error·generation·session_id를 발행합니다. 기존 한국어 실패 문구를 그대로 옮겼습니다. 중복 confirm과 stale generation/Running 상태의 실패 기록은 전이·이벤트 없이 성공을 반환하는 기존 정책입니다.
4. 메모리 store와 EventSink만 사용했으며 자기 UUID root fixture는 이전 조회 검사에서 재사용했습니다. 실제 LSP 프로세스의 send·메시지 framing·write 중 취소·사용자 앱은 실행하지 않았습니다. 이번 이전은 그 실기나 전체 종료 보장을 증명하지 않습니다.

## 검증 기록

새 action 부재 E0425(exit 101)로 RED를 확인했습니다. send 오류 기대값은 실제 AppError Display 원천을 확인해 실행 전에 `operation failed:`로 맞췄습니다. patch context 순서 오류는 파일 무변경을 확인한 뒤 올바른 hunk 순서로 적용했습니다.

| 명령 | 실제 결과 |
| --- | --- |
| `cargo test -p taide --test lsp_actions_runtime --test taide_lsp_session_lifecycle_extraction` | 최종 action 9·lifecycle 4건, exit 0 |
| `cargo clippy -p taide-runtime --all-targets -- -D warnings` | exit 0 |
| `cargo clippy -p taide --lib --test lsp_actions_runtime -- -D warnings` | exit 0 |
| `RUSTDOCFLAGS='-D warnings' cargo doc -p taide-runtime --no-deps` | exit 0 |
| `cargo fmt --all -- --check`, `git diff --check` | exit 0 |

이번 단위에서 실제 실행한 서로 다른 검사는 13건입니다. 공개 생성 입력과 의존이 불변이므로 직전 설치 단위의 실제 Specta 생성/IPC 성공과 runtime normal graph 543줄/Tauri 0개를 재사용합니다. 재사용 결과를 별도 실행 수에 더하지 않습니다. binding/manifest digest는 각각 `f874269e742c9ac204c0b35085a41815743211b18ee33aabd48384f353a5975a`·`343950f91338121d04edd36d5aa8d09491437cdb78e096fa0f519d3315a04bce`입니다.

## 남은 경계

정적 entry 배치는 3개 P→F로 F176/S0/A13/P17입니다. 실제 LSP spawn/stop/restart/install·다른 application/nested worker/root·Git guard·locale 보안 선택·직접 Exit/OS 오류·M6/M7/M8는 미완료입니다. M6 전체 완료 뒤 일반 push 조건을 유지합니다.
