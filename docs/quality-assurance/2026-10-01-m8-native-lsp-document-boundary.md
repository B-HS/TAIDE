# M8 native LSP 문서·편집 경계

## 대상과 근거

- 제품 actor: `crates/taide-lsp/src/native/session.rs`.
- Editor adapter: `native/taide-native-editor/src/lsp.rs`, `tests/lsp.rs`.
- 실제 process 검사: `experiments/lsp-coordinator-spike/tests/native-session.rs`와 기존 bounded mock-server.
- 원본: `src/shared/lib/lsp/position.ts`, `src/shared/lib/lsp/adapters/formatting.ts`, `src/widgets/editor-pane/use-editor-lsp-integration.ts`.
- [공식 LSP meta model](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/metaModel/metaModel.json)의 Position·TextEdit·formatting 계약과 설치된 `lsp-types` 0.97.0/Ropey 1.6.1, Monaco textModel의 실제 좌표 보정 구현을 직접 확인했습니다.

## 구현과 검증

- [x] 기존 coordinator의 didClose/didSave를 실제 SessionClient command에 연결했습니다. URI 보유 capacity도 기존 command byte quota에 포함하며, 기존 supervisor·process owner·queue 정책은 바꾸지 않습니다.
- [x] 실제 child 왕복에서 open→change→save→close→reopen 순서, 최신 includeText 저장 통지, 닫힌 문서의 중복 close/save 거절, 재열린 문서의 원본 hover 결과, stop/child join을 확인했습니다. 신규 해당 filter 1건 0.35초 통과입니다. 다른 native-session 성공 검사는 반복하지 않았습니다.
- [x] Position/TextEdit를 손으로 재정의하지 않고 기존 제품 LSP와 같은 `lsp-types = 0.97.0`을 editor에 재사용했습니다. UTF-16/byte 변환은 기존 Ropey 인덱스를 사용합니다. 줄 밖은 EOF, 열 밖은 EOL로 보정하고 CRLF 내부 byte 표시는 EOL로 매핑합니다.
- [x] 형식화 편집 batch를 실제 transaction에 연결했습니다. 요청 document/key/revision, readonly, range 순서·UTF-8/surrogate 경계·overlap·용량을 확인하며 같은 원문 좌표로 모든 edit을 원자적으로 적용합니다. 공유 view 선택과 단일 참여자 undo 경계를 유지합니다.
- [x] 신규 editor 검사 1건 0.00초 통과입니다. CJK/보조문자/combining mark/CRLF 왕복, 경계 보정, 잘못된 scalar 위치·stale·overlap·역순·용량 거절과 batch 실패 시 body/revision 보존, shared selection·undo/redo를 확인했습니다.
- [x] 대상 strict clippy: editor lib/신규 lsp 검사 exit 0(2.78초), 제품 taide-lsp lib exit 0(4.15초), 실제 검사 lib/mock-server/native-session exit 0(6.28초). 새 의존성 경로를 포함한 실제 app lib/bin check exit 0(1.19초), 변경 파일 rustfmt/diff check exit 0입니다.

검증 명령은 각 manifest에 `--locked --offline --target-dir experiments/native-shell-spike/target`을 사용했습니다. 실제 프로토콜 검사는 `cargo test --manifest-path experiments/lsp-coordinator-spike/Cargo.toml --test native-session native_actor는_최신`, editor 검사는 `cargo test --manifest-path native/taide-native-editor/Cargo.toml --test lsp`입니다. 최초 프로토콜 target 의존성 컴파일은 28.99초였고 검사 본체는 0.35초입니다.

## 의존성과 환경 보존

- Native editor의 독립 lock 갱신 때 발생한 무관한 transitive 버전 이동은 기존 app lock의 버전으로 복원했습니다. 최종 양 lock의 공통 package 버전 비교에는 차이가 없습니다. app/UI lock은 editor의 기존 `lsp-types` 의존 경로만 반영합니다. 제품 root lock의 이 단계 추가 package는 없습니다.
- 실행 환경 Rust 1.98.1에서 검증했으며 root MSRV 1.89/native 선언 1.95의 실제 compiler 검증을 주장하지 않습니다.
- 사용자 실기 앱·입력기·VoiceOver·실제 GUI는 실행/변경하지 않았습니다. commit/push는 M8 전체 완료 뒤입니다.

## 남은 연결

- [ ] 실제 app의 server discovery/root 선택·문서 refcount·open/change/close/save lifecycle 소비자를 연결해야 합니다. actor에 메서드가 생겼다고 GUI composition을 완료로 세지 않습니다.
- [ ] formatter·명시적 fixAll/organizeImports·resolve/executeCommand와 server ApplyEdit의 실제 root 승인·다른 문서 batch를 연결해야 합니다. 현재 batch adapter만으로 전체 저장 파이프라인을 완료로 세지 않습니다.
- [ ] 실제 서버가 surrogate 한가운데 편집을 보낼 때의 정책은 추가 확인 대상입니다. 현재 native는 scalar를 쪼개지 않고 거절합니다. Monaco의 range 자동 보정까지 동등하다고 주장하지 않습니다.
- [ ] native syntax/전체 UI·성능·메모리·GPU·실기 IME/VoiceOver·cutover와 M8 상위 게이트는 계속 미완료입니다.
