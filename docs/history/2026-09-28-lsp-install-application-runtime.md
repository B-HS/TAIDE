# LSP 설치 application의 runtime 이전

## 대상 파일

- `crates/taide-runtime/src/lsp_actions.rs`
- `src-tauri/src/domain/lsp/commands.rs`, `tests/lsp_actions_runtime.rs`, `tests/taide_lsp_install_store_extraction.rs`
- `docs/architecture.md`, `docs/PROCESS.md`

## 리포트

LSP 설치의 shutdown·server 해석·중복 입장·전략 선택·오류 조립을 runtime으로 이전했습니다. Tauri는 같은 State와 EventSink adapter를 전달하며 기존 download/toolchain runtime 구현과 InstallStore guard/lease를 사용합니다.

## 상세

1. app shutdown이면 spec 조회 전에 store를 닫고 취소 오류를 반환합니다. 그렇지 않으면 bundled spec을 먼저 조회하고 store 입장을 시도합니다. stopped store에서도 unknown server가 먼저 InvalidArgument인 기존 순서를 유지합니다.
2. 중복 요청은 기존 실행 lease를 취소하지 않고 같은 localized 오류를 반환합니다. SDK-only는 같은 오류 뒤 guard Drop으로 자기 슬롯을 반납합니다. download/toolchain 분기는 동일 spec·lease·TaskSupervisor·EventSink를 기존 구현에 전달합니다.
3. 전체 공개 command 11개 signature/문서와 spawn/stop/restart/OS 탐지 본문은 byte 동일합니다. 원격 정책·등록 상태·의존·설치 구현을 변경하지 않았습니다. 사용하지 않게 된 production import만 제거하고 기존 unit test의 전략 import는 test scope로 옮겼습니다.
4. 자기 UUID AppState와 메모리 store/event만 사용했습니다. 실제 다운로드·toolchain 설치·OS 탐지·사용자 LSP/프로젝트·앱·외부 네트워크는 실행하지 않았으며 정상 설치 실기 합격을 주장하지 않습니다.

## 검증 기록

새 action 부재 E0425(exit 101)로 RED를 확인했습니다. 최종 action 13·설치 추출/배선 2로 이번 단위의 서로 다른 검사 15건이 통과했습니다.

| 명령 | 실제 결과 |
| --- | --- |
| `cargo test -p taide --test lsp_actions_runtime --test taide_lsp_install_store_extraction` | 최종 13·2건, exit 0 |
| `cargo clippy -p taide-runtime --all-targets -- -D warnings` | exit 0 |
| `cargo clippy -p taide --lib --test lsp_actions_runtime --test taide_lsp_install_store_extraction -- -D warnings` | exit 0 |
| `RUSTDOCFLAGS='-D warnings' cargo doc -p taide-runtime --no-deps` | exit 0 |
| `cargo fmt --all -- --check`, `git diff --check` | exit 0 |

공개 생성 입력/의존 불변으로 직전 실제 binding 생성/IPC 성공과 runtime normal graph 543줄/Tauri 0개 결과를 재사용합니다. bindings digest `f874269e742c9ac204c0b35085a41815743211b18ee33aabd48384f353a5975a`·manifest `343950f91338121d04edd36d5aa8d09491437cdb78e096fa0f519d3315a04bce`는 같은 입력입니다. 재사용한 검사를 이번 실행 수에 더하지 않습니다.

## 남은 경계

설치 1개 P→F로 정적 entry 배치는 F177/S0/A13/P16입니다. LSP spawn/stop/restart·다른 application/nested worker/root·Git guard·locale 보안 선택·직접 Exit/OS 오류·다운로드/toolchain 실기·M6/M7/M8는 미완료입니다. M6 전체 완료 뒤 일반 push 조건을 유지합니다.
