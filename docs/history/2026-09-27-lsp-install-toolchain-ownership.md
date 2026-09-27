# LSP toolchain child·reader 소유권 보완

## 대상 파일

- `crates/taide-lsp/src/install.rs`
- `crates/taide-runtime/src/lsp_install_toolchain.rs`, `lsp_install_actions.rs`, `lib.rs`, `Cargo.toml`
- `src-tauri/src/domain/lsp/commands.rs`, 설치 store extraction·platform event sink 검사
- `docs/architecture.md`, `docs/PROCESS.md`, 연결된 toolchain lifecycle QA

## 리포트

Tauri의 기존 toolchain 설치는 요청 future가 child를 직접 보유하고 reader의 JoinHandle을 버렸습니다. 요청 Drop은 child 종료를 보장하지 않았고 성공 반환은 reader EOF를 기다리지 않았으며 실패 반환은 EOF가 오지 않으면 계속 기다릴 수 있었습니다. 설치를 runtime의 감독된 worker로 이전해 요청 수명과 실제 child/pipe 종료 수명을 구분했습니다. 전체 앱 종료 완료를 증명한 것은 아닙니다.

## 상세

1. InstallGate가 최종 commit·취소·자원 생성/등록을 직렬화합니다. resource는 weak 등록하므로 resource→lease→control 순환 소유가 생기지 않습니다. 취소는 gate 밖에서 등록된 자원에 동기 전달하며 이미 commit한 설치는 늦은 취소로 뒤집지 않습니다.
2. std child는 감독된 blocking worker의 admission 안에서만 spawn합니다. InstallChild는 lease를 보유하고 취소 시 kill, worker에서 실제 try_wait/reap, 예외 Drop에서도 kill/wait를 수행한 뒤 lease를 해제합니다. 시작한 blocking 작업을 abort했다고 주장하지 않습니다.
3. Unix group 취소는 기존 TERM에서 KILL로 강화했습니다. 직접 만든 process_group(0)의 아직 살아 있는 parent를 mutex 안의 try_wait로 확인한 경우에만 음수 pid를 쓰며 0/1은 거부합니다. 이미 종료/reap된 parent의 그룹은 PID 재사용 위험 때문에 다시 signal하지 않습니다. Windows에서는 직접 child만 종료하며 전체 자손 종료는 미검증입니다.
4. stdout/stderr는 등록된 async reader task가 pipe·tail·lease를 소유합니다. owner Drop은 abort 요청일 뿐이며 실제 task Drop 전에는 설치 슬롯을 재사용하지 않습니다. 정상/실패 child 종료 뒤 두 reader를 함께 기다리며 EOF가 500ms 지연되면 abort 후 실제 join합니다. timeout은 reader 대기에만 적용되며 child kill/reap 전체에 대한 제한 시간 보장이 아닙니다.
5. 마지막 20줄·stderr→stdout 실패 출력 순서·자격증명 마스킹·byte 변환·기존 localized 오류와 spawn 실패의 진행 이벤트 정책을 유지합니다. Done은 실제 process/reader 결과와 취소 gate 뒤에만 발행합니다. 실제 AppHandle 전송은 TauriEventSink adapter에 남습니다. 기존 Tokio 의존성의 process/io-util feature만 명시하며 새 패키지·버전 변경은 없습니다.

## 검증 근거와 한계

새 resource/helper API가 없는 상태의 compile RED(exit 101) 뒤 store·runtime 검사들이 통과했습니다. 추가 fixture 단언의 Debug 요구(E0277)는 생산 타입에 불필요한 derive를 넣지 않고 matches 단언으로 수정했습니다. 실제 legacy child orphan을 실패 테스트로 실행한 것은 아니며 legacy 위험은 본문과 공식 Drop 계약을 대조한 결과입니다. 실제 실행 명령·건수는 [QA](../quality-assurance/2026-09-27-lsp-install-toolchain-lifecycle.md)에 기록합니다.

[Rust Child 공식 문서](https://doc.rust-lang.org/std/process/struct.Child.html)는 Child Drop이 child 종료/wait를 하지 않으며 wait/try_wait가 실제 종료 상태를 회수한다고 명시합니다. Tokio 1.53.1 설치 원천의 process ChildStdout/ChildStderr::from_std와 JoinHandle 문서에서 pipe 변환·Drop detach·abort 후 join·시작한 blocking 작업의 abort 불가 계약을 확인했습니다. struct 필드 Drop 순서는 [Rust Reference](https://doc.rust-lang.org/reference/destructors.html)를 따릅니다.

root ExitRequested/Exit의 admission 취소와 TaskSupervisor stop_all은 전체 설치 worker join을 뜻하지 않습니다. 부모가 먼저 종료한 뒤 계속 실행하는 자손·TERM 무시 자손·Windows process tree·OS 강제 종료·실제 앱/설치기는 아직 검증하지 않았습니다. LSP wait worker 자체·PTY·M6 command body 전수 판정 및 M7/M8도 미완료입니다. 이번 slice만으로 JD~JG나 전체 목표를 완료 처리하지 않습니다.
