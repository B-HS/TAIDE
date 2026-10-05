# M8 native WorkspaceEdit 삭제·탭 회수 연결

## 대상·원본

- 실제 app: `native/taide-native-app/src/{workspace_delete,lsp_workspace_worker,lsp,application,file_sync}.rs`, `tests/{lsp-workspace-worker,disk}.rs`.
- 문서/UI: `native/taide-native-editor/src/store.rs`의 cheap snapshot 조회, `native/taide-native-ui/src/controller.rs`의 완료 layout 반영과 `tests/controller.rs`.
- 원본: `src/shared/lib/lsp/workspace-edit-applier.ts`의 DeleteFile, `src/entities/file/workspace-resource-operations.ts`의 프로젝트별 unsaved/mirror 검사와 `src/entities/layout/tab-path-change.ts`의 삭제 후 탭·모델 회수입니다. 기존 root `delete_entry`의 OS 휴지통 동작과 layout path-change 서비스를 재사용합니다.

## 연결한 동작

- [x] URI를 검증하고 open-project entry 경계와 optional server roots를 확인합니다. CLI 단일 파일 승인을 삭제 권한으로 확대하지 않습니다. final symlink는 파일 내용으로 따라가지 않고 기존 entry service 경계를 사용합니다. ignoreIfNotExists는 경로 승인 뒤 적용하며 recursive 옵션은 원본처럼 기존 삭제 서비스의 동작을 바꾸지 않습니다.
- [x] GUI의 실제 dirty 문서와 display alias·runtime tab dirty·모든 해당 프로젝트의 mirror를 확인합니다. root layout은 먼저 clone에서 닫기를 검증하며 파일 삭제가 실패하면 실제 layout을 저장하지 않습니다. 성공 시 해당 프로젝트의 주·보조 창 File 탭을 기존 path-change 계약으로 닫고 closed history를 남깁니다. Diff/ClaudeDiff를 임의로 File 탭처럼 닫지 않습니다.
- [x] owned mutation guard·tracked operation·resource activity는 실제 blocking 삭제부터 GUI commit ack까지 유지합니다. activity의 drop이 실패/정상 종료를 repaint에 알립니다. app은 이 짧은 변경 경계에서 편집/다른 workspace text edit·종료를 막고 affected persistence/formatter generation을 취소합니다. 취소된 저장의 기존 경로 재생성 오류는 별도 bug 문서에 재현·수정 근거를 기록합니다.
- [x] GUI는 최신 document identity/revision/dirty를 확인한 뒤 닫힌 view·last document·cached file/loading/observation/diagnostics/dirty 상태를 회수합니다. 다른 display path나 살아 있는 view가 문서를 사용하는 경우 유지합니다. 문서 회수 ID를 worker에 반환해 같은 WorkspaceEdit의 delete→create→edit가 폐기된 이전 snapshot을 재사용하지 않습니다.
- [x] root controller의 비동기 snapshot은 owned guard가 풀리기 전 새 layout을 읽을 수 없습니다. GUI commit의 layout을 revision gate로 즉시 반영하고 더 오래된 snapshot으로 되돌아가지 않습니다. backend snapshot이 따라오면 별도 clone을 회수하며 닫힌 프로젝트를 다시 설치하지 않습니다.

## 직접 검증

공통 flags는 `--locked --offline --target-dir experiments/native-shell-spike/target`입니다.

- [x] `cargo test --manifest-path native/taide-native-app/Cargo.toml --test lsp-workspace-worker 실제_workspace_delete` 최초 sandbox 실행은 실제 macOS 휴지통 API의 permission 오류로 실패했습니다. 구현을 영구 삭제로 바꾸지 않았으며 해당 합성 검사에만 escalation해 1건 통과, 0.15초입니다.
- [x] 이후 회수 ID와 폐기 snapshot 제거를 추가하고 같은 경로의 delete→create→edit로 검사를 확장한 변경 상태에서 1건 통과, 0.23초입니다. dirty buffer/mirror 보존·server root 거절·missing 옵션/승인 경계·중첩된 두 프로젝트의 탭과 공유 view/document 회수·worker 수명·GUI ack까지의 mutation 잠금·관련 없는 이름-prefix 파일 보존·새 파일 내용 `recreated`와 tracked count 0을 확인했습니다. 동일 코드/입력의 성공 검사를 반복한 것이 아닙니다.
- [x] `cargo test --manifest-path native/taide-native-ui/Cargo.toml --test controller 완료_layout은` 신규 1건 0.00초 통과입니다. 실제 controller/owned guard 상태에서 즉시 새 layout을 읽고 stale revision을 거절하며 backend ack 뒤 Arc snapshot 재사용·프로젝트 close 후 비복원·worker 회수를 확인했습니다. 실제 GUI 픽셀 검사는 아닙니다.
- [x] 취소된 저장 신규 검사는 실패 재현 후 0.01초에 통과했습니다. 최종 app lib/bin 및 disk·lsp·workspace-worker strict clippy exit 0, 0.91초이며 UI lib/controller 대상도 exit 0, 0.88초입니다.

검사 중 임시 디렉터리에 새로 만든 합성 파일 두 개만 macOS 휴지통으로 이동했습니다. 휴지통에서 복구할 수 있으며 기존 사용자 파일·실기 앱·OS 설정은 변경하지 않았습니다.

## 남은 전체 resource·M8 범위

- [ ] RenameFile의 모든 tab/metadata/mirror·epoch 이동, dormant mirror admission, live multi-root session과 전체 version 갱신을 연결해야 합니다. CreateFile/DeleteFile worker를 모든 resource provider의 최종 완료로 세지 않으며 resourceOperations capability는 아직 광고하지 않습니다.
- [ ] 디렉터리·symlink/display alias·pinned tab·파생 view 캐시/agent wait marker·닫기/reopen/실제 GUI·queue 포화/Exit/중간 실패/외부 FS 교체를 검증해야 합니다. 닫을 수 없는 탭은 현재 실제 삭제 전 거절하며 원본의 삭제 뒤 close 실패와 같은 동작으로 계산하지 않습니다.
- [ ] 새 완료-layout 경계를 다른 host layout mutation에도 필요한지 적용·검토해야 합니다. 이번 삭제 연결과 controller의 직접 검사만으로 모든 native 화면의 수명을 검증했다고 주장하지 않습니다.
- [ ] N1~N8·213개 view·Rust 99%·TS 제거·최종 서명/공증/install/upgrade·commit/push는 계속 미완료입니다. 사용자 요청대로 실기 검증은 코드 구현 뒤 마지막 순서입니다.
