# M8 native 탭 닫기·파일 초안 복원 첫 연결

## 대상과 현재 상태

- `native/taide-native-app/src/{application,close_dialog,host,tabs,lib}.rs`, `tests/{tabs,host}.rs`, 독립 manifest/lock
- `native/taide-native-editor/src/store.rs`, `tests/store.rs`
- `native/taide-native-ui/src/{document_admission,editor_surface}.rs`, `tests/editor_surface.rs`, 독립 manifest/lock

N2-A2c의 첫 구현입니다. 파일 탭 닫기의 저장·폐기·취소와 기존 파일의 canonical 초안 복원을 실제 host/store에 연결했습니다. 삭제된 원본 Save As·untitled/AppFile·전체 충돌 배너/외부 변경·저장 pipeline·모든 탭 close 진입점·실제 GUI 동등성은 남아 있으므로 N2-A2c/N2/M8 전체 완료가 아닙니다.

## 원본과 구현 근거

기존 `src/widgets/editor-area/use-request-close-tab.tsx`, `src/features/tab/close-dirty-tab-dialog.tsx`, `src/widgets/editor-pane/use-editor-file-persistence.ts`, `src/features/editor/conflict-banner.tsx`, 기존 Rust layout/file/terminal/IDE service와 고정 egui 0.36.2 Modal source를 확인했습니다.

1. native 앱의 `RequestCloseTab`을 하나의 pending close 상태로 연결했습니다. locale catalog의 원본 확인 제목·설명·저장/폐기/취소 버튼을 사용합니다. Cancel/Escape는 닫기 mutation을 보내지 않고, pinned는 UI와 실제 guard 내부에서 모두 거절합니다. Save는 immutable snapshot의 실제 파일 쓰기와 canonical baseline 완료 후 root dirty 표시를 먼저 보내고 close를 제출합니다. 실패·read-only·새 편집 때문에 snapshot 완료가 clean이 아닌 경우 탭을 닫지 않습니다.
2. 정상 close와 explicit discard는 등록된 TaskSupervisor operation·owned mutation guard·blocking worker에서 처리합니다. layout을 먼저 복제해 close 가능 여부를 확인하고, mirror 정리 실패라면 in-memory layout을 commit하지 않습니다. discard는 닫힌 기록에 ghost dirty가 남지 않도록 먼저 dirty를 해제합니다. terminal은 기존 retirement store에 넘기고 ClaudeDiff는 기존 pending responder에 TabClosed를 전달합니다. 실제 PTY/IDE와 연결한 close 실기는 아직 하지 않았습니다.
3. 주 창·auxiliary layout 전체에서 canonical file이 남아 있는지 확인합니다. 하나만 닫았으면 공유 문서·mirror·다른 탭의 dirty를 유지합니다. 마지막 tab의 성공 완료에서만 해당 view들을 detach하고 clean document를 release하거나, 사용자가 승인한 document/revision의 dirty document를 discard합니다. 확인 이후 revision이 달라진 경우 core discard는 거절합니다. 아직 모든 preview 교체·pane 이동·global view eviction 경로를 연결한 것은 아닙니다.
4. egui native editor는 비활성 UI에서 focus 요청·text/IME 입력 소비·scroll 소비·IME 출력 요청을 하지 않습니다. 확인 창 뒤에서 입력을 계속 편집하는 경로를 막았습니다. 현재 pending close 동안 앱 body 입력을 비활성화하는 첫 정책이며 원본의 모든 비동기 modal·다중 탭 close·키맵/UX 동등성은 후속입니다.
5. file admission의 기존 owned guard/worker 안에서 root가 허용하는 canonical mirror를 읽습니다. 신규 document만 disk Rope를 baseline으로 유지하면서 mirror Rope를 body로 복원합니다. conflict인 mirror도 원본처럼 body를 복원하고 conflict 상태를 반환하며, 이미 존재하는 공유 document는 오래된 mirror로 덮지 않습니다. 복원 본문이 cap을 초과하면 document를 입장시키지 않고 mirror는 보존합니다. 정상 save는 기존 file writer가 mirror를 정리한 뒤 canonical 저장 snapshot을 baseline으로 채택합니다. 복원 notice는 현재 locale 상태 메시지에 표시하며 원본의 배너/보기-디스크/내 것 유지 UI 전체 연결은 남습니다.

미장착 dirty file의 close-save fallback은 기존 mirror를 사용합니다. source_missing/conflict mirror는 resolve/Save As가 구현되기 전 명시적으로 거절합니다. 이는 원본의 unmounted fallback 직접 저장보다 엄격한 임시 상태이며 최종 parity에서 해결해야 합니다. untitled Save As도 아직 미연결 오류입니다. 삭제된 원본은 파일 admission 자체가 실패하므로 missing-source draft 표면을 다음 단계에서 구현해야 합니다.

