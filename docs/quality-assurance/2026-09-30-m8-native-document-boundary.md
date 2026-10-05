# M8 native 문서 파일 열기 선행 경계

## 대상과 판정

대상은 `crates/taide-runtime/src/native_file_actions.rs`, 기존 `file_actions::run_guarded_file_worker`, `tests/native_file.rs`입니다. `open_document_file`은 native editor가 사용할 파일 payload를 준비하는 opt-in Rust API입니다. 기존 Tauri `file_open` caller·저장 형식·UI·제품 의존성과 lockfile은 이번 변경에서 바꾸지 않았습니다. GUI·rope 후보를 제품에 선정하지 않았으며 canonical DocumentStore·ViewStore를 구현한 것으로 계산하지 않습니다.

PROCESS N3의 canonical 문서 열기에 필요한 권한·경로·작업 수명 경계를 먼저 연결했습니다. N1의 후보 선정과 실기 gate는 별도로 미완료입니다. 사용자 실기용 앱·입력기·VoiceOver 설정을 조작하지 않았습니다.

## 구현과 근거

1. 기존 root guard에서 open project의 가장 구체적인 root 또는 CLI 승인 단일 파일을 해석합니다. 결과의 `PathBuf`는 `canonical_path`로 유지하며, 요청 원문은 `display_path`로 분리합니다. 심볼릭 링크 별칭은 canonical 경로를 공유하지만 표시 경로를 덮어쓰지 않습니다.
2. 기존 owned mutation guard·operation lease를 읽기 worker에 함께 이전합니다. 프로젝트 닫기와 같은 잠금을 공유하며, 요청 waiter가 취소돼도 시작한 worker가 끝날 때까지 잠금·종료 추적을 유지합니다. guard를 얻고 worker를 시작한 뒤 현재 project/CLI 권한을 검사합니다. 거절된 경로는 language overlay loader를 호출하지 않습니다.
3. 기존 file service가 내용을 읽고 UTF-8 손실·read-only·크기 tier·EditorConfig·language 정책을 판정합니다. 로더는 권한 검사 뒤 blocking worker에서 호출하는 Send closure입니다. 내용을 다른 buffer로 복제하거나 새 편집기 backend를 추가하지 않았습니다.

공식 근거: [Rust canonicalize](https://doc.rust-lang.org/std/fs/fn.canonicalize.html)는 절대 경로와 심볼릭 링크 해석을 정의하며 Windows에서는 extended-length 경로를 반환할 수 있습니다. [Tokio OwnedMutexGuard](https://docs.rs/tokio/1.53.1/tokio/sync/struct.OwnedMutexGuard.html)는 Arc로 잠금을 소유하고 Drop 때 해제합니다. 기존 runtime의 owned worker 경계를 재사용하며 새로운 task supervisor를 만들지 않습니다.

`canonical_path`는 파일 URI나 영구 DocumentId가 아닙니다. 기존 `workspace_folder_uri`는 Monaco workspace 문자열과 일치시키기 위한 drive fixup을 포함하므로 이 단계에서 문서 URI 변환기로 재사용하거나 새 변환기를 임의로 만들지 않았습니다. 기존 file DTO의 path는 file service의 정책대로 유지하고, native 문서 식별에는 별도 PathBuf를 제공합니다.

## 실행 증거

```sh
CARGO_TARGET_DIR=experiments/terminal-core-spike/target cargo test -p taide-runtime --test native_file --locked --offline
CARGO_TARGET_DIR=experiments/terminal-core-spike/target cargo clippy -p taide-runtime --lib --test native_file --locked --offline -- -D warnings
rustfmt --edition 2021 --check crates/taide-runtime/src/native_file_actions.rs crates/taide-runtime/tests/native_file.rs crates/taide-runtime/src/file_actions.rs crates/taide-runtime/src/lib.rs
git diff --check
```

신규 module 부재 E0432 red를 먼저 확인하고 구현 후 테스트 2건이 0.01초에 통과했습니다. 같은 성공 테스트는 반복하지 않습니다. 최초 기본 target의 의존성 빌드는 중단(exit 130)하고 기존 공유 cache target에서 red와 검증을 수행했으며, 중단된 빌드는 성공 증거에 포함하지 않습니다. 설치된 rustc 1.98.1에서 검사했고 제품 MSRV 1.89 선언은 유지합니다. 실제 1.89 실행 검사는 미완료입니다.

- [x] 한글·supplementary·NFD·CRLF 내용과 공백·percent·hash 파일명 보존, canonical/표시 경로 분리, Unix 내부 symlink 별칭 일치와 외부 symlink 거절, 승인 전 CLI 경로 거절·승인 후 owner 없음, lossy UTF-8 read-only, 50MiB sparse fixture의 Refused·내용 미할당, project 제거 후 권한 거절, supervisor 종료 후 거절과 작업 추적 0을 확인했습니다. 합성 fixture만 사용하며 data 저장 디렉터리를 만들지 않습니다.
- [x] 실제 blocking worker의 overlay loader를 event로 정지한 뒤 요청을 취소했습니다. 프로젝트 mutation lock과 root shutdown future를 직접 poll해 Pending을 확인하고, worker release 뒤 종료 완료·잠금 획득·추적 0을 확인했습니다. sleep이나 반복 계측으로 읽기 시간을 추정하지 않습니다. 실제 project close 전체 adapter가 아니라 공유 닫기 잠금의 유지 증거입니다.
- [x] 대상 runtime lib·신규 test의 strict clippy가 exit 0(0.50초), 대상 Rust format·diff 검사도 exit 0입니다. 기존 LSP·terminal·file worker의 성공 검사는 변경 동작이 없으므로 재사용하고 다시 실행하지 않았습니다.

## 남은 경계와 검증 부채

- 파일 payload 반환 뒤 DocumentStore에 commit하는 사이의 root/version 재검증은 native composition owner에서 연결해야 합니다. 이번 잠금은 worker 완료까지이며, 반환 후 영구 권한을 부여하지 않습니다. 문서 incarnation·여러 view의 단일 buffer·LSP URI·revision transaction·dirty/save/hot-exit의 단일 소유권은 아직 없습니다.
- 같은 inode의 hard link나 파일 교체는 canonical 문자열만으로 구별되지 않습니다. OS에서 외부 프로세스가 검사와 read 사이에 symlink/ancestor를 교체하는 TOCTOU와 외부 편집 중 일관된 disk snapshot은 기존 root guard/file service가 제공하는 수준을 그대로 유지합니다. file-handle 기반 강화가 필요한 보안 gate에서 재현 fixture로 검증해야 하며 이번 symlink 거절 검사를 완전한 race 방어로 해석하지 않습니다.
- 기존 mutation guard를 읽기에 유지해 project close와 직렬화합니다. native 실제 caller가 생기면 대형 파일·동시 열기와 close latency를 측정하고 정책을 판정해야 합니다. 테스트의 0.01초는 제품 성능 기준선이 아닙니다.
- 비 UTF-8 OS 경로·Windows extended-length/UNC·case-sensitive volume·hard link의 URI/identity 정책, 실제 GUI 선택·저장·외부 변경과 closed/reopened project incarnation은 native DocumentStore 연결 시 검증합니다. 임의 URI encoding이나 GUI 의존성 선정으로 우회하지 않습니다.
