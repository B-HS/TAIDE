# PTY 종료 pause gate QA

## 대상 파일과 리포트

infra PauseGate/PtySession kill·Drop·child wait 배선과 Tauri 공개 spawn 문서를 확인합니다. child 없이 자기 할당 PTY·가짜 killer·채널/자기 thread만 사용했습니다.

## 상세 검증

- [x] `cargo test --offline -p taide-infra --lib pty::tests::명시적_kill --quiet`: 선행 RED는 kill 뒤 reader 대기가 해제되지 않아 exit 101이었습니다. 자기 thread를 Drop의 unpause 후 join한 뒤 실패했습니다. 수정 뒤 1건 통과(exit 0)했습니다.
- [x] `cargo test --offline -p taide-infra --lib pty::tests::종료_요청_뒤의_pause --quiet`: 종료 요청 뒤 재pause RED(exit 101), 수정 뒤 1건 통과(exit 0)했습니다.
- [x] `cargo test --offline -p taide-infra --lib pty::tests::kill_오류 --quiet`: 수정 뒤 1건 통과(exit 0), killer 오류 반환과 gate 해제를 함께 확인했습니다. RED 명령에 연결했던 이 검사는 앞 검사 실패로 실행되지 않았으며 RED 성공으로 집계하지 않습니다.
- [x] `cargo test --offline -p taide-infra --lib pty::tests -- --skip 명시적_kill --skip 종료_요청_뒤의_pause --skip kill_오류 --skip drop은_일시정지된 --skip drop은_셸_통합_임시_디렉터리도_정리한다`: 기존 batching/flush·builder와 새 child-wait source unit 15건 중 14건 통과·비활성 builder 1건 실패(exit 101)했습니다. 중복된 새 3건과 실제 child 검사 2건은 제외했습니다.
- [x] 원인은 pty/shell_integration 테스트의 동일 비시크릿 환경 플래그 변경 경합입니다. test-only 공유 mutex/RAII 복구를 구현한 뒤 `cargo test --offline -p taide-infra --lib 주입 --quiet` 5건과 `명시적_bash_override`/`명시적_zsh_override` 각 1건이 통과했습니다. 제품 환경 접근·검사 predicate는 유지합니다.
- [x] `cargo test --offline -p taide-infra --lib --quiet -- pty::tests::셸_통합이_비활성이면 pty::tests::zsh_주입 pty::tests::bash_주입 shell_integration::tests::이미_통합된`: 경합 당사자 4건이 기본 병렬 실행에서 통과(exit 0)했습니다. 앞선 단독 비활성 builder 실행은 진단용이고 중복 집계하지 않습니다. 당시 중간 상태의 unused guard 경고는 실제 consumer 적용 뒤 infra tests clippy로 확인했습니다.
- [x] `cargo test --offline -p taide --test taide_terminal_store_extraction 없는_세션_조회 --quiet`: 1건 통과(exit 0), 기존 공개 조회/오류 경로를 확인했습니다.
- [x] `cargo test --offline -p taide-infra --lib pty::tests::종료_전의_pause --quiet`: 기존 pause/unpause 상태 보존 1건 통과(exit 0)했습니다. 변경 후 서로 다른 성공 검사는 infra 24·Tauri 1로 25건이며 중복 실행은 합산하지 않습니다.

## 정적 검사

- [x] `cargo clippy --offline -p taide-infra -p taide-terminal -p taide --all-targets -- -D warnings`, `RUSTDOCFLAGS='-D warnings' cargo doc --offline -p taide-infra -p taide-terminal --no-deps --quiet`: exit 0입니다. 뒤에 변경한 test-only fixture와 추가 보존 검사 후 `cargo clippy --offline -p taide-infra --tests -- -D warnings`도 exit 0입니다. 제품 코드가 같은 성공은 재사용했습니다.
- [x] `cargo fmt --all -- --check`, `git diff --check`와 새 history/QA·갱신된 LSP wait QA 세 문서의 대상 Prettier write/check: exit 0입니다. 큰 PROCESS/architecture와 누적 이력의 무관한 재포맷은 하지 않았습니다.
- [x] dependency manifest·Cargo.lock·bindings/IPC manifest diff는 없습니다. 기존 타입 생성/IPC 성공 입력이 같아 재사용합니다.

## 미완료 gate와 생략 이유

- [ ] Unix 숫자 PID 권한의 wait/kill 직렬화는 가짜 killer로 먼저 재현합니다. 실제 회수된 PID나 사용자 프로세스에 시그널을 보내는 시험은 하지 않습니다.
- [ ] reader/flusher/wait/callback 완료와 SIGHUP 무시·pipe 보유 자손, spawn 작업 입장·제거 세션 소유/정상 root drain을 구현·검증합니다. gate의 깨우기를 OS Read 취소나 join으로 해석하지 않습니다.
- [ ] 기존 실제 child를 띄우는 infra Drop 2건과 Tauri PTY 출력/프로젝트 회수 검사는 이번 실행에서 제외했습니다. Unix clone_killer에 회수 뒤 숫자 PID 재신호 위험이 확인됐고 zsh fixture는 사용자 profile을 실행할 수 있으므로 해당 소유 gate를 해결하고 자기 전용 환경을 구성한 뒤 다시 실행합니다.
- [ ] 경합 실패 시 생성된 자기 fixture 임시 경로가 출력에 없어 이전 산출물의 정확한 경로는 확인되지 않았습니다. 임의 glob 삭제는 하지 않으며, 현재부터 반환된 자기 경로는 assert 전에 정리합니다. 재현 로그에 정확한 경로가 확보되면 해당 경로만 회수합니다.
- [ ] Windows·native ExitRequested/직접 Exit 및 실제 앱 종료/재시작은 별도 검증합니다. 현재 macOS 자기 gate/PTY 검사가 OS 전체 수명 종료의 근거는 아닙니다.
- [ ] M6 전수 body/port·나머지 자원과 M7/M8·Phase 0은 미완료입니다.
