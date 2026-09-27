# PTY worker 완료 handle 보존

## 대상 파일

- `crates/taide-infra/src/pty.rs`: 성공한 spawn의 세 worker handle·별도 완료 handle·reader unwind 정리·회귀 검사
- `docs/architecture.md`, `docs/PROCESS.md`, PTY child/worker 완료 QA

## 리포트

기존 spawn은 reader/flusher/wait의 `std::thread::JoinHandle`을 즉시 버렸습니다. source 검사로 이 경계를 RED(exit 101) 확인했으며, 이것을 실제 OS 회수 실패 재현으로 표현하지 않습니다. 앞선 PID 권한 보완은 `56ab97f`로 로컬 commit했습니다.

## 상세

1. 성공한 spawn은 세 handle을 `PtyCompletionHandle`에 보존합니다. 세션에서 완료 handle만 복제하면 PTY master/writer를 보유하지 않은 채 세션 Drop 이후에도 join할 수 있습니다. 종료 요청은 기존 permanent pause gate·동일 killer 권한을 사용합니다.
2. 실제 join은 대기 시점에 Tokio blocking pool에서 수행합니다. async/native 이벤트 루프에서 blocking join하지 않습니다. async mutex 안의 `&mut JoinHandle` 대기는 외부 future를 Drop해도 join task를 보존하므로 재대기할 수 있습니다. 동시 대기는 같은 결과를 공유하며 결과를 cache합니다.
3. 한 worker의 panic/오류에도 다른 worker를 모두 join한 다음 실패를 반환합니다. child wait 오류를 callback의 기존 `None` 코드와 별개로 완료 오류에 보존합니다. `is_finished`는 성공한 실제 join만 참이며 panic·runtime join 실패를 성공한 child 회수로 해석하지 않습니다.
4. reader의 unwind guard는 callback/reader panic에도 flusher의 stop condvar를 알립니다. 기존 normal reader의 stop→최종 flush 순서와 batching·scan/replay·exit callback 전달 순서는 유지합니다. OS read와 non-yield callback을 강제 중단하거나 bounded 종료한다고 주장하지 않습니다.
5. 자기 `/bin/sh` 종료 코드 7 뒤 exit callback을 채널로 붙잡았습니다. 첫 대기 취소와 세션 Drop 뒤에도 재대기가 pending이며 callback 해제 뒤 세 worker의 실제 join과 반복 대기가 성공했습니다. 별도 synthetic worker panic/오류와 reader panic에서 실제 다른 worker 종료를 확인했습니다. 실제 앱·사용자 profile/프로세스·시크릿은 사용하지 않았습니다.

## 공식 근거와 검증

[Rust thread JoinHandle](https://doc.rust-lang.org/std/thread/struct.JoinHandle.html)의 Drop detach/실제 join 계약과 설치된 Tokio 1.53.1 `runtime/task/join.rs`의 mutable await cancel-safety, `task/blocking.rs`의 started blocking 작업 비취소 계약을 대조했습니다. docs.rs의 잘못된 버전 경로 조회 실패를 성공으로 집계하지 않으며 lockfile의 실제 버전 원천을 사용했습니다. 새 의존성·unsafe·IPC 변경은 없습니다.

명령과 실제 결과는 [PTY worker 완료 QA](../quality-assurance/2026-09-27-pty-worker-completion-lifecycle.md)에 기록합니다.

## 미완료 경계

이 단위는 성공한 spawn 결과의 완료 handle입니다. TerminalStore의 시작/등록 admission, 실제 blocking spawn worker, 제거·교체 세션의 root 소유 목록과 정상 ExitDrain 배선은 아직 구현하지 않았습니다. 마지막 완료 handle까지 Drop하면 남은 join handle을 잃을 수 있으므로 현재 제품 전체 종료를 완료 처리하지 않습니다. partial thread spawn/pipe 실패·OS wait 오류·SIGHUP 무시/pipe 보유 자손·직접 native Exit·Windows 실기 및 전체 M6/M7/M8은 별도 gate입니다. 일반 push는 M6 전체 완료 후입니다.
