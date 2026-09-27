# AgentHooksStore 공유 상태 분리

상태: M6의 이 변경 단위는 자동 검증 완료. M6 전체와 M7·M8은 미완료입니다.

## 대상과 변경

`crates/taide-agent/src/store.rs`가 hook 서버 정보·Tokio accept 핸들·프로젝트 활동 override를 소유합니다. 내부 Arc<Mutex>를 공유하는 clone을 `crates/taide-runtime/src/app_services.rs`가 생성하며 `src-tauri/src/lib.rs`는 같은 인스턴스를 기존 Tauri State에 등록합니다. `src-tauri/src/domain/agent/commands.rs`의 공개 경로는 재수출로 유지합니다.

첫 서버 유지와 중복 핸들 취소, 900초 override 만료, 종료 시 서버 정보·override 정리를 유지했습니다. 신규 패키지나 버전 변경 없이 기존 parking_lot·Tokio와 로컬 agent crate 의존을 조립했습니다. agent/runtime의 normal dependency graph에 Tauri가 없습니다.

## 검증

- `cargo test -p taide-agent --lib store:: --quiet`: 만료 회귀 1건 통과.
- `cargo test -j 1 -p taide --test app_services_runtime --test task_supervisor --test rust_native_phase0_contract --quiet`: 공유/배선 2건·감독 22건·IPC 계약 7건 통과.
- `cargo test -p taide --lib domain::agent:: --quiet`: 기존 agent 도메인 22건 통과.
- `cargo clippy -p taide-agent -p taide-runtime -p taide --all-targets -- -D warnings`와 두 crate의 `RUSTDOCFLAGS='-D warnings' cargo doc --no-deps --quiet`: exit 0.
- `cargo fmt --all -- --check`·`git diff --check`: exit 0. bindings SHA-256은 `174a2d617372f4782b8397351660f503194dc567ece4feb82c2a79199c09977a`로 불변입니다.

최초 통합 검증은 proc-macro 동적 라이브러리 로딩의 `dyld → fcntl` 대기가 관찰되어 해당 rustc를 종료했습니다(exit 101, SIGTERM). clippy와 대기 중 agent 검사·rustdoc은 완료 전 종료했습니다(exit 143). `/private/tmp/taide-m6-rustc-33550.sample.txt`에 관찰 스택을 수집했고, `libtauri_specta_macros-68ea0e495361fba7.dylib`의 읽기 전용 codesign 검사는 정상으로 끝났습니다. 산출물·보안 설정을 변경하지 않았으며 직렬 재검증이 통과했습니다. 대기의 OS 근본 원인은 확정하지 않았습니다.

이 변경 뒤 전체 Rust workspace·TypeScript typecheck·실제 hook 서버와 앱 종료 GUI는 미검증입니다. 생성 IPC DTO·command/event 등록·기존 TS UI는 변경하지 않았습니다. 원격 push는 기존 승인 거절로 실행하지 않습니다.
