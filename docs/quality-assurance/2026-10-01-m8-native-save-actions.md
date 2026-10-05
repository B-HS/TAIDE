# M8 native 저장 코드 액션 연결

## 대상·원본

- 실제 app: `native/taide-native-app/src/{lsp,lsp_workspace,application}.rs`, `tests/{lsp,lsp-workspace}.rs`.
- 합성 실제 서버: `experiments/lsp-coordinator-spike/src/bin/mock-server.rs`의 `--native-actions`.
- 원본: `src/widgets/editor-pane/{use-editor-file-persistence,use-editor-lsp-integration}.ts`, `src/shared/lib/lsp/{adapters/code-action,command-relay,workspace-edit-applier}.ts`.
- [공식 LSP meta model](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/metaModel/metaModel.json)의 CodeAction/resolve와 edit-before-command 계약, 설치된 `lsp-types` 0.97.0의 실제 타입을 사용합니다.

## 구현한 저장 경로

- [x] app의 explicit Save에서만 설정에 따라 fixAll/organizeImports를 활성화하고 auto-save에서는 비활성화합니다. `format_on_save`가 꺼져도 코드 액션 참여자를 실행할 수 있습니다. 기존 저장 epoch/최종 cleanup/파일 worker를 그대로 사용합니다.
- [x] 서버별로 fixAll → organizeImports 순서이며 서버 진단·전체 UTF-16 문서 range·Automatic trigger·단일 `only` kind를 전달합니다. bare Command는 보존하고 CodeAction kind는 exact 또는 점으로 구분된 하위 kind만 허용합니다.
- [x] edit가 없는 액션의 resolve를 기존 capability guard로 시도하고 실패 시 원본 액션을 유지합니다. edit 적용 실패 시 후속 command를 실행하지 않습니다. 기존 typed SessionClient를 통해 요청하고 임의 OS 명령을 직접 실행하지 않습니다.
- [x] GUI edit acknowledgement → registry의 실제 문서 sync 완료 → 후속 command 순서를 지킵니다. 후속 명령에서 server `workspace/applyEdit`가 발생해도 독립 worker가 GUI 결과를 기다려 typed ApplyWorkspaceEditResponse를 반환합니다. registry는 이 대기 동안 다른 요청·sync를 처리합니다.
- [x] 총 코드 액션 대기는 원본처럼 5초 상한이며 시간 초과 시 포맷/저장이 계속 진행하도록 연결했습니다. 이미 실행 중인 액션 task는 timeout으로 취소하지 않습니다. 이후 edit는 현재 revision 검사와 dirty/persistence 경로를 거칩니다. 실제 timeout/late-command 왕복은 이번 검사에서 실행하지 않았습니다.

## 현재 WorkspaceEdit 지원 경계

- [x] 현재 LSP session에 연결돼 알려진 열린 문서의 `changes`/versioned `documentChanges`/AnnotatedTextEdit를 실제 EditorStore transaction에 적용합니다. documentChanges를 우선하며 한 문서의 여러 순차 edit operation과 각각의 undo 경계를 유지합니다.
- [x] protocol version과 GUI snapshot key/revision을 별도로 검증합니다. stale version/late GUI edit를 거절합니다. failure 전 이미 적용된 문서의 dirty/persistence 변경을 잃지 않으며 전체 workspace 원자적 rollback을 주장하지 않습니다. 최초 열린 문서 전용 경계 이후의 background/unopened 연결은 `2026-10-01-m8-native-workspace-text-edits.md`에 기록합니다.
- [ ] rename, root별 전체 workspace 승인·mirror/tab 경로 갱신과 dormant mirror admission은 구현해야 합니다. CreateFile의 연결·검증은 후속 workspace-text-edits QA, DeleteFile·회수 연결은 workspace-delete QA에 기록합니다. 연결된 텍스트 편집·CreateFile/DeleteFile worker를 전체 workspace-edit 기능의 대체나 완료로 세지 않습니다. 광고한 workspaceEdit에 resourceOperations는 넣지 않았습니다.
- [ ] server commands는 해당 SessionClient를 사용합니다. 원본 process-wide command registry의 중복 id 선택·client-only navigation command·실제 모든 provider와 동일하다고 주장하지 않습니다.

## 직접 검사

공통 flags는 `--locked --offline --target-dir experiments/native-shell-spike/target`입니다. 동일 상태의 성공 검사는 재사용합니다.

- [x] `cargo build --manifest-path native/taide-native-app/Cargo.toml --example native-lsp-mock` exit 0, 2.76초. fixture의 native-actions mode만 추가하고 기존 modes의 계약은 유지했습니다.
- [x] `cargo test --manifest-path native/taide-native-app/Cargo.toml --test lsp 실제_저장은` 신규 1건 통과, 본체 0.16초. actual discovery/root/runtime/actor → server 진단 → kind filter → resolve → versioned edit → command → server applyEdit → organizeImports → format → cleanup → 실제 host 파일 저장 → worker 회수/tracked count 0을 확인했습니다. 최종 디스크 내용은 `formatted:imports:command:fixed:文😀 ` 뒤 LF입니다. 실제 egui 앱/키보드 Cmd-S 검사는 아닙니다.
- [x] 위 검사의 최초 실행은 편집 3회 중 1회만 관찰돼 실패했습니다. GUI ack 뒤 후속 command가 비동기 discovery/sync보다 먼저 서버에 전달되는 제품 순서 오류를 확인했습니다. command 전에 registry FIFO sync 완료를 기다리도록 수정하고 실패한 검사만 1회 재실행했습니다. mock의 expected edit-before-command를 완화하지 않았습니다.
- [x] `cargo test --manifest-path native/taide-native-app/Cargo.toml --test lsp-workspace` 신규 1건 통과, 0.00초. documentChanges 우선/annotated edit/동일 문서 순차 operation/두 undo·stale protocol version·unknown document·resource 거절·late GUI edit와 거절 시 body/revision 보존을 확인했습니다. malformed scalar/overlap/capacity는 이전 editor LSP batch 검사 결과를 재사용합니다.
- [x] app lib/bin/LSP integration/mock example strict clippy exit 0, 0.87초. 이후 admission 실패가 registry를 종료하거나 저장 응답을 유실하지 않도록 수정했고 최종 app/lib/bin/workspace 검사 strict clippy exit 0, 0.26초입니다. 두 신규 test 이름의 snake_case 경고는 이름을 바로잡았으며 검사기를 끄지 않았습니다. 대상 rustfmt/QA Prettier/diff check exit 0입니다.

## 남은 M8 검증·구현

- [ ] 실제 egui 저장 설정/explicit-auto 분기/닫기/강제 Exit, 5초 timeout 이후 late action·서버 실패·admission 실패·mirror/auto-save 재진입을 직접 검증해야 합니다. GUI 설정과 사용자 실기 앱은 이번 단계에서 조작하지 않았습니다.
- [ ] multi-root shares_sessions·모든 provider·진단/status/progress UI·indentation guessing·실제 syntax/EOL parity와 전체 queue byte/RSS·성능을 구현/검증해야 합니다.
- [ ] N1~N8, 213개 TS view 동등성·Rust 99%·최종 TS 제거·서명/공증/install/upgrade는 계속 미완료이며 commit/push는 전체 M8 완료 뒤입니다.
