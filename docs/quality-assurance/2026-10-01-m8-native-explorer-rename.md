# M8 native 탐색기의 이름 변경 입력

## 대상·원본

- 대상: `native/taide-native-app/src/{explorer,application,lsp,lib}.rs`, `tests/explorer.rs`, 변경된 `tests/lsp.rs` 응답 분기입니다.
- 원본: `src/features/explorer/{file-tree,file-tree-draft-row,explorer-shortcuts}.tsx` 또는 `.ts`, `src/widgets/explorer/use-explorer-entry-crud.ts`, `src/shared/lib/{entry-name,ime-composition}.ts`입니다. 실제 파일·문서 이동은 기존 native `workspace_rename`을 재사용합니다.
- 고정 egui 0.36.2의 `TextEdit`·`TextEditState`·`AtomLayoutResponse`·IME event·Tooltip·ScrollArea source와 현재 `lsp-types` 0.97.0의 RenameFile DTO를 읽고 사용했습니다. 새 dependency·제품 lock 변경·GUI 앱 실행은 없습니다.

## 연결한 동작

- [x] 프로젝트별 selected path와 inline rename 상태를 실제 explorer surface에 연결했습니다. 클릭으로 선택·preview/open, 우클릭 Rename, focused tree의 Enter/F2와 화살표 이동에서 입력을 시작합니다. 입력 시작과 실패 재시도 때 전체 이름을 선택합니다.
- [x] 원본의 trim·빈 이름/동일 이름 취소·마지막 segment의 reserved/invalid/trailing dot·실제 destination sibling 중복 검사와 nested 이름을 구현했습니다. 오류는 locale 메시지와 항상 열린 입력 tooltip으로 표시합니다. 실제 filesystem collision/root 검사는 기존 서비스가 다시 수행합니다.
- [x] Enter 확정·Escape/포커스 이탈 취소와 IME 조합/commit frame의 Enter/Escape 비실행을 연결했습니다. 처리 중 request latch로 중복 확정을 막으며 failed completion은 입력·오류를 유지하고 성공은 새 경로 선택으로 바꿉니다.
- [x] bounded 기존 bridge command→별도 TaskSupervisor 작업→공통 native rename prepare/GUI ack→기존 tree refresh/reveal→새 전체 TreeRowPage를 연결했습니다. local explorer rename은 LSP 서버의 존재나 process 요청에 의존하지 않으며 같은 bridge의 GUI 응답 경로만 재사용합니다.
- [x] source 프로젝트와 entry-scoped root를 확인합니다. 디렉터리 parent만 canonicalize하는 기존 정책을 사용해 마지막 symlink 자체의 이름 변경을 content 대상 권한과 혼동하지 않습니다. 공통 rename의 모든 포함 프로젝트 경계·목적지 collision·문서/mirror/guard 정책은 그대로입니다.
- [x] rows 전체를 매 frame clone하지 않고 visible row range만 렌더합니다. 작업 중 원래 row가 tree에서 먼저 없어지면 해당 draft row 하나만 유지합니다. 기존 22px row 상수와 원본 12px depth indent를 사용하며 전체 geometry/theme/icon gate는 아래 미완료 항목입니다.

## 실제 검사·오류 수정

