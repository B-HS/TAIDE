# M8 native 저장 정리 연결

## 대상 파일과 원본

- Core: `native/taide-native-editor/src/{document,store,save_cleanup,syntax}.rs`와 해당 검사.
- App: `native/taide-native-app/src/{application,host,save}.rs`, `tests/save.rs`.
- 원본: `src/widgets/editor-pane/use-editor-file-persistence.ts`, `src/shared/lib/monaco/on-save-cleanup.ts`, `src/shared/lib/editorconfig.ts`.
- 설치된 Monaco의 trimTrailingWhitespaceCommand, InsertFinalNewLineCommand, pieceTreeTextBufferBuilder, tracked selection과 standalone configuration 구현을 직접 대조했습니다.

## 구현 결과

- [x] dirty 문서만 저장 참여자를 실행합니다. clean 저장은 정리·파일 쓰기 없이 성공하고, 닫기 저장에서도 미발생 reply를 기다리지 않습니다. readonly 거절은 clean 판정보다 먼저입니다.
- [x] 실제 편집·undo·redo는 dirty를 유지합니다. undo로 disk와 같아져도 저장/명시적 정착 전에는 clean으로 바꾸지 않습니다. untitled 합류 대상도 같은 dirty 판정을 사용합니다.
- [x] `.editorconfig`의 명시적 true/false가 전역 설정보다 우선합니다. ASCII space/tab만 trim하고, 자동 저장은 같은 줄의 가장 오른쪽 선택 head 왼쪽 공백을 보존합니다.
- [x] 빈 마지막 줄과 space/tab만 있는 마지막 줄에는 개행을 추가하지 않습니다. model의 CRLF/LF 선택을 유지하고 origin 커서를 추가 개행 앞에 남깁니다. trim/newline은 각각 undo 경계를 갖고 공유 view 선택을 보존합니다.
- [x] 비plaintext의 unknown/inaccurate token은 trim하지 않으며 String/Regex도 보호합니다. revision/language/줄/UTF-8 경계를 확인한 토큰만 받습니다. 참여자 실패는 다른 참여자나 쓰기 가능한 snapshot 저장을 취소하지 않습니다.
- [x] 실제 App은 마지막 편집 view와 AutoSave command를 구별하고, 정리 후 최종 snapshot·mirror revision을 기존 tracked worker에 전달합니다. worker가 저장한 뒤 발생한 편집은 dirty로 남습니다.

## 단일 검증 증거

- 회귀 재현: 신규 dirty-latch 검사에서 undo 후 dirty assertion이 실제 실패했습니다. 첫 실행의 fixture 중첩 mutable borrow는 검사 코드 수정 후 재현했습니다.
- `cargo test --manifest-path native/taide-native-editor/Cargo.toml --test save_cleanup --test store --test save_boundary --test untitled --locked --offline --target-dir experiments/native-shell-spike/target`: 정리 신규 5건과 dirty 판정 영향 기존 10건, 총 15건 모두 통과, 각 suite 실행 0.00초.
- `cargo test --manifest-path native/taide-native-app/Cargo.toml --test save --locked --offline --target-dir experiments/native-shell-spike/target`: 실제 합성 root의 `.editorconfig` 읽기·clean no-op·stale 거절·auto cursor 보호·수동 정리·CRLF 저장·late draft·readonly·untitled 경계·worker 추적 0, 1건 0.03초 통과.
- App 대상 strict clippy는 lib/bin와 신규 save 검사 exit 0, 0.89초입니다. Core 대상 strict clippy는 lib와 정리/dirty 영향 검사 exit 0, 0.24초입니다.
- 변경 상태가 같은 성공 검사는 반복하지 않았습니다. 저장 정리 단계 자체에는 신규 의존성이 없습니다. 후속 LSP의 기존 타입 재사용은 별도 문서에 기록합니다. OS 앱 실행·실기 bundle 변경·입력기/VoiceOver 변경·commit/push는 없습니다.

## 미완료 경계

- [ ] 실제 syntax/token provider와 renderer가 아직 연결되지 않았습니다. 토큰 계약 검사는 trim 전체 동등성 완료가 아닙니다. 실제 provider를 연결한 뒤 언어별 문자열/regex 보호를 확인합니다.
- [ ] raw Rope 입장 시 mixed/lone CR와 paste/insert EOL 정규화는 남았습니다. 현재 majority EOL 선택과 final newline만 구현했고 전체 Monaco EOL 동등성을 주장하지 않습니다.
- [ ] formatter·명시적 fixAll/organizeImports·didSave를 실제 native LSP composition과 연결해야 합니다. 정리 단계 성공을 전체 저장 파이프라인 완료로 세지 않습니다.
- [ ] tracked selection의 모든 Monaco stickiness·실제 GUI 동등성은 N3에서 확인합니다. 현재 origin final-newline caret와 공유 trim 선택의 코드 검사만 완료입니다.
- [ ] 기존 파일 삭제 관찰의 missing-source 전환, 전체 close/unmount/project lifecycle, 실제 OS autosave/IME/VoiceOver 및 전체 M8 cutover는 미완료입니다.

M8 상위 N1–N8은 완료하지 않습니다. 코드 우선 재현을 이어가며 실제 UI·실기 검사는 마지막 순위로 유지합니다.
