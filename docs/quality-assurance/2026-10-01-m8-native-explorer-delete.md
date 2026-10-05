# M8 native 탐색기 삭제 확인·선택 프로젝트 삭제

## 원본과 정책

- 원본은 `src/features/explorer/{entry-delete-dialog,file-tree-context-menu,explorer-shortcuts}`, `src/widgets/explorer/use-explorer-entry-crud.ts`, `src/entities/file/file.query.ts`, `src/entities/layout/tab-path-change.ts`, 기존 Rust Trash/layout 서비스입니다.
- 탐색기는 확인 후 plain fileDelete를 호출하고 선택 프로젝트의 main/aux File 탭을 닫습니다. 이 경로에는 LSP workspace-resource-operations의 dirty/mirror 거절이 없습니다. 기존 native `workspace_delete.rs`는 LSP의 all-project/require-saved 정책으로 유지하며 GUI 삭제에 그대로 연결하지 않았습니다.
- 설치된 Radix AlertDialog source의 취소 초기 focus·외부 pointer/interact 기본 취소 방지와 egui 0.36.2 Modal source를 읽었습니다. Modal::should_close의 backdrop 취소는 사용하지 않습니다. locale 제목/설명·취소·destructive 확인·Escape와 확인 후 modal 닫힘을 연결했습니다.

## 구현과 범위

- 대상은 `native/taide-native-app/src/{explorer,delete_dialog,explorer_delete,application,lsp,lib,missing_draft}.rs`, `tests/{explorer-delete,lsp}.rs`입니다. 기존 앱·root MSRV·dependency·실기 bundle은 변경하지 않았습니다.
- row context의 Delete와 tree-local exact Cmd+Backspace가 captured project/path/name을 요청합니다. 원본처럼 modifier 추가 시 단축키를 실행하지 않으며 popup/inline 입력은 기존 tree gating을 사용합니다. modal은 app 단위 하나이며 취소/확정 시 tree focus를 요청하고 확정 후 자동 닫습니다. 제출·중간 실행의 duplicate latch를 둡니다.
- 별도 GUI-confirmed worker가 selected project의 절대 entry path/root/live/shutdown을 확인합니다. 프로젝트 root 자체나 CLI 승인만 있는 외부 파일은 삭제하지 않습니다. tracked operation/owned mutation guard를 파일 작업과 GUI completion ack까지 유지합니다.
- GUI prepare는 canonical snapshot과 표시 경로로 찾은 추가 document를 수집합니다. dirty snapshot을 거절하지 않으며 기존 generation/mirror deadline·pending format/save epoch를 중단합니다. commit은 captured key/revision을 재검사하고 선택 프로젝트의 닫힌 view만 회수합니다. 다른 프로젝트의 같은 파일 탭은 닫지 않습니다.
- 다른 프로젝트에 살아 있는 삭제 경로의 탭은 최신 공유 snapshot, 없으면 기존 mirror, 없으면 disk content를 먼저 mirror에 보존합니다. 기존 source-missing 읽기 전용 초안/Save As 화면으로 전환하고 해당 view 회수 후 다른 표시 경로가 document를 사용하지 않을 때만 승인한 revision을 폐기합니다. unrelated document와 파일은 유지합니다. 남은 canonical alias가 있다면 source-missing draft를 근거로 auto-save delay를 0으로 두어 자동 경로 재생성을 막습니다.
- GUI에 실제 삭제 완료 event가 도착하면 commit 성공 여부와 무관하게 affected document를 auto-save 금지로 표시합니다. 후처리 실패 뒤 suspended 작업을 재개하더라도 삭제 경로를 자동 생성하지 않습니다. 문서 회수나 실제 성공한 명시적 저장에서 해당 표시를 지웁니다. 이 실패 경로의 전체 GUI 통합은 아직 미검증이며 코드 경계와 정적 검사로 구분합니다.
- 실제 파일 삭제는 기존 macOS recoverable Trash 서비스이며 영구 삭제로 대체하지 않습니다. layout/self-write/FsRescan 뒤 GUI ack를 기다리고 완료 후 parent tree refresh를 요청합니다. 후처리 실패가 물리 삭제를 rollback하지 않으며 status에 오류를 표시합니다.
- 중첩 root에서 surviving 프로젝트의 mirror가 더 좁은 다른 프로젝트의 소유권 선택 때문에 Save As 거절되는 오류를 재현했습니다. draft.project의 live root와 source canonical identity/missing/mirror를 직접 검사하도록 수정했습니다. root 밖이나 다른 mirror를 허용하는 변경이 아닙니다.

## 실제 검사와 재사용

공통 Cargo 옵션은 `--manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target`이며 Cargo는 한 번에 하나만 실행했습니다.

