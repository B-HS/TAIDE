# M8 native 문서 소유·화면 상태·파일 admission 구현

## 대상·기준

대상은 `native/taide-native-editor/{Cargo.toml,src/*,tests/*}`와 native UI의 `src/document_admission.rs`·해당 검사입니다. 기준은 roadmap의 단일 DocumentStore·독립 ViewStore·공통 transaction 계약, 기존 `use-editor-view-state.ts`·`use-editor-file-persistence.ts`의 분할 화면/저장 수명과 Rust 파일 정책입니다.

기존 후보 Ropey 1.6.1의 cr_lines/simd 구성을 격리 native 구현에 재사용합니다. [공식 Rope API](https://docs.rs/ropey/1.6.1/ropey/struct.Rope.html)와 기존 indexed transaction source를 확인했습니다. root MSRV 1.89·기존 제품 manifest/lock·실기 bundle은 유지합니다. 최종 후보 채택을 의미하지 않습니다.

## 구현

- DocumentId는 재사용하지 않는 identity입니다. canonical 파일 경로/untitled TabId별 document와 Rope·revision·disk baseline·파일 정책·undo/redo를 단일 소유합니다. 같은 파일의 추가 open은 기존 dirty 본문을 stale disk 응답으로 교체하지 않습니다.
- 화면은 window/pane/tab별 ViewId를 가지며 선택 set·primary·scroll·fold·IME preedit을 분리합니다. preedit은 본문에 넣지 않습니다. transaction에서 같은 document를 보는 모든 view의 선택/접힘 좌표를 변환하고 origin의 명시적 선택을 적용합니다.
- byte/UTF-8 경계·overlap·revision·read-only/lossy/refused·caller byte 상한·선택 유효성을 commit 전에 검사합니다. 여러 삽입의 원래 순서를 유지하고 내용·view 변경을 공통 transaction으로 처리합니다.
- caller의 undo group/origin에 따라 묶되 save snapshot 생성 시 group을 분리합니다. undo/redo revision은 단조 증가하고 원래 view의 선택을 복원하며 이후 추가된 view의 좌표는 유효한 경계로 제한합니다.
- 저장 snapshot은 불변 Rope 사본입니다. 저장 중 편집이 있으면 완료 시 disk baseline만 저장된 본문으로 갱신하며 새 편집의 dirty를 유지합니다. 뒤늦은 구 revision 완료는 baseline을 덮지 않습니다. 실제 저장 worker의 직렬 실행과 파일 쓰기는 아직 host 연결 항목입니다.
- dirty 문서는 마지막 view를 닫아도 store에 남습니다. attached/dirty document의 일반 release를 거절합니다. 별도 discard 확인·save-as·project close 정책은 host 구현에서 연결해야 합니다.
- caller의 document/view/group/본문 byte 상한을 사용하며 새 제품 기본값을 만들지 않았습니다. 파일/URI 문자열·selection·fold·preedit·undo 사본의 전체 byte/RSS 상한은 아직 제품 memory gate가 아닙니다.

## 실제 파일 commit 경계

`prepare_opened_document`는 기존 native 파일 응답을 받지만 과거 open 권한만 신뢰하지 않습니다. 기존 owned mutation guard를 다시 잡고 TaskSupervisor에 등록한 blocking worker에서 현재 프로젝트/CLI 단일 파일 승인과 canonical 경로를 다시 해석합니다. byte size와 동일한 mtime 경계를 비교합니다.

반환되는 PreparedDocument는 복제할 수 없는 일회성 token입니다. operation lease와 mutation guard를 보유해 app 내부 프로젝트 close가 commit/drop보다 앞서지 못하게 합니다. commit은 현재 shutdown을 다시 확인하고 native editor store를 사용합니다. callback 취소/실패 때 worker 결과나 token을 폐기하면 guard/lease가 함께 해제됩니다. GUI frame에서 filesystem을 읽지 않습니다.

이 잠금은 앱 내부 소유권 잠금이며 외부 프로세스의 파일 교체를 막지 않습니다. 검사 뒤 외부 교체, 같은 size/mtime로 내용 교체, 원래 file read의 race는 여전히 TOCTOU 검증 항목입니다. 실제 앱은 token을 다음 commit 단계에서 처리하거나 즉시 drop하고, close/flush 전에 남은 token을 회수해야 합니다. host 통합은 아직 미완료입니다.

## 검증 결과

- [x] `cargo test --manifest-path native/taide-native-editor/Cargo.toml --test store --locked --offline --target-dir experiments/terminal-core-spike/target`: 최초 3 passed/1 failed, 0.00초. 공유 본문·독립 view·저장 중 편집/undo·read-only·revision/UTF-8/선택 거절·preedit/group/stale save가 통과했고 restored untitled dirty assertion이 실패했습니다.
- [x] 해당 baseline과 인접 replacement/insertion의 선택 매핑을 수정한 뒤 `--test store 문서_초안`만 검사: 1 passed/0 failed, 0.00초. unchanged 나머지 성공은 재사용합니다.
- [x] `--test save_boundary`: 저장 시작 전후가 같은 typing group이어도 undo로 저장 본문에 정확히 돌아가는 새 검사 1 passed, 0.00초입니다.
- [x] `cargo test --manifest-path native/taide-native-ui/Cargo.toml --test document_admission --offline --target-dir experiments/terminal-core-spike/target`: 실제 합성 파일·기존 runtime에 대한 2 passed/0 failed, 0.00초. commit까지 root/작업 소유, 동일 canonical document 재사용, token drop 회수, 닫힌 root·바뀐 파일·shutdown 거절을 확인했습니다.
- [x] core all-target strict clippy exit 0(0.25초), 새 save test strict clippy exit 0(0.06초), native UI all-target strict clippy exit 0(0.92초), 대상 rustfmt/diff 검사 exit 0입니다.

source·입력·환경이 동일한 성공 검사는 반복하지 않았습니다. 모든 파일 I/O 검사는 새로 만든 합성 temp project에 한정했고 사용자 파일·앱·IME/VoiceOver 설정·bundle을 조작하지 않았습니다.

## 후속 editor surface 구현

`src/editing.rs`와 native UI의 `src/editor_surface.rs`에 실제 canonical Rope transaction을 사용하는 기본 입력과 viewport render를 연결했습니다. 상세 및 새 증거는 `2026-09-30-m8-native-editor-surface.md`에 기록했습니다. 이는 최초 store 검사 범위와 별개이며 executable·전체 editor parity 완료가 아닙니다.

## 남은 구현

- [ ] 실제 native executable/AppServices·document/view owner 연결과 전체 shaping/render 동등성
- [ ] LSP 좌표/동기화·revision feature reply, 전체 multi-cursor 명령/affinity·실제 IME/취소·fold 복원·decorations·AI/snippet/Emmet
- [ ] 실제 save/hot-exit/mirror·reload/conflict·save-as/rekey·dirty discard/close·app-owned 문서 처리
- [ ] pending admission의 실제 host 수명·외부 FS TOCTOU·metadata 업데이트·memory/history 전체 byte budget
- [ ] 전체 TS 화면 동등성·실제 픽셀·IME/VoiceOver·GPU·성능·배포 gate

이 통과 범위는 native 문서·화면 상태와 파일 admission 코드 경계입니다. editor surface·N2/N3 전체·M8 완료 증거는 아닙니다.
