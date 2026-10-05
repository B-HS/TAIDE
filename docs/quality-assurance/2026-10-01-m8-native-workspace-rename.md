# M8 native WorkspaceEdit 이름 변경

## 대상·기준

- 대상: `native/taide-native-app/src/{workspace_rename,workspace_activity,lsp_workspace_worker,lsp,application}.rs`, `tests/lsp-workspace-worker.rs`, `native/taide-native-editor/src/store.rs`, `tests/file-retarget.rs`입니다.
- 원본: `src/shared/lib/lsp/workspace-edit-applier.ts`, `src/entities/file/workspace-resource-operations.ts`, `src/entities/layout/tab-path-change.ts`, `src/entities/editor/model-registry.ts` 및 기존 Rust `rename_entry`·`apply_tab_path_change`입니다.
- 공식 계약: [LSP 3.17 metamodel](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/metaModel/metaModel.json)의 RenameFile·WorkspaceEdit를 읽었습니다. 배열 순서대로 적용하고 실패 뒤 후속 작업은 중단합니다. 앞서 성공한 작업을 rollback한다고 주장하지 않습니다.
- overwrite가 true여도 기존 제품의 `rename_entry`는 서로 다른 목적지 파일을 덮어쓰지 않습니다. native도 실제 TS→Rust 동작을 유지하며 LSP 규격의 overwrite를 이유로 새로운 파괴적 동작을 추가하지 않았습니다.

## 구현

- [x] 엄격한 local file URI와 entry-scoped root guard를 재사용합니다. 마지막 symlink 자체의 이름 변경은 content 대상 이동과 구분합니다. source를 포함하는 모든 열린 프로젝트에 destination도 속해야 하며 server roots가 있으면 양쪽 경로를 검사합니다.
- [x] source buffer를 GUI에서 cheap Rope snapshot으로 얻고 실제 기존 파일 rename·모든 해당 프로젝트의 main/auxiliary File 탭·closed history를 이동합니다. 기존 문서 id·본문·baseline·dirty·공유 view/selection/scroll을 유지하며 새 경로 metadata와 language/config를 다시 읽습니다.
- [x] 이전 지연 mirror보다 최신 GUI draft를 우선해 프로젝트별 새 mirror를 기록합니다. 이전 canonical storage key는 물리 이동 전에 캡처하고 새 기록 후 서로 다른 key일 때만 이전 기록을 지웁니다. case-only 같은 canonical key에서는 새 mirror를 지우지 않습니다. 새 mirror 쓰기 실패 시 기존 복구 초안을 먼저 삭제하지 않습니다.
- [x] stale destination 문서는 dirty 여부와 무관하게 source 본문으로 대체하고 모든 destination view를 합류시킵니다. 원본 `retargetModel`과 같은 정책이며 이전 native의 dirty destination 거절을 제거했습니다. undo/redo·syntax·IME composition과 destination fold는 초기화하며 source fold는 유지합니다.
- [x] root owned mutation guard·TaskOperationLease·ActivityOwner를 물리 blocking 작업이 소유하고 GUI commit ack까지 돌려받아 유지합니다. GUI 편집·닫기·다른 workspace edit을 해당 구간 동안 막고 old-path mirror/save/format generation을 취소합니다. 성공 또는 실패 후 살아 있는 dirty 문서의 persistence를 재예약하는 코드도 연결했습니다.
- [x] GUI files 표시 경로·displaced id·loading/failed/observation/diagnostics·dirty tab 상태와 완료 layout을 연결했습니다. 후속 WorkspaceEdit에서는 source/목적지의 폐기 snapshot을 제거하고 새 URI·현재 snapshot을 사용합니다. 기존 실제 LSP registry의 URI 변경 didClose→didOpen 경로를 재사용합니다.
- [x] 물리 rename 이후 metadata/mirror 실패는 새 layout·GUI 경로 commit과 함께 경고를 전달합니다. 이미 이동된 파일을 이전 경로의 정상 문서인 것처럼 유지하지 않으며 이후 WorkspaceEdit은 오류로 중단합니다. 완전한 파일 시스템 transaction/rollback은 아닙니다.

