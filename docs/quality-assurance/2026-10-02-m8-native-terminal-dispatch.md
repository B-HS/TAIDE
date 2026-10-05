# M8 native terminal의 borrowed metadata·agent·query 소비

## 대상·상태

`native/taide-native-app/src/terminal_dispatch.rs`, `tests/terminal-dispatch.rs`, app lib와 native terminal lib의 typed Event/Rgb/WindowSize 재수출, `crates/taide-agent/src/{service,store}.rs`입니다. native Dispatcher는 기존 AppServices metadata/events/agents·TerminalCommandClock와 bounded writer를 사용합니다. 실제 root session registry/spawn/close composition·제품 query palette/geometry/UI ports는 아직 연결 전이며 전체 N4/M8는 미완료입니다.

## 원본 정책·소유

- accepted Delivery의 단조 revision을 한 번만 소비합니다. 소비 시작 때 실패 latch를 세우고 모든 effect의 성공 뒤에만 해제합니다. IO 실패·port 오류·future 취소 뒤 부분 처리된 metadata/agent/UI/query를 재실행하지 않습니다. actor는 이 실패를 native Core/PTY stop에 연결해야 하며 Dispatcher 단독이 child를 중단한다고 주장하지 않습니다.
- chunk의 마지막 cwd를 먼저 metadata에 적용하고 실제 변화에서만 TerminalCwdChanged를 게시합니다. 그 다음 모든 command marker를 기존 clock에 기록하며 완료 event는 최신 cwd를 사용합니다. timestamp는 queue Delivery에 기록한 관찰 Instant이고 actor 처리 지연 시각으로 바꾸지 않습니다.
- 같은 Outcome의 stream effect iterator·text·overlap를 borrowed 상태로 기존 AgentStore에 전달합니다. `apply_scan_to_signals`의 본문 정책을 `apply_scan_parts_to_signals`에 유지하고 기존 API는 그 함수로 위임합니다. 기존 blocked event→substantive text→dialog priority·echo·overlap 규칙은 바꾸지 않았습니다. native caller는 두 번째 scanner나 출력/event Vec/String 사본을 만들지 않습니다. agent store는 기존대로 미감지 session의 signal record를 만들지 않습니다.
- typed native color/pixel geometry query와 PtyWrite를 같은 bounded writer로 순서대로 보내고 실제 receipt를 await합니다. 이때 borrowed Delivery가 계속 permit을 보유합니다. legacy opaque callback/child-exit·clipboard effect는 명시적으로 거절합니다. clipboard를 읽거나 OSC52 권한을 추가하지 않았습니다. stream/UI event ports도 borrowed effect를 받습니다. 실제 palette·pixel geometry와 title/bell/notification UI policy는 composition에서 supplied port를 연결해야 합니다.
- query response/metadata/event의 추가 allocations, callback closure·AgentStore 전체/전역 memory·Core와 queue/writer의 합산 예산은 별도 남습니다. 원본 agent process discovery/poll·input/output의 통합 actor 순서·spawn env provider·마지막 join Frame·다중 view/resize/sync deadline·GUI parity를 이 좁은 consumer 성공으로 완료 처리하지 않습니다.

## 실제 검증

모든 Cargo는 직렬·locked·offline·공유 target입니다. 합성 AppState metadata/memory writer만 사용하며 데이터 디렉터리를 만들거나 keychain/clipboard/OS 앱을 호출하지 않았습니다. 기존 보호 bundle·root MSRV·원본 Tauri caller의 signature와 product scanner는 유지합니다.

1. [x] app `cargo check … --lib`: exit 0(1.60초).
2. [x] app `cargo test … --test terminal-dispatch -- --nocapture`: 신규 2 PASS, compile 4.46초/suite 0.00초. actual Core의 latest-cwd collapse→현재 cwd의 5,500ms command 완료 event 순서, 오래된 관찰 시각의 agent Idle, color/geometry의 정확한 reply byte·writer 순서, duplicate 거절과 query IO 실패의 단일 시도/partial event 재실행 거절·tracked_count 0을 확인했습니다.
3. [x] app `cargo clippy … --lib --test terminal-dispatch -- -D warnings`: 최종 exit 0(0.47초). 첫 검사 type_complexity는 두 effect sink의 공용 typed boundary alias로 수정했습니다. 다음 test 검사 await_holding_lock은 explicit drop 대신 검증 block의 lexical scope로 해결했습니다. suppression·async mutex 추가 없이 수정했으며 runtime 성공은 반복하지 않았습니다. inherited Wry 17 warnings는 유지됩니다.
4. [x] root `cargo test -p taide-agent --lib … 래치 -- --nocapture`: 영향받은 기존 정책 16 PASS, compile 1.81초/suite 0.00초. same-chunk permission/tool/실질 출력, 에코 창·input·여러 agent·원본 scanner의 split/overlap/점멸 latch를 확인했습니다. 테스트 이름의 실물 PTY 바이트는 고정 raw fixture를 뜻하며 이 검사 자체의 실제 OS PTY 실행으로 확대하지 않습니다.
5. [x] root `cargo clippy -p taide-agent --lib … -- -D warnings`: exit 0(0.43초). exact app/core fmt·root agent exact fmt와 `git diff --check`: exit 0.

다음은 단일 native session registry·기존 spawn lease/TaskSupervisor·정상/취소/닫힌 project/최종 Frame과 Core 실패/PTY 회수·HostIntent/surface 연결입니다. 전체 M8 완료 뒤만 commit·일반 push합니다.
