# M8 실제 편집기 LSP 자동 복구

## 대상과 연결

대상은 `native/taide-native-app/src/lsp.rs`, `lsp-recovery.rs`, `tests/lsp-recovery.rs`입니다. 실제 NativeApplication이 사용하는 LspBridge·sync_document에서 native SessionClient의 watch 상태를 감독된 monitor에 연결했습니다. 기존 remote LSP adapter를 native editor 구현으로 이름만 바꾸지 않았습니다.

기존 taide-lsp의 process restart3회·500/1000/1500ms backoff·healthy30초 정책을 재사용합니다. degraded actor는 같은 SessionClient/coordinator의 restart를 통해 generation 증가·initialize·최신 mirror replay를 수행합니다. coordinator의 기존 request timeout15초·replay 완료 전 Running 금지·pending 거절·소유 process 회수는 유지합니다.

monitor는 AppServices를 weak 참조하고 mutation guard 안에서 shutdown·project 존재/root_missing·captured root·generation/phase를 다시 확인한 뒤 재시작합니다. Stopping/Stopped·watch 종료·registry 폐기 시 끝납니다. 동일 상태 갱신은 backoff 시각을 미루지 않고, healthy reset은 같은 generation의 Running이 유지될 때만 인정합니다. deadline이 이미 지난 경우 watch 갱신이 timer를 계속 굶기지 않도록 ready를 먼저 확인합니다.

registry의 notice/status 전달은 watch channel identity도 확인해 같은 key/generation을 재사용한 다른 actor의 늦은 전달을 거절합니다. 정상 initial generation의 기존 Synced/Formatted reply 순서는 유지하고 복구 상태만 별도 Synced로 전달합니다. 실제 NativeApplication은 아직 Synced 상태를 표시하지 않으므로 status UI 완성으로 세지 않습니다.

## 실패와 최소 검사

공통 명령 prefix는 `CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo cargo`이고 flags는 `--manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target`입니다.

- [x] 최초 `--test lsp-recovery`는 compile13.90초/suite5.01초 timeout(handle89799), HashMap 추론 E0282(handle59051), 수정 뒤 compile11.81초/suite5.10초 timeout(handle78490)이 있었습니다. 두 timeout은 `--native-document`가 진단을 발행하지 않는데 초기 진단을 기다린 fixture 오류이며 제품 recovery RED가 아닙니다. 관련 모드를 `--native-actions`로 바꾸고 대기 단계 이름을 추가했습니다. 제품 root/timeout 정책은 완화하지 않았습니다.
- [x] 올바른 fixture로 monitor attach 전 baseline을 분리해 실제 initial diagnostics 이후 child kill·crash recovery timeout을 확인했습니다. compile10.97초/suite5.38초·RED(handle39327). 이때 구현 중이던 미사용 monitor7 warnings는 실제 detach baseline의 경고이며 최종 strict 성공으로 계산하지 않습니다. attach는 다시 복구됐습니다.
- [x] 추가 편집 fixture에서 replace_selections의 필수 Option 인자 누락 E0061(handle71012)을 고쳤습니다. 실제 bridge 신규 검사 1 PASS(handle57524): compile8.38초/suite0.63초입니다. 실제 발견된 UUID 합성 executable·root/canonical 문서·native session을 사용해 child kill→Degraded→대기 중 live edit→같은 actor generation1/Running→최신 `formatted:live:` 문서·두 번째 kill 후 마지막 문서 close→registry만 tracked1→worker/process join·shutdown/task0·weak owner 해제를 확인했습니다. 사용자 앱이나 프로세스를 종료하지 않았습니다.
- [x] 별도 policy unit `--lib lsp::recovery::tests` 1 PASS(handle72995): compile10.52초/suite0.00초·filtered215입니다. limit3·중복 status의 불변 deadline/count·ready 경계·다른 generation의 healthy 거절·같은 generation reset·Stopping timer 폐기를 확인했습니다. 실제 bridge 성공 뒤 추가된 deadline-ready 우선 분기는 이 결정적 timer 검사와 정적 검사로 덮고 성공한 child 왕복을 반복하지 않았습니다.
- [x] native lib/bin/tests clippy `-- -D warnings` exit0·17.31초(handle5031 종료), authored3 exact rustfmt입니다. 기존 Wry dependency17 warnings는 authored 검사와 구분합니다. tracked/new Rust3개 whitespace 출력 없음이며 no-index exit1은 신규 diff입니다. 결과 문서의 Prettier/check는 아래 마무리 기록으로 확인합니다.

모든 Cargo는 직렬/locked/offline/기존 target입니다. 새 dependency/manifest/lock/root/Tauri/MSRV/제품TS/Git 변경은 없으며 보호 bundle·사용자 home·OS/IME/VoiceOver·키 파일에는 접근하지 않았습니다.

마무리: 대상5문서 Prettier 완료·tracked whitespace exit0, 신규 QA/bug no-index whitespace 출력 없음(exit1은 신규 diff)입니다. 모든 Cargo handle은 종료됐고 앞선 자산 준비 성공과 기존 remote adapter 성공을 재사용했습니다.

## 확인한 API

Tokio1.53.1 [watch Receiver](https://docs.rs/tokio/1.53.1/tokio/sync/watch/struct.Receiver.html)의 borrow_and_update·changed cancellation safety·same_channel을 확인했습니다. borrow는 await 전에 해제합니다. sleep_until 웹 조회 오류는 성공으로 기록하지 않으며 설치된 동일 버전 `src/time/sleep.rs`의 공식 API/cancellation 문서를 확인했습니다. SessionClient.restart·native coordinator replay·기존 process policy와 원본 TS lsp-session-registry의 재초기화 계약은 실제 파일로 대조했습니다.

## 미완료와 부채

- [ ] 원본 TS의 같은 generation handshake 최대3회/2초 대기와 실패 후 dispose/reacquire, native 수동 재시작·UI recovery 상태/진단 제거는 전체 client parity에서 남습니다. 현재는 actor가 복구 가능한 Degraded에서 process 재시작 정책만 연결했고 closed runner·spawn 오류·exhausted 정책은 종료합니다.
- [ ] 다른 actor의 늦은 notice/status를 실제 close→같은 key 재생성 경합으로 구동하는 검사는 미실행입니다. source identity 코드를 연결했지만 이번 unit을 실제 ABA 검증으로 주장하지 않습니다. registry 보존/재획득·다중 창/보안 gate에서 실행해야 합니다.
- [ ] 초기 coordinator와 실제 remote LspStore entry의 하나된 상태·multi-root/shares_sessions·discovery 비용·모든 provider·log/progress/diagnostics UI·실제 언어 서버/GUI/OS/장기30초 healthy 및 반복 crash stress·전체 queue/RSS/성능은 남습니다.
- [ ] actual App 필수 assets/effects/ports owner·Settings/AppFile/IDE 화면·Rust 원격 UI 생성·N1~N8 0/8·최종 TS 제거/배포는 미완료입니다. M8 전체 완료 전에 commit/push하지 않습니다.
