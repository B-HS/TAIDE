# Agent 조회·marker·대기열 정책의 runtime 이전

## 대상 파일

- `crates/taide-runtime/src/agent_actions.rs`, `src/lib.rs`
- `src-tauri/src/domain/agent/commands.rs`, `src-tauri/tests/agent_actions_runtime.rs`
- `docs/architecture.md`, `docs/PROCESS.md`

## 리포트

공개 action 3개와 공용 state/detected-agent·marker cleanup 정책을 runtime으로 이전했습니다. 프로젝트 gate 뒤에만 AppHandle PID와 OS probe callback을 실행하며 실제 probe/CLI/hook/server는 기존 Tauri adapter에 남습니다.

## 상세

1. list는 프로젝트 검증→lazy PID→probe await→기존 세션 신호/eligible HTTP hook override→응답 순서입니다. PID/probe를 선실행하지 않으며 mutation guard를 새로 잡거나 poll의 diff cache를 변경하지 않습니다.
2. release-marker는 기존 guard 안에서 경로 검증→remove_file→추적 해제 순서입니다. 검증 오류는 추적을 유지하고, 검증 후 삭제 오류는 원래대로 추적을 해제하고 오류를 반환합니다. missing은 성공입니다. cleanup은 전체 추적을 선소비하고 유효 경로만 삭제하며 오류는 원래대로 무시합니다.
3. pending-open은 순서를 유지해 한 번 꺼냅니다. 실제 외부 열기 이벤트와 queue 조립은 변경하지 않았습니다.
4. 공용 helper 3개와 release/pending body는 원본과 같습니다. list body는 참조/lazy callback 치환 외 정책이 같으며 공개 command 시그니처 11개 cfg 변형(고유 9개)과 문서 입력은 불변입니다. 미이전 함수 구간 42개를 원본과 대조해 body 불일치 0을 확인했습니다. resolve_state의 기존 영어 문서는 runtime에 보존했습니다.
5. 새 테스트는 메모리 store·미생성 UUID 프로젝트 경로와 직접 만든 UUID marker만 사용합니다. 생성한 marker만 정리하며 사용자 PID/파일/home·CLI·hook·서버·네트워크·앱을 실행하지 않았습니다. 의존 입력도 불변입니다.

## 검증 기록

새 모듈 부재 E0432(exit 101)로 RED를 확인했습니다. action 9·agent 추출 1·IPC 7·도메인 3, 서로 다른 검사 20건이 통과했습니다. 이전 후 사용처가 없어진 AgentActivity import를 제거했습니다. 테스트 함수명의 PID/HTTP 대문자로 strict clippy가 실패해 이름을 snake_case로 고쳤고, 한국어 표기를 정리한 해당 action target 9건만 다시 실행해 exit 0을 확인했습니다. 같은 검사를 두 번 합산하지 않습니다.

| 명령                                                                                                                                                 | 실제 결과                               |
| ---------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------- |
| `cargo test -p taide --test agent_actions_runtime --test taide_agent_service_extraction --test rust_native_phase0_contract --test domain_boundaries` | action 9·agent 1·IPC 7·도메인 3, exit 0 |
| `cargo test -p taide --test agent_actions_runtime`                                                                                                   | 이름 수정 후 9건, exit 0                |
| `cargo clippy -p taide-runtime --all-targets -- -D warnings`                                                                                         | exit 0                                  |
| `cargo clippy -p taide --lib --test agent_actions_runtime -- -D warnings`                                                                            | 이름 수정 후 exit 0                     |
| `RUSTDOCFLAGS='-D warnings' cargo doc -p taide-runtime --no-deps`                                                                                    | exit 0                                  |
| `cargo fmt --all -- --check`, `git diff --check`                                                                                                     | exit 0                                  |

공개 command 생성 입력이 불변이므로 이전 Specta 생성은 재사용합니다. bindings digest `f874269e742c9ac204c0b35085a41815743211b18ee33aabd48384f353a5975a`, manifest `343950f91338121d04edd36d5aa8d09491437cdb78e096fa0f519d3315a04bce`도 불변입니다. dependency graph 입력이 이전 단위와 같아 runtime의 Tauri 0개 결과를 재사용하며 실행 수에 합산하지 않습니다.

## 공식 근거와 남은 경계

[Future](https://doc.rust-lang.org/std/future/trait.Future.html)의 Output 계약과 기존 app_actions의 generic callback 패턴을 확인했습니다. [remove_file](https://doc.rust-lang.org/std/fs/fn.remove_file.html)의 directory 오류를 직접 만든 빈 UUID marker로 재현하며 오류의 기존 추적 정책을 바꾸지 않았습니다.

F146/S20/A13/P27에서 3개(1 S + 2 P)를 반영하면 F149/S19/A13/P25입니다. 정적 entry 배치 수이며 전체 shutdown/실기 동등성 합격 수가 아닙니다. hook status/install/uninstall·user home/CLI·HTTP 서버·실제 OS probe와 이미 시작한 worker, 나머지 application/root admission/drain·M6/M7/M8는 미완료입니다. M6 전체 완료 뒤 일반 push 조건을 유지합니다.
