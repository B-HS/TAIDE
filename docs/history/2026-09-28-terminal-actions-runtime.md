# 터미널 조회·구독·경로 정책의 runtime 이전

## 대상 파일

- `crates/taide-runtime/src/terminal_actions.rs`
- `src-tauri/src/domain/terminal/commands.rs`, `src-tauri/tests/terminal_actions_runtime.rs`
- `docs/architecture.md`, `docs/PROCESS.md`

## 리포트

spawn/write를 제외한 공개 action 10개를 기존 runtime 모듈로 이전했습니다. resize/pause·kill/attach/detach·세션/shell 조회·경로 검증·기본 옵션 정책을 공유하고 raw Channel sink만 Tauri adapter에서 조립합니다.

## 상세

1. kill/attach/detach는 기존 mutation guard를 유지합니다. attach는 guard 뒤에 lazy sink factory를 실행하므로 대기 중 Channel adapter를 선생성하지 않습니다. missing session의 오류와 detach의 멱등성을 유지합니다.
2. 경로는 기존 terminal service와 root guard에 위임합니다. 열린 root 밖/없는 link는 동일한 NotFound로 거절하고 후보 개수 제한과 순서를 유지합니다. default options의 None cwd는 원래대로 root를 그대로 반환하며 요청 cwd의 파일/디렉터리 판정 정책을 변경하지 않습니다. shell override·80열/24행·scrollback None도 같습니다.
3. 명시 sink 치환과 포맷 외 이전 body 불일치 0, 공개 command 12개 시그니처·Rustdoc 68줄 불변, spawn/write body 불변을 원본과 대조했습니다. 의존성·observer/callback·생성 bindings 입력은 변경하지 않았습니다.
4. 테스트는 자기 UUID 프로젝트/파일과 메모리 sink를 사용합니다. 자기 전용 `/bin/sh` PTY는 ENV/BASH_ENV를 비워 사용자 profile을 실행하지 않으며 attach 재생·detach·resize·pause·kill과 실제 worker 완료를 확인했습니다. 출력 검증은 직접 주입한 session buffer를 사용하며 shell 출력의 end-to-end 동등성을 주장하지 않습니다. 사용자 프로세스/파일·앱·네트워크는 사용하지 않았고 shell_profiles의 시스템 탐색도 새 테스트에서 실행하지 않았습니다.

## 검증 기록

runtime action 부재 E0425(exit 101)로 RED를 확인했습니다. 최종 action 8·terminal store 3·service 추출 1·출력 추출 6·IPC 7·도메인 3, 서로 다른 검사 28건이 통과했습니다. 최초 batch의 action 7건 뒤 자기 PTY fixture를 추가해 해당 target 8건과 관련 clippy만 재검사했습니다. 중복 실행은 합산하지 않습니다.

| 명령 | 실제 결과 |
| --- | --- |
| `cargo test -p taide --test terminal_actions_runtime --test taide_terminal_store_extraction --test taide_terminal_service_extraction --test taide_terminal_session_output_extraction --test rust_native_phase0_contract --test domain_boundaries` | 최초 action 7·관련 20건, exit 0 |
| `cargo test -p taide --test terminal_actions_runtime` | 최종 action 8건, exit 0 |
| `cargo clippy -p taide-runtime --all-targets -- -D warnings` | exit 0 |
| `cargo clippy -p taide --lib --test terminal_actions_runtime -- -D warnings` | 최종 fixture 포함 exit 0 |
| `RUSTDOCFLAGS='-D warnings' cargo doc -p taide-runtime --no-deps` | exit 0 |
| `cargo fmt --all -- --check`, `git diff --check` | exit 0 |

공개 생성 입력 불변이므로 이전 Specta 생성 성공을 재사용합니다. bindings digest `f874269e742c9ac204c0b35085a41815743211b18ee33aabd48384f353a5975a`, manifest `343950f91338121d04edd36d5aa8d09491437cdb78e096fa0f519d3315a04bce`도 불변입니다. dependency graph 입력은 같으므로 runtime normal graph의 Tauri 0개 결과도 재사용하며 실행 수에 합산하지 않습니다.

## 남은 경계

F149/S19/A13/P25에서 10개(6 S + 4 P)를 반영하면 F159/S13/A13/P21입니다. 정적 entry 배치 수이며 전체 자원 회수·실기 동등성 합격 수가 아닙니다. spawn/write 전체 action·observer/callback·blocking write owner와 나머지 application/root·M6/M7/M8는 미완료입니다. M6 전체 완료 뒤 일반 push 조건을 유지합니다.
