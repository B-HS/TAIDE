# M8 native 파일 문서의 경로 전환 경계

## 대상·원본

- 대상: `native/taide-native-editor/src/store.rs`의 `retarget_file`, `tests/file-retarget.rs`, `native/taide-native-app/src/lsp.rs`의 문서 identity sync와 `tests/lsp.rs`입니다.
- 원본: `src/entities/editor/model-registry.ts`의 `retargetModel`, `src/entities/layout/tab-path-change.ts`의 이름 변경 경로·mirror 이동, `src/entities/file/workspace-resource-operations.ts`의 모든 해당 프로젝트 처리입니다.
- 원본 Monaco 경로 전환은 본문·언어·view state를 유지하지만 URI가 불변이므로 새 모델을 만들고 undo를 잃습니다. native도 경로 전환의 undo/redo를 초기화하며 저장으로 오인하지 않습니다.

## 구현·실제 검증

- [x] canonical File key를 snapshot identity/revision으로 검증한 뒤 바꿉니다. 문서 id·본문·baseline·dirty·공유 ViewId·커서·스크롤을 유지하고 언어/config 메타데이터를 갱신합니다. 새로운 경로에서의 주소 인덱스가 기존 문서를 찾으며 이전 경로는 제거합니다.
- [x] source selection/fold는 본문이 같아 유지합니다. clean destination 문서는 view를 source로 합류시키고 selection을 clamp하며 다른 본문에 속한 fold와 IME composition을 제거합니다. source undo/redo·syntax를 초기화하고 이전 경로의 늦은 save와 같은 canonical 경로의 이전 save도 거절합니다.
- [x] 실제 LSP app registry에서 같은 DocumentId의 URI 또는 language id가 바뀌면 기존 URI didClose → 새 URI didOpen을 실행합니다. 잘못된 새 URI didChange를 보내지 않으며 같은 root의 다른 문서와 공유 actor는 유지합니다.
- [x] `cargo test --manifest-path native/taide-native-editor/Cargo.toml --test file-retarget --locked --offline --target-dir experiments/native-shell-spike/target` 신규 1건 0.00초 통과입니다. 최초 fixture가 Copy가 아닌 ScrollPosition을 이동한 뒤 재사용해 컴파일 실패했으며 fixture의 clone으로 수정했습니다. 이후 displaced view의 fold 정리를 추가한 변경 위험만 같은 대상에서 1회 확인했고 source fold 보존·target fold 제거도 통과했습니다. 동일 코드 상태의 성공 검사를 반복하지 않았습니다.
- [x] `cargo test --manifest-path native/taide-native-app/Cargo.toml --test lsp 실제_native_app_lsp는 --locked --offline --target-dir experiments/native-shell-spike/target` 변경된 실제 process 통합 1건 0.41초 통과입니다. 합성 파일을 rename한 뒤 실제 문서를 retarget/sync해 공유 PID 유지·새 URI 포맷·닫기와 worker 회수를 확인했습니다. mock의 balanced didOpen/didClose와 열린 URI에만 format을 허용하는 계약은 완화하지 않았습니다.
- [x] 변경된 app lib/bin 및 lsp·workspace-worker 대상 strict clippy exit 0, 0.87초이며 editor lib/file-retarget 대상은 exit 0, 0.18초입니다. fold fixture 추가 뒤 single-range Vec lint가 실패해 같은 range 목록을 표준 once iterator로 구성했고 최종 대상 strict clippy exit 0, 0.07초입니다. 검사기를 끄거나 통과한 기능 검사를 재실행하지 않았습니다. 대상 rustfmt·QA Prettier·diff check exit 0입니다.

## 실제 이름 변경 후속과 남은 전체 범위

`2026-10-01-m8-native-workspace-rename.md`에서 실제 typed RenameFile·탭/mirror/문서 이동과 신규 검사를 기록했습니다. 아래 초기 미완료 상태 중 변경된 부분을 현재 구현으로 갱신합니다.

- [x] 실제 typed RenameFile worker·UI 표시 경로/문서/layout commit·generation 취소와 프로젝트별 mirror 이동을 연결했습니다. 실제 두 중첩 프로젝트·폴더 하위 파일 이동은 신규 통합 1건 0.06초 통과입니다. explorer UI/키맵·전체 alias/외부 교체·모든 view는 아직 미완료입니다.
- [x] 원본처럼 stale destination 모델은 dirty여도 폐기하고 source 본문과 view에 합류시킵니다. 변경된 core 1건 0.00초·실제 worker 통합에서 검사했습니다. 실제 목적지 파일의 collision 거절은 기존 Rust 서비스 정책을 유지합니다.
- [x] source metadata는 실제 이동 뒤 기존 file/plugin/config provider로 다시 읽으며 txt→rs 언어 변경을 통합에서 확인했습니다.
- [ ] case-only 실제 FS·외부 프로세스 동시 수정·GUI/OS keymap은 후속 구현·검증 범위입니다.
- [ ] N1~N8·213개 view·Rust 99%·TS 제거·최종 배포/Git는 미완료입니다. 사용자 실기 bundle·OS 설정은 유지했습니다.