## 실제 검사

- [x] `cargo test --manifest-path native/taide-native-app/Cargo.toml --test lsp-workspace-worker 실제_workspace_rename --locked --offline --target-dir experiments/native-shell-spike/target`: 신규 통합 1건 통과, 실행 0.06초입니다. 실제 합성 파일·중첩된 두 프로젝트·shared view·dirty stale destination·최신 본문/mirror·extension 언어 변경·rename→새 URI 편집·폴더 하위 이동·미열림 파일·closed history·목적지 옵션 우선순위·프로젝트/root 거절·접두사 형제 보존·guard ack·worker 회수를 검사했습니다.
- 검사 fixture가 기존 `open_tab`의 실제 4개 인자 시그니처와 맞지 않아 최초 컴파일 실패했습니다. `Tab`을 직접 구성하는 수정 중 `view_state` 필드 누락도 컴파일로 확인해 추가했습니다. 기능 assertion은 완화하지 않았고 첫 기능 실행에서 통과했습니다.
- [x] `cargo test --manifest-path native/taide-native-editor/Cargo.toml --test file-retarget --locked --offline --target-dir experiments/native-shell-spike/target`: dirty destination 정책을 변경한 관련 core 1건 통과, 0.00초입니다. 기존 stale/invalid identity·late save·공유 선택/스크롤/fold·undo 검사를 유지했습니다.
- [x] app lib/bin·lsp-workspace-worker·lsp strict clippy exit 0, 0.92초입니다. 최초 명령의 잘못된 bin target `taide-native`는 실행 오류였으며 실제 package/bin `taide-native-app`로 수정했습니다. editor lib/file-retarget strict clippy exit 0, 0.21초입니다. 동일 성공 기능 검사는 재실행하지 않았습니다.
- [x] app/editor `cargo fmt --check`, 대상 QA Prettier와 `git diff --check`가 모두 exit 0입니다.
- 기존 실제 DeleteFile·취소 save·즉시 layout gate·실제 LSP URI sync 성공은 재사용합니다. 공통 Activity/root helper 추출로 휴지통 삭제를 다시 실행하지 않았습니다.

## 전체 M8에서 남은 검증·연결

- [x] explorer inline 이름 변경·Enter/F2·Rename context action과 기존 worker→tree/선택 연결은 후속 `2026-10-01-m8-native-explorer-rename.md`에 기록했습니다. 실제 시스템 GUI·모든 context action·open-with/agent/derived byte cache와 전체 view는 미완료입니다.
- [ ] 실제 case-insensitive/case-only FS·symlink alias 전체·이동되는 디렉터리 안의 별도 열린 project root·외부 프로세스 동시 교체·mid-blocking cancellation·mirror/metadata I/O 실패 injection은 아직 직접 검사하지 않았습니다. 재현 조건과 실제 ownership/복구 결과를 검사해야 합니다.
- [ ] resourceOperations capability 광고·live protocol version/다중 root·모든 provider와 실제 GUI 저장 예약/종료는 미완료입니다. rename 뒤 후속 텍스트에 새 protocol revision을 확인하지 못한 경우 `None`인 기존 background 정책을 사용하며 전체 버전 동등성을 주장하지 않습니다.
- [ ] 기존 source가 clear-before-write였던 mirror 순서와 native의 write-before-clear 차이는 초안 보존을 위한 구현 차이입니다. 전체 crash recovery/failure parity는 검사 전 완료 처리하지 않습니다.
- [ ] N1~N8·213개 view·Rust 99%·TS 제거·성능/보안·배포/Git는 미완료입니다. 사용자 실기 앱·입력기·VoiceOver·OS 설정을 조작하지 않았습니다. 임시 합성 fixture만 테스트 자체의 cleanup으로 제거했습니다.