- [x] `cargo test --manifest-path native/taide-native-app/Cargo.toml --test explorer --locked --offline --target-dir experiments/native-shell-spike/target`: 최초 실제 기능 실행에서 파일 통합 1건 통과·입력 1건 실패, 전체 실행 0.02초입니다. 입력의 실제 pointer click·F2·전체 선택·검증·nested 이름·중복 확정·오류 유지까지 통과한 뒤 IME Enter에서 rename 상태가 사라지는 오류를 재현했습니다.
- 최초 fixture 컴파일은 egui cursor의 `CharIndex`를 usize와 비교해 실패했습니다. 고정 API 타입으로 비교를 수정했으며 assertion 기대값을 바꾸지 않았습니다. 앞선 app 컴파일의 HostReply/Reply 분기 위치·AtomLayoutResponse 타입 연결과 deprecated IME variant 사용도 실제 API에 맞춰 수정했습니다.
- [x] 조합 중 Enter/Escape도 기본 TextEdit에 넘기지 않도록 consume하되 rename 액션만 비실행하도록 수정했습니다. 관련 입력 검사만 재실행했고 조합/commit 단계는 통과했으나 후속 Escape 취소 assertion에서 실패했습니다. 이미 포커스를 잃은 응답에 current `owns_focus`까지 요구하던 취소 조건을 제거해 실제 `lost_focus` 전환을 처리했습니다.
- [x] `cargo test --manifest-path native/taide-native-app/Cargo.toml --test explorer 탐색기_이름입력 --locked --offline --target-dir experiments/native-shell-spike/target`: 수정한 관련 1건 통과, 0.02초입니다. Escape·Enter 재진입·외부 포커스 이탈·빈 이름 취소까지 assertion을 유지했습니다. 통과한 파일 통합은 반복하지 않았습니다. 자세한 재현은 `docs/bug/2026-10-01-native-explorer-ime-enter-cancel.md`입니다.
- 파일 통합의 실제 범위는 headless egui 입력→bridge 제출→합성 실제 파일 rename·문서 metadata→refresh/reveal된 새 tree→새 경로 선택·worker join입니다. UI app의 모든 lifecycle/OS event를 실행했다고 주장하지 않습니다.
- [x] app lib/bin·explorer·lsp 대상 strict clippy 최종 exit 0, 0.57초입니다. 중간 성공 0.96초 뒤 불필요한 focus 대입 경고가 생겨 해당 대입을 제거한 최종 결과입니다. 기능 성공 검사를 다시 실행하지 않았습니다.
- [x] app `cargo fmt --check`, 변경 QA/bug Prettier와 `git diff --check`가 모두 exit 0입니다.

## 남은 전체 탐색기·M8

- 후속 구현: `2026-10-01-m8-native-explorer-create.md`와 `2026-10-01-m8-native-explorer-open-and-context.md`에 toolbar 생성/refresh/collapse·header·경로별 parent 펼치기·row context 생성·Space/Cmd+Down을 기록했습니다. 실제 TS container의 onOpenPreview도 false를 넘기므로 여기의 초기 단일 클릭 preview=true 구현을 source false로 수정했습니다. 초기 성공을 현재 preview 정책의 근거로 사용하지 않습니다. 전체 explorer 완료 판정은 그대로 미완료입니다.

- [ ] 탐색기 toolbar/새 파일·폴더·삭제 확인·클립보드/붙여넣기·multi-selection/typeahead·모든 context action·open-with·Git badges·auto reveal·키맵/locale/theme/아이콘/정확한 scroll geometry를 연결해야 합니다. 이름 변경 입력만으로 213개 view 또는 전체 explorer를 완료 처리하지 않습니다.
- [ ] source toast retry와 동일한 notification surface, 장애 중 partial rename의 최종 입력/selection 처리·큐 포화/종료·project close·source 변경·단일 디렉터리 안의 nested project root·symlink/case-only 실제 OS·권한 실패는 관련 구현/검사 범위로 남습니다.
- [ ] 실제 한글·일본어·중국어 IME와 VoiceOver는 사용자가 직접 수행하기로 한 마지막 실기 검증입니다. 합성 IME event 검사는 실제 시스템 입력기 통과를 대신하지 않습니다. 입력기·VoiceOver·기존 실기 bundle/프로세스를 변경하지 않았습니다.
- [ ] M8 N1~N8·Rust 99%·TS 제거·전체 성능/보안/배포·최종 commit/push는 아직 미완료입니다. 기존 사용자 작업과 합성 fixture 밖의 파일은 건드리지 않았습니다.