- [x] lib/bin 최초 check는 RefreshTree dirs의 Vec/BTreeSet 자료형 차이로 실패했습니다. 실제 BTreeSet 계약으로 수정했고 후속 test compile/strict clippy로 해당 lib/bin을 검사했습니다. 불필요한 같은 check 반복은 하지 않았습니다.
- 새 fixture의 최초 compile은 잘못된 layout_open_tab 인자와 존재하지 않는 TaskSupervisor::stats 호출로 실패했습니다. 실제 signature/tracked_count로 바로잡았고 unused import·test identifier 경고도 제거했습니다. 컴파일 실패를 기능 검사로 세지 않습니다.
- [x] `cargo test ... --test explorer-delete`: 실제 context menu pointer→delete action, exact shortcut, modal 취소 초기 focus/backdrop 비취소/Escape/confirm pointer 검사 1건 PASS입니다. 같은 suite의 실제 Trash 검사는 sandbox 권한으로 FAILED이며 전체 suite 결과는 1 PASS/1 FAIL, 0.39초입니다. 이 결과를 전체 통과로 쓰지 않습니다.
- [x] `cargo test ... --test explorer-delete 실제_탐색기_삭제`: 정확한 합성 fixture의 macOS Trash 접근에만 좁은 실행 권한을 사용해 1건 PASS, 0.30초입니다. dirty/latest document 승인·선택 프로젝트만 닫기·다른 프로젝트 mirror 최신 본문과 source_missing·document/view 회수·unrelated 내용 보존·mutation guard 유지·project root/CLI-only 외부 거절·worker 0을 확인했습니다. 같은 LSP strict dirty prepare는 여전히 거절함도 확인했습니다. 통과한 modal 검사는 재실행하지 않았습니다.
- [x] `cargo test ... --test explorer-delete 삭제후_공유초안`: 중첩 root의 surviving mirror Save As가 Forbidden으로 먼저 실패, 0.02초였습니다. source scope 수정 뒤 관련 1건만 PASS, 0.03초입니다. 최신 내용의 실제 destination 저장, 원본 부재 유지, 정확한 mirror cleanup, worker 0을 확인했습니다. 앞선 Trash 성공은 반복하지 않았습니다.
- [x] 최초 lib/bin·explorer-delete/explorer/lsp strict clippy exit 0, 2.87초입니다. 그 뒤 변경된 missing_draft/new fixture의 최종 좁은 검사 결과는 아래에 기록합니다.
- [x] missing_draft/new recovery fixture 변경 후 lib/bin/explorer-delete strict clippy exit 0, 1.01초입니다. 이후 auto-save 금지 표시와 신규 ack fixture가 변경돼 마지막 정적 검사를 별도로 남깁니다.
- [x] `cargo test ... --test explorer-delete 삭제_ack`: 늦은 revision 거절이 view/latest 본문을 그대로 유지하고 승인한 dirty/pinned File 탭의 closed history를 clean으로 만드는 신규 비화면 경계 검사 1건 PASS, 0.00초입니다. 물리 파일은 삭제하지 않았으며 이전 Trash/modal/복구 성공은 재실행하지 않았습니다.
- [x] 마지막 auto-save 금지 표시/new ack fixture의 lib/bin/explorer-delete strict clippy exit 0, 1.09초입니다. cargo fmt, 해당 QA/bug의 Prettier와 git diff --check도 exit 0입니다. 앞선 성공 기능 검사는 반복하지 않았습니다.

## 남은 검증·동등성

- [ ] actual OS modal/keyboard focus trap·취소 focus 복귀·외부 클릭 뒤 focus·VoiceOver/IME·locale/theme/정확한 geometry/픽셀은 최후 실기입니다. headless pointer/input와 lib/bin compile을 앱 실행·모든 화면 동등성으로 확대하지 않습니다.
- [ ] selected auxiliary File 탭/세 개 이상 overlapping 프로젝트·diff/preview 독자·global path-keyed draft의 project 선택·여러 표시 alias·symlink/case-only·외부 파일 교체/큐 포화/ack 중 종료와 실패 후 retry는 전체 수명/동등성 게이트에 남습니다. 현재 검사는 두 프로젝트의 실제 main File 탭과 한 canonical dirty 문서를 덮습니다.
- [ ] 기존 selected project mirror의 source-missing 보존/정리·다른 프로젝트 model/cache 재사용의 원본 TS 세부 경계, 물리 Trash 뒤 GUI commit/parent refresh 실패 알림과 재시도, 실제 Git/FileIndex 갱신의 전체 연결은 미완료입니다. 안전한 최신 survivor mirror 보존을 전체 TS 후처리 일치로 주장하지 않습니다.
- [ ] 모든 Explorer action/view·전체 editor/LSP/terminal·성능/보안/beta/rollback/배포·TS 제거/Rust99%와 M8 N1~N8은 미완료입니다.

사용자 앱·실기 bundle·OS 설정·실제 clipboard를 조작하지 않았습니다. 성공한 합성 검사 파일 한 개는 macOS 휴지통에서 복구할 수 있습니다. 실패한 sandbox fixture와 나머지 합성 임시 데이터는 fixture cleanup으로 정리됐습니다. M8 전체 완료 전 commit/push는 하지 않았습니다.
