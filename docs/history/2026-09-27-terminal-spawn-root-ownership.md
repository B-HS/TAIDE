# 터미널 spawn과 정상 root 종료 소유권

## 대상 파일

- `crates/taide-terminal/src/store.rs`, `Cargo.toml`, `Cargo.lock`: 입장 lease·세션과 독립된 완료 목록·cleanup task·idle 대기
- `crates/taide-runtime/src/terminal_actions.rs`, `state.rs`, `exit_drain.rs`, `lib.rs`: 실제 spawn worker·owned mutation guard·정상 root drain
- `crates/taide-infra/src/pty.rs`: 완료 handle의 같은 worker identity 확인
- `src-tauri/src/domain/terminal/commands.rs`, `lib.rs`, `remote_gateway.rs` 및 관련 store/조립 integration 검사

## 리포트

기존 외부 waiter가 빌린 mutation guard를 보유하는 동안 비감독 spawn이 실행됐습니다. 요청 Drop은 이미 시작한 worker를 취소하지 않지만 guard를 먼저 해제했고, 삭제/교체한 세션의 완료 handle도 저장소에서 사라졌습니다. 정상 root drain에는 PTY 완료 대기가 없었습니다. 새 admission API 두 검사는 구현 전 E0599(exit 101)였으며, 이것을 실제 OS 회수 실패 재현으로 집계하지 않습니다.

## 상세

1. `begin_spawn`은 종료 뒤 신규 입장을 거절합니다. lease는 실제 blocking factory 완료 또는 미시작 queued work의 Drop까지 저장소를 보유합니다. factory는 저장소 mutex 밖에서 실행하며 shutdown과 경합해 생산한 세션도 완료 목록에 등록한 뒤 종료를 요청합니다.
2. 세션 맵과 별개인 완료 목록은 removed/replaced/unreturned worker를 보유합니다. 같은 완료 handle은 Arc identity로 중복 등록하지 않습니다. 제거한 PTY의 master/writer는 Drop하고, runtime이 있으면 소유한 cleanup task로 실제 join합니다. runtime이 없거나 종료 중이면 정상 root의 `wait_for_idle`이 기다립니다. 대기 future Drop은 저장소의 완료 handle·cleanup task를 버리지 않습니다.
3. `AppState`의 기존 mutation mutex를 Arc로 보관하고 같은 lock의 owned guard를 비동기로 획득합니다. 감독된 worker·미반환 결과 owner가 guard를 보유하며 성공한 전달은 등록/이벤트 발행까지 guard를 반환합니다. 시작한 blocking 작업을 abort로 완료했다고 주장하지 않습니다. queued shutdown과 factory panic도 lease/guard를 반납합니다.
4. Tauri spawn은 provider→owned guard→프로젝트 확인→감독 worker→등록→기존 이벤트 순서를 유지합니다. 종료 중 입장/등록 오류에는 Spawned를 발행하지 않습니다. 원격 adapter의 내부 State 인자도 같은 감독자를 주입하며 opts/onData·channel/replay·scan/counter/exit payload는 변경하지 않았습니다.
5. 정상 `ExitDrain`은 terminal admission을 닫고 감독 작업·설치 lease·일반 LSP·터미널 idle을 비동기로 기다립니다. 자기 child 종료 코드 7 뒤 callback을 붙잡아 root 종료 준비가 premature하게 올라가지 않는 것을 확인했습니다. callback panic의 join 실패에는 준비 플래그와 종료 callback을 실행하지 않습니다. native 이벤트 루프는 이 대기의 blocking join 장소가 아닙니다.

## 공식 근거와 검증

설치된 Tokio 1.53.1 `sync/mutex.rs`의 `lock_owned`/OwnedMutexGuard, `sync/notify.rs`의 notified 생성 이전/이후 notify_waiters 계약 및 [spawn_blocking 공식 문서](https://docs.rs/tokio/1.53.1/tokio/task/fn.spawn_blocking.html)의 started work 비취소 계약을 대조했습니다. 기존 프로젝트 Tokio를 terminal crate의 직접 의존으로 재사용한 이유는 native 루프를 막지 않는 알림·cleanup 핸들 대기입니다. lockfile에는 기존 tokio의 직접 의존 한 줄만 추가됐고 버전/패키지 추가는 없습니다.

실제 명령과 결과는 [터미널 spawn/root QA](../quality-assurance/2026-09-27-terminal-spawn-root-lifecycle.md)에 기록합니다. 사용자 profile을 비활성화한 자기 `/bin/sh`와 전용 blocking pool/채널만 사용했습니다. 실제 앱·사용자 프로세스·시크릿은 사용하지 않았습니다.

## 남은 경계

이 결과는 성공한 PTY 생산 결과와 정상 ExitRequested 경로의 소유권입니다. master reader/writer 취득 실패·thread 부분 시작/패닉 시 아직 생성 중인 child/worker, OS wait 오류·SIGHUP 무시/pipe 보유 자손·non-yield callback/Read의 bounded 종료·직접 native Exit·Windows/다른 Unix·native 실기 및 M6 전체/M7/M8/Phase 0은 미완료입니다. 마지막 store/완료 handle Drop을 실제 join 증거로 사용하지 않습니다. 일반 push는 승인된 M6 전체 완료 조건 이후입니다.
