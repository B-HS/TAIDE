# M8 production remote LSP process adapter

## 대상과 구현

대상은 `native/taide-native-app/src/lsp-process.rs`, `lsp-process-tests.rs`, `remote-lsp.rs`, `lib.rs`입니다. 원본 Tauri `src-tauri/src/domain/lsp/commands.rs::create_process/handle_process_exit`를 실제 AppServices/EventSink/TaskSupervisor 기반으로 연결했습니다. remote Ports::new/default는 실제 process factory와 PATH provider입니다. fixture/no-op factory가 아닙니다.

- 실제 공유 spawn_language_server의 command/root/managed resolver와 framing·bounded stdout/stderr·writer/owned child를 사용합니다. root LspStore가 실제 생성·재시작된 모든 프로세스를 소유합니다.
- message의 active epoch·subscribers broadcast, 종료의 감독 task/변경 guard·stopped/superseded gate, restart_backoff_delay의 원본3회 상한·500ms 배수, Starting 오류·재생성/새 epoch·generation 증가·재초기화 전 Crashed 상태를 보존합니다. 임의 Running 성공을 만들지 않습니다.
- 원본30초 healthy window·동일 process slot/아직 살아 있음 확인 뒤 restart count reset과 오류 경로를 연결합니다. stderr tail은 기존 known-secret masking 뒤 한 줄로 만듭니다.
- proc callback은 AppServices를 weak로 보유합니다. transient recovery만 실행 중 strong owner를 보유하며 root shutdown/TaskSupervisor가 회수합니다. native editor SessionClient의 복구·화면 handshake와 App의 full dispatcher 조립은 아직 미완료입니다.

## 최소 검증

- [x] 첫 exact `--lib lsp_process::tests`: compile9.33초/suite5.02초, 정책1 PASS·통신1 timeout입니다. 신규 fixture initialize에 요구되는 processId/rootUri/capabilities가 누락돼 합성 서버가 종료했습니다. 실제 mock source를 읽고 필드를 정정했으며 제품 spawn/프로토콜은 변경하지 않았습니다.
- [x] 정확한 실패 통신1만 재실행: compile6.37초/suite0.54초·PASS입니다. 실제 기존 native-lsp-mock 프로세스의 initialize 응답→kill→실제500ms backoff→새 프로세스/epoch/generation1/재초기화 전 Crashed→새 응답/실제 confirm-reinitialize/Running을 확인했습니다. 이전 epoch 종료 무시, shutdown 후 프로세스 finished·감독 task0·마지막 서비스 weak None입니다. unsigned ID mock에 원본 string shutdown request를 보내 정상 handshake했다고 주장하지 않으며 실제 root shutdown의 kill/회수만 확인했습니다.
- [x] 정책 검사1의 의미 없는 문자열 부재 assertion을 실제 원본 상한 오류 문구 assertion으로 정정했습니다. assertion 변경에 직접 해당하는 이 검사만0.22초/suite0.00초 PASS입니다. 초기 검사의 stopped/missing/마스킹 성공과 동일 입력 통신 성공은 재사용하며 전체 suite/기존 remote LSP4건을 반복하지 않았습니다.
- [x] native lib/bin/tests strict `cargo clippy --lib --bin taide-native-app --tests ... -- -D warnings`: exit0·14.14초입니다. 기존 Wry dependency17 warnings와 authored strict를 구분합니다. authored4 exact edition2024 rustfmt·tracked diff check exit0, 신규 no-index check 빈 출력/exit1은 정상 신규 diff입니다.

Cargo 환경은 CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo, native manifest, --locked --offline --target-dir experiments/native-shell-spike/target입니다. 종료된 handles는3279/52065/30165이며 live Cargo handle은 없습니다. 실제 프로세스는 테스트가 생성한 합성 mock뿐입니다. 사용자 LSP/앱/파일/home/자격 증명/OS 설정/보호 bundle은 조작하지 않았습니다. root/Tauri/manifest/lock/MSRV/제품TS/TAIDE Git 불변입니다.

## 공식 근거와 미완료

[Rust weak ownership](https://doc.rust-lang.org/std/sync/struct.Arc.html#breaking-cycles-with-weak), [Tokio1.53.1 sleep](https://docs.rs/tokio/1.53.1/tokio/time/fn.sleep.html)를 확인했습니다. 실제 pinned root process/store/lifecycle와 원본 Tauri가 정책의 정본입니다. 새 package/버전/suppression은 없습니다.

- [ ] 실제30초 healthy reset·반복 crash3회·재생성 실패의 OS 실기는 전체 lifecycle/performance gate에서 확인합니다. 이번 slice는 새 policy를 만들지 않고 기존 root 정책 호출·실제 단일 재시작과 상한 상태를 직접 덮었습니다. 이 경계가 바뀌면 관련 검사만 추가합니다.
- [ ] 전체 production App/domain OS/Gist/assets·IDE/remote startup/Exit·Settings/HostBridge/AppFile 저장·native editor SessionClient 복구/화면 reinitialize를 연결합니다.
- [ ] 전체 N1~N8 0/8·keybinding RED·PTY remount 결정·view parity/cutover/Rust99%·IME/AX/perf/security/package/rollback은 남습니다. M8 완료 전 commit/push하지 않습니다.
