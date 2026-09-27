# PTY 부분 시작 QA

## 대상 파일과 리포트

infra PtyChildWaitOwner/PtySpawnOwner의 partial reader/writer/thread 실패와 actual wait/join을 확인합니다. 성공한 세션/정상 root 종료 성공과 실제 OS 오류·native 실기를 구분합니다.

## 실제 결과

- [x] `cargo test --offline -p taide-infra --lib pty::tests::부분_시작_child_owner_drop --quiet`: 자기 native child wait 기록 wrapper에서 실제 wait 누락 RED(exit 101), 수정 뒤 1건 GREEN(exit 0)입니다. 같은 child handle observer가 fixture를 회수하며 숫자 PID를 재신호하지 않습니다.
- [x] `cargo test --offline -p taide-infra --lib pty::tests -- --skip drop은_일시정지된 --skip drop은_셸_통합`: 부분 시작 신규 5건 포함 31건 통과·기존 source predicate 1건 실패(exit 101)였습니다. 옛 `pause.clone` 표기를 실제 `owner.pause.clone`으로 대조하고 reader/child가 같은 gate를 복제하는 두 predicate와 wait→stop→callback 순서를 유지했습니다. 해당 실패 검사만 단독 재실행해 1건 통과(exit 0)했으며 현재 서로 다른 infra 성공은 32건입니다. 앞 GREEN은 중복 합산하지 않습니다.
- [x] 같은 32건의 오류/unwind 두 검사에서 reader/writer·세 worker factory 각각 5경로, actual child wait·시작한 worker 종료·자기 임시 경로 정리를 확인했습니다. 새로운 factory의 합성 오류를 실제 OS 고갈 재현으로 집계하지 않습니다.
- [x] held output callback 검사는 child 회수 뒤에도 worker 반환이 pending이고 callback 해제 뒤 join/오류 반환이 완료되는 것을 확인했습니다. 기존 성공한 spawn의 callback 지연·worker panic·PID/pause·배치/빌더도 유지합니다.

## 연결·정적 검사

- [x] `cargo test --offline -p taide-terminal --lib store::tests --quiet`: 4건, `cargo test --offline -p taide-runtime --lib terminal_actions::tests --quiet`: 6건, `cargo test --offline -p taide-runtime --lib exit_drain::tests --quiet`: 5건, `cargo test --offline -p taide --test taide_terminal_runtime_extraction --quiet`: 1건 통과(exit 0)입니다. 변경한 실제 producer에 연결된 admission/제거/교체/idle·실제 spawn/guard·정상 root/실패 ready·기존 출력/스캔/replay를 확인했습니다. infra 32·연결 16으로 서로 다른 48건입니다.
- [x] `cargo clippy --offline -p taide-infra -p taide-terminal -p taide-runtime -p taide --all-targets -- -D warnings`와 변경한 source predicate/공개 설명 뒤 `cargo clippy --offline -p taide-infra --tests -- -D warnings`: exit 0입니다.
- [x] 공개 spawn의 동기 cleanup 설명 뒤 `RUSTDOCFLAGS='-D warnings' cargo doc --offline --no-deps -p taide-infra --quiet`, `cargo fmt --all -- --check`, `git diff --check`: exit 0입니다. 공개 IPC 인수/반환 타입·bindings 생성 입력·dependency가 같아 앞선 생성/IPC·AppServices 성공은 재사용합니다.
- [x] 새 history/QA와 연결된 PTY QA 세 문서, 총 5개 MD를 `bun node_modules/prettier/bin/prettier.cjs --ignore-path /dev/null --write` 및 `--check`로 대상별 확인했습니다(exit 0). 큰 PROCESS/architecture는 무관하게 재포맷하지 않았습니다.

## 생략과 필수 잔여 gate

- [ ] 실제 kill/wait 오류·SIGHUP 무시 child·pipe 보유/그룹 이탈 자손·non-yield callback/Read의 bounded 종료는 미검증입니다. 종료 요청이나 cleanup 오류 무시를 성공 회수로 처리하지 않으며 자기 fixture/OS 정책이 준비된 뒤 수행합니다.
- [ ] OOM/abort panic은 catch_unwind로 회복하지 않습니다. OS 한도 변경/실제 thread 고갈은 사용자 시스템에 영향이 있어 실행하지 않았습니다. factory 오류 주입이 해당 Result 경로의 소유/정리만 검사한다는 범위를 유지합니다.
- [ ] 위험한 기존 profile Drop fixture 두 건, Windows/다른 Unix·직접 native Exit·실제 앱 시작/종료와 전체 M6/M7/M8·Phase 0은 미실행/미완료입니다. 현재 성공으로 실기 결과를 대체하지 않습니다.

fixture가 직접 생성한 UUID 경로만 startup owner가 제거했으며 사용자 파일을 삭제하지 않았습니다. 선행 owner RED도 observer가 같은 실제 child handle을 회수해 남긴 프로세스는 없습니다.
