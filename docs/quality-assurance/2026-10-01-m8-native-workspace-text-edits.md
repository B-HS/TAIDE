# M8 native WorkspaceEdit 파일·버퍼 연결

## 대상·원본

- 대상: `native/taide-native-app/src/{lsp_workspace_worker,lsp_workspace,lsp,application}.rs`, `tests/lsp-workspace-worker.rs`, `crates/taide-file/src/service.rs`의 `has_mirror`.
- 원본: `src/shared/lib/lsp/workspace-edit-applier.ts`의 열린 모델 편집·미열림 파일 read/edit/save·순차 operation 계약과 기존 root/file service입니다.
- 저장 코드 액션의 선행 근거는 `2026-10-01-m8-native-save-actions.md`에 있습니다. 이 문서는 그 이후 변경 상태를 기록합니다.

## 연결한 경로

- [x] GUI에 canonical 경로로 기존 DocumentSnapshot을 질의합니다. 비활성 문서도 기존 버퍼·dirty·undo 경로로 편집하며 디스크를 직접 덮어쓰지 않습니다. LSP에 알려진 문서는 요청 snapshot과 protocol version을 보존합니다.
- [x] 미열림 파일은 tracked blocking worker에서 기존 file-size/lossy/read-only 정책으로 읽고 indexed UTF-16 transaction을 적용한 뒤 기존 원자적 저장·self-write 경로로 저장합니다. owned mutation guard와 operation을 읽기부터 저장까지 유지합니다.
- [x] 설치된 `lsp-types`의 URI 공개 API로 로컬 file URI를 엄격한 UTF-8로 디코딩합니다. 원격 authority·다른 scheme·query·fragment·NUL·상대 경로와 승인 root 밖 경로를 거절합니다. 신규 의존성을 추가하지 않았습니다.
- [x] documentChanges를 우선하고 첫 실패에서 중단합니다. 이미 성공한 앞선 operation의 전체 rollback을 주장하지 않습니다. 실제 app의 code-action edit와 server applyEdit 모두 이 worker를 사용합니다.
- [x] CreateFile은 기존 file service의 생성 또는 원자적 빈 내용 저장을 사용합니다. overwrite가 ignoreIfExists보다 우선하며 기존 파일 버퍼를 저장 처리하거나 덮어쓰지 않습니다. 원본처럼 명시적 overwrite의 디스크·owning mirror 변경과 기존 dirty GUI 버퍼는 구분합니다. 일반 create는 open-project 경계이며 overwrite/save에만 기존 CLI 단일 파일 승인을 적용합니다.
- [ ] 복원 대기 mirror가 있는데 GUI 버퍼가 없으면 현재 저장을 거절해 초안을 보존합니다. 원본 미열림 경로와 완전히 동일한 정책으로 계산하지 않습니다. mirror admission·실제 화면·경로 변경을 연결한 뒤 이 보수적 거절을 해소해야 합니다.

## 직접 검증

공통 flags는 `--locked --offline --target-dir experiments/native-shell-spike/target`입니다.

- [x] `cargo test --manifest-path native/taide-native-app/Cargo.toml --test lsp-workspace-worker` 신규 1건 통과, 본체 0.03초. 실제 AppServices/TaskSupervisor에서 CJK·공백·emoji 파일명, UTF-16 편집·CRLF 보존·미열림 디스크 저장, dirty 비활성 버퍼 편집과 디스크 불변, lossy 거절, dormant mirror 보존, server root 거절, URI 오류와 worker 회수/tracked count 0을 확인했습니다.
- [x] 변경된 worker를 사용하는 실제 저장 통합 `--test lsp 실제_저장은` 1건 통과, 0.39초. 기존 저장 액션 검사를 같은 상태에서 반복한 것이 아니라 새 파일·GUI query/ack 경로 연결 뒤 영향 검사를 한 번 실행했습니다.
- [x] lib/bin 및 `lsp`·`lsp-workspace`·`lsp-workspace-worker` 대상 strict clippy exit 0, 1.14초입니다. 최초 URI 컴파일 오류는 공개 Path API의 `as_estr()` 경로로 수정했고, 합성 worker fixture의 Result-returning future는 실제 supervisor의 unit future 계약에 맞춰 oneshot 결과 채널로 수정했습니다. test 이름만 snake_case로 바로잡았으며 같은 성공 검사는 반복하지 않았습니다.
- [x] `--test lsp-workspace-worker 실제_workspace_create` 신규 1건 0.05초 통과입니다. 실제 nested create→edit/CRLF·옵션 우선순위·파일 mode 보존·disk truncate/mirror 정리와 dirty 버퍼 불변·첫 실패의 prefix 유지/후속 중단·CLI 분기·server root 제한·symlink escape 거절과 worker 회수를 확인했습니다. 최초 이름 필터 입력 오류는 0건 실행이므로 성공 근거로 세지 않았고 정확한 신규 검사만 실행했습니다. create 연결 뒤 대상 app/workspace-worker strict clippy exit 0, 0.59초입니다.

## 남은 범위

- [ ] rename과 mirror·tab·DocumentStore 경로 이동, live multi-root session 승인, 반복 versioned operation의 실제 sync 버전 갱신을 연결해야 합니다. 경로 전환 core/LSP 검사는 `2026-10-01-m8-native-file-path-retarget.md`, 후속 DeleteFile·문서 회수 연결과 실제 검사는 `2026-10-01-m8-native-workspace-delete.md`에 기록합니다. resourceOperations capability는 아직 광고하지 않습니다.
- [ ] GUI query 대기·실제 timeout/late command/Exit·대형 응답 메모리·외부 프로세스의 파일 교체 TOCTOU·실제 GUI를 검증해야 합니다. synthetic worker 결과를 이 범위의 완료 근거로 확대하지 않습니다.
- [ ] N1~N8·213개 view·Rust 99%·TS 제거·서명/공증/install/upgrade와 최종 commit/push는 미완료입니다. 사용자 실기 bundle과 시스템 설정은 유지했습니다.
