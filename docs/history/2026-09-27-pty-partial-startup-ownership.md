# PTY 부분 시작 소유권과 실제 회수

## 대상 파일

- `crates/taide-infra/src/pty.rs`: child 초기 Drop wait·생성 중인 자원 owner·reader/writer/thread factory·오류/패닉 fixture
- `docs/architecture.md`, `docs/PROCESS.md`와 관련 PTY QA

## 리포트

기존 spawn은 child 생성 뒤 reader/writer 취득 오류에 종료 요청만 보냈고 실제 child를 wait하지 않았습니다. reader/flusher/wait thread 생성은 `std::thread::spawn`을 사용해 OS 시작 오류에서 패닉했으며, 이미 시작한 worker와 생성된 integration 경로를 묶어 회수할 owner도 없었습니다. 자기 native child를 강하게 보유한 기록 wrapper에서 Drop의 실제 wait 누락을 RED(exit 101)로 확인했습니다. fixture observer는 같은 child handle로 회수하며 재사용될 수 있는 숫자 PID에 신호를 보내거나 사용자 프로세스를 사용하지 않았습니다.

## 상세

1. child wait owner의 초기 Drop은 기존 killer 권한으로 종료를 요청하고 같은 mutex에서 권한을 반납한 뒤 직접 소유한 child를 wait합니다. 정상 finish에서 이미 권한을 반납했으면 Drop은 다시 신호/회수하지 않습니다. 이 수정은 기존 Unix 회수 전 숫자 PID gate를 유지합니다. wait 오류를 성공한 회수로 보장하지 않습니다.
2. `PtySpawnOwner`는 child spawn 이전부터 master·생성 경로를 보유하고 생성 뒤 child slot·writer·pause/flush signal·모든 시작한 thread를 보유합니다. 실패/언와인드 시 gate를 열고 master/writer를 닫아 child/reader가 종료하도록 하며, 남은 child의 실제 wait와 모든 시작한 worker join 뒤에 owned 임시 경로를 정리합니다.
3. wait worker는 공유 slot에서 child를 단독으로 가져갑니다. waiter 생성 실패의 closure Drop만으로 child를 잃지 않으며 startup owner가 계속 보유합니다. 성공한 초기화에서만 세 worker handle·pause/killer·master/writer·integration 경로를 기존 PtySession/PtyCompletionHandle로 전달합니다. 정상 reader batching·stop/최종 flush·wait→pause stop→exit callback 순서는 유지합니다.
4. thread 시작은 `Builder::spawn`의 io Result를 사용합니다. 세 생성 지점의 OS 오류는 기존 Internal wire 오류로 반환하고 이미 시작한 worker를 Drop detach로 버리지 않습니다. 테스트는 resource/worker factory에 오류와 unwind를 주입하며 OS thread 제한을 변경하거나 실제 시스템을 고갈시키지 않습니다.
5. reader/writer·flusher/reader/wait worker의 오류 5경로와 unwind 5경로에서 자기 child의 실제 wait와 시작한 worker 종료를 확인했습니다. child가 생성되지 않은 spawn 오류도 자기 UUID 경로를 정리합니다. 별도 held output callback은 child wait 뒤에도 blocking spawn의 반환을 지연시키며 callback 해제 뒤 실제 join과 원래 오류 반환이 끝납니다.

## 공식 근거와 검증

[Rust Builder::spawn](https://doc.rust-lang.org/std/thread/struct.Builder.html#method.spawn)의 OS 생성 실패 Result 계약과 [catch_unwind](https://doc.rust-lang.org/std/panic/fn.catch_unwind.html)의 unwind/abort 구분을 확인했습니다. AssertUnwindSafe는 테스트에서만 단독 소비한 startup owner에 적용하며 unwind 뒤 그 owner를 재사용하지 않습니다. 설치된 portable-pty 0.9.0의 master reader/writer·직접 child/clone killer 및 Unix writer Drop도 대조했습니다. 새 의존성·unsafe·IPC 인수/반환 타입 변경은 없습니다.

실제 명령과 결과는 [PTY 부분 시작 QA](../quality-assurance/2026-09-27-pty-partial-startup-lifecycle.md)에 기록합니다. `/bin/sh -c`와 비어 있는 ENV/BASH_ENV, fixture가 직접 만든 UUID 경로·채널·child만 사용했습니다. 실제 앱·사용자 profile/프로세스·시크릿은 사용하지 않았습니다.

## 남은 경계

partial startup의 정상 OS cleanup과 synthetic 오류/언와인드는 확인했지만 OOM/abort panic, kill/wait의 실제 OS 오류 회복, SIGHUP 무시 child·그룹 이탈/pipe 보유 자손·non-yield callback/Read의 bounded 종료는 별도 gate입니다. 동기 cleanup은 호출한 blocking worker를 실제 완료까지 보유하며 강제 abort/기한을 보장하지 않습니다. 일반 root와 spawn lease는 이 worker를 기다립니다. 직접 native Exit·Windows/다른 Unix·native 실기 및 전체 M6/M7/M8·Phase 0, 승인된 push의 M6 전체 완료 조건은 유지합니다.