`taide-file` 직접 edge는 이미 runtime graph에 있는 기존 crate를 native app/UI에서 재사용합니다. 새 외부 package·root 제품 manifest/MSRV·Tauri adapter는 이번 변경으로 바꾸지 않았습니다. 격리 native 후보의 최종 제품 채택을 의미하지 않습니다.

## 검증

Cargo는 모두 `--offline --target-dir experiments/native-shell-spike/target`을 사용했습니다. 독립 manifest의 직접 edge 갱신 외에는 `--locked`를 사용합니다. GUI 프로세스·OS dialog·secret store·사용자 shell·실기 bundle은 실행하거나 변경하지 않았습니다.

- [x] `cargo test --manifest-path native/taide-native-app/Cargo.toml --test tabs`: 2 passed/0 failed, 0.04초입니다. 실제 host의 pinned/dirty 거절, 주 창과 aux의 공유 mirror 보존, explicit discard의 clean closed history, mirror save의 실제 디스크 쓰기 후 close/정리와 operation 회수를 확인했습니다. missing-source save 실패는 탭/mirror를 남기며 실제 egui Modal의 Escape는 Cancel을 반환합니다. 신규 test의 초기 locale DTO 구성 오류는 실제 locale resolver 호출로 수정했습니다.
- [x] `cargo test --manifest-path native/taide-native-editor/Cargo.toml --test store 명시적_폐기`: 1 passed/0 failed, 0.00초입니다. attached/shared view와 stale revision에서는 dirty document를 유지하고, 승인한 마지막 view 분리 후에만 identity map까지 회수합니다.
- [x] `cargo test --manifest-path native/taide-native-ui/Cargo.toml --test editor_surface 비활성_editor`: 1 passed/0 failed, 0.01초입니다. 실제 egui frame에서 비활성 editor가 text/IME commit을 소비하거나 body/revision을 바꾸지 않습니다. 최초 fixture signature 오류는 기존 fixture에 맞춰 수정했고 영향 범위만 실행했습니다.
- [x] `cargo test --manifest-path native/taide-native-app/Cargo.toml --test host native_host`: 2 passed/0 failed, 0.04초입니다. 실제 파일의 결정적인 mtime 변경으로 conflict mirror 복원·disk baseline·canonical alias/live edit 보존·실제 저장/mirror 제거를 확인했습니다. 변경된 기존 host는 이미 살아 있는 canonical document를 mirror로 덮지 않고 token drop/worker 회수도 확인합니다. 신규 호출의 초기 editing 인수 누락을 실제 함수에 맞춰 고쳤습니다. 관련 없는 CLI/layout-flush 검사는 앞선 증거를 재사용했습니다.
- [x] `cargo test --manifest-path native/taide-native-editor/Cargo.toml --test store mirror_복원`: 1 passed/0 failed, 0.00초입니다. 신규 draft의 용량 실패는 빈 store를 유지하고, 복원 본문은 disk와 별개로 dirty이며, 이미 살아 있는 canonical body를 오래된 mirror로 다시 초기화하지 않습니다. 앞선 잘못된 filter로 실행된 0 tests는 통과 증거로 계산하지 않았습니다.

대상 strict clippy는 app의 `--lib --bin taide-native-app --test tabs --test host` exit 0(0.65초), editor의 `--lib --test store` exit 0(1.85초), UI의 `--lib --test editor_surface --test document_admission` exit 0(4.44초)입니다. 변경 파일 rustfmt check와 `git diff --check`도 exit 0입니다. 컴파일 중 미연결 신규 reply arm은 실제 앱 handler로 연결한 뒤 check exit 0(0.59초)이고, 후속 mirror 연결 check exit 0(0.76초)입니다. 동일 변경 상태의 성공 검사를 다시 계측하지 않았습니다.

## 다음 필수 연결

- [ ] missing-source/untitled의 실제 Save As, canonical rekey·기존 target 공유 문서와 충돌·실제 경로/layout/mirror 완료 경계
- [ ] 원본 conflict banner·view disk/keep mine/dismiss·외부 파일 변경/삭제/rename와 staged draft epoch, autosave/주기적 mirror·저장 cleanup/format/code actions
- [ ] close-all/others/right/saved·키보드/middle click/context menu와 한 번의 다중 dirty 확인, preview/pane 이동의 view 회수·실제 다중 창 close
- [ ] 3 locale/theme·focus/AX·실제 GUI pixel/OS dialog/IME/VoiceOver, 전체 queue/memory·정상 project close의 draft handshake

M8 상위 N1~N8은 계속 미완료입니다. 이번 범위의 관찰을 213 TS view 동등성·Rust99%·최종 배포/Git 완료로 확대하지 않습니다. 사용자 실기 앱·OS 입력기·VoiceOver 설정은 그대로 두었습니다.
