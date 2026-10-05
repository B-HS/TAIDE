# M8 native app LSP 연결

## 대상과 원본

- 실제 app: `native/taide-native-app/src/{lsp,application}.rs`, `tests/lsp.rs`.
- 기존 제품 actor: `crates/taide-lsp/src/native/session.rs`, `taide-runtime::native_lsp_actions`, 기존 discovery/process/root guard/TaskSupervisor/LspStore.
- 합성 서버: `experiments/lsp-coordinator-spike/src/bin/mock-server.rs`의 `--native-document`, native app의 `native-lsp-mock` example.
- 원본: `src/widgets/editor-pane/{use-lsp-session,use-editor-lsp-integration,use-editor-file-persistence,code-editor-visibility}.ts`, `src/shared/lib/lsp/server-request-handler-registry.ts`와 native shell의 실제 pane/slot 표시 경계.

## 연결된 범위

- [x] 문서별 canonical file/root 승인, 설치·언어 일치 서버 discovery와 기존 process config를 재사용합니다. Normal tier만 연결하며 CLI 외부 파일은 app에서 제외하고 worker도 프로젝트 밖 경로를 거절합니다.
- [x] 같은 project/server/root의 여러 문서가 한 actor/process를 공유합니다. 중복 view를 DocumentId로 통합하고 동일 revision 재전송을 생략합니다. GUI revision과 프로토콜의 연속 revision을 분리해 background tick 사이 여러 편집도 전송합니다.
- [x] 실제 shell의 활성 pane tab/표시 slot/Zen/창 scope를 따라 연결합니다. 비활성 탭·숨겨진 프로젝트·missing draft·loading/error 화면은 연결하지 않습니다. 표시 문서가 사라지면 didClose, 마지막 문서가 사라지면 현재 구현의 actor stop/join으로 회수합니다.
- [x] 저장 전 formatter는 별도 tracked task에서 대기해 registry가 configuration 등 서버 요청에 계속 응답합니다. 실제 typed TextEdit를 revision/key guard와 기존 GUI transaction에 적용하고, 저장 epoch 동안 중복 저장·자동 저장 재진입을 막습니다.
- [x] 원본의 formatter → trim/final newline → 최종 snapshot → 디스크 쓰기 → didSave 순서를 연결합니다. 포맷 실패는 상태에 알리고 저장을 계속하며 late reply는 최신 편집을 덮어쓰지 않습니다. 실제 syntax token provider가 없는 언어의 trim 제한은 기존 save-cleanup QA 그대로입니다.
- [x] 실제 root initialize/options와 source의 null configuration 응답을 사용합니다. 현재 subset만 capability로 광고하고 진단은 generation/문서 revision 확인 후 보관합니다. 진단 렌더링은 아직 없습니다.
- [x] 정상 종료·직접 Exit는 host/LSP worker의 회수를 모두 기다립니다. 종료 전 mirror/layout 실패로 앱에 돌아오면 host와 LSP를 재연결하도록 코드에 연결했습니다. 실제 OS Exit 실패 복구 실기는 미검증입니다.

## 검사와 관찰

공통 flags는 `--locked --offline --target-dir experiments/native-shell-spike/target`입니다. 성공한 동일 검사는 반복하지 않았습니다.

- [x] 실제 app integration: `cargo test --manifest-path native/taide-native-app/Cargo.toml --test lsp 실제_native_app_lsp는` — 1건 통과, 본체 0.59초. canonical 파일·실제 발견된 합성 executable·기존 runtime/session을 사용해 initialize/configuration·CJK/emoji formatting·공유 PID·change/save·late edit·close/남은 문서·미등록 root 거절·worker 회수/tracked count 0을 연속 확인했습니다. 이는 실제 egui 앱 실행 검사가 아닙니다.
- [x] 표시 수명: 같은 manifest/test의 `실제_shell` filter — 1건 통과, 0.00초. inactive/settings 뒤 file·두 프로젝트 slot·Zen focus·auxiliary scope·빈 project 목록을 확인했습니다.
- [x] 초기화 실패 대기: `cargo test --manifest-path crates/taide-lsp/Cargo.toml --lib running_대기는` — 1건 통과, 0.00초. Running 대기의 Degraded/Stopping/Stopped 즉시 실패와 Stopped join 대기의 유지가 확인됐습니다.
- [x] 별도 attach 정책: 같은 manifest/test의 `실제_native_lsp_연결_정책` filter — 1건 통과, 0.00초. 합성 snapshot의 Large/ReadOnly/Refused tier는 session/formatter가 없고, 열려 있는 프로젝트 밖 canonical 경로는 Forbidden으로 거절됩니다. 최초 검사는 localized AppError의 내부 variant를 직접 비교해 실패했고, 기존 `kind()` 분류 계약을 확인해 assertion을 바로잡았습니다. 경로 승인 구현은 완화하지 않았습니다.
- [x] app lib/bin/actual integration/mock example strict clippy exit 0(1.27초), 제품 LSP lib strict clippy exit 0(0.33초). 추가 attach 정책 test strict clippy exit 0(0.30초)입니다. 대상 app rustfmt, 제품 actor rustfmt, 새 QA Prettier, diff check exit 0입니다.

최초 integration은 first format 대기에서 5초 timeout이 발생했습니다. 합성 서버가 기존 실험의 null root만 허용해 실제 app의 유효한 프로젝트 root initialize를 거절한 것이 원인이었습니다. native-document fixture에서 실제 rootUri/rootPath/workspaceFolders 일치를 검증하도록 고쳤으며 app의 root를 null로 우회하지 않았습니다. 이 실패에서 발견한 제품 actor의 Degraded 이후 무한 Running 대기도 terminal phase에 즉시 오류를 반환하도록 수정했습니다. 성공 결과를 세 번 반복한 것이 아닙니다.

## 미완료와 다음 구현

- [ ] `shares_sessions: true` 서버의 다른 root workspace 공유/등록, 원본 session 보존·지연 해제 정책과 서버 설치·언어 변경 시 재발견을 구현해야 합니다. 현재 같은 root 공유만 완료입니다. revision마다 discovery를 실행하는 비용도 남습니다.
- [ ] 모든 editor provider·format provider 선택 UI와 전체 workspace-edit/root/resource 승인이 남습니다. 열린 문서의 fixAll/organizeImports·resolve/executeCommand·server ApplyEdit 경로는 후속 `2026-10-01-m8-native-save-actions.md`의 직접 증거로 연결됐으며 이를 전체 저장/LSP 완료로 세지 않습니다.
- [ ] 진단 표시·status/log/progress/show-message UI와 indentation guessing의 원본 연결이 남습니다. 광고하지 않은 provider를 구현됐다고 주장하지 않습니다.
- [ ] 실제 egui 저장/Close/Exit와 OS 실패 복구·여러 창·실제 LSP 서버·syntax provider·장기 RSS/queue byte 총량/성능·root TOCTOU를 확인해야 합니다. 이번 본체 검사는 합성 소형 문서이며 대형 파일 메모리 검사가 아닙니다.
- [ ] M8 N1~N8, TS view 213개 대응·Rust 99%·최종 cutover·서명/공증/install/upgrade는 계속 미완료입니다. 사용자 실기 bundle·입력기·VoiceOver는 변경하지 않았습니다. commit/push는 M8 전체 완료 뒤입니다.
