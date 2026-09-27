# PTY 종료 pause gate 보완

## 대상 파일

- `crates/taide-infra/src/pty.rs`: PauseGate·명시 kill/Drop·child wait 종료 배선과 fixture
- `crates/taide-infra/src/shell_integration.rs`: 공유 환경 플래그 test fixture의 잠금·원상 복구
- `src-tauri/src/domain/terminal/commands.rs`: 공개 spawn 문서의 종료 요청/실제 완료 구분
- `docs/architecture.md`, `docs/PROCESS.md`, 연결된 PTY/LSP 종료 QA

## 리포트

TerminalStore.kill_all은 세션을 보유한 채 kill만 호출하지만 기존 kill은 pause를 해제하지 않았습니다. child 없이 직접 할당한 PTY·가짜 killer·자기 대기 thread의 fixture에서 명시 kill 뒤 thread가 깨지 않는 실패(exit 101)를 재현했습니다. 별도 fixture는 종료 요청 뒤 다시 pause할 수 있어 실패(exit 101)했습니다. 두 RED 모두 세션 Drop/자기 thread join 또는 자기 PTY Drop을 마친 뒤 실패합니다.

## 상세

1. PauseGate는 같은 mutex 안에 paused/stopped 상태를 두며 stop이 영구적으로 paused를 해제하고 condvar를 알립니다. 이후 set_paused는 상태를 다시 닫지 않습니다. 동기 종료 요청과 늦은 pause 명령을 같은 gate에서 직렬화하며 유휴 polling을 추가하지 않습니다.
2. 명시 kill은 killer를 호출하기 전에 gate를 닫습니다. killer 오류가 있어도 reader의 pause 대기는 해제되고 기존 오류를 반환합니다. Drop은 같은 kill 경로를 재사용하며 기존 자기 shell-integration 임시 디렉터리 정리를 유지합니다.
3. child wait thread도 wait 결과 뒤 exit callback 전에 같은 pause gate를 닫습니다. 이 연결은 source unit으로 확인했으며 실제 OS child/pipe와 callback 완료를 전부 회수했다고 표현하지 않습니다.
4. 기존 infra Drop과 Tauri spawn 문서의 무누수 보장을 제거했습니다. 시작한 blocking spawn은 외부 waiter Drop으로 취소되지 않으며 도달하지 않은 결과의 Drop은 종료 요청일 뿐 child wait·reader·flusher·callback join이 아닙니다. IPC 서명·출력 batching/scan/replay·killer의 SIGHUP 정책은 변경하지 않았습니다.
5. 기존 builder 회귀는 병렬 검사에서 14건 통과·비활성 주입 1건 실패(exit 101)했습니다. pty와 shell_integration의 테스트가 같은 비시크릿 플래그를 서로 덮어쓰는 원인을 확인하고 test-only 공유 mutex/RAII 복구를 적용했습니다. 제품 환경 접근은 변경하지 않고 검사 전체를 직렬화하거나 판정을 끄지 않습니다. 실패 fixture도 자신이 받은 UUID 임시 경로를 assert 전에 정리하도록 보완했습니다. 선행 실패의 실제 생성 경로는 출력에 없어 임의 glob 삭제는 하지 않았습니다.

## 공식 근거와 검증

[portable-pty 0.9.0 ChildKiller](https://docs.rs/portable-pty/0.9.0/portable_pty/trait.ChildKiller.html)와 설치된 공식 원천을 대조했습니다. parking_lot 0.12.5 Condvar 웹 조회는 접근 실패하여 설치된 공식 `src/condvar.rs`의 predicate mutex·wait/notify_all 계약을 확인했습니다. 새 의존성/unsafe는 없습니다.

명령·실제 결과와 제외한 검사는 [PTY pause QA](../quality-assurance/2026-09-27-pty-shutdown-pause-lifecycle.md)에 기록합니다. 실제 앱·child·사용자 PTY/프로세스·키링·시크릿은 사용하지 않습니다.

## 미완료 경계

이번 완료 범위는 pause gate의 해제와 재진입 거절입니다. Unix clone_killer의 숫자 PID와 child wait 사이 회수 gate, SIGHUP 무시 child, 세 thread의 실제 join, spawn worker/제거 세션 소유와 root drain은 별도 잔여 구현입니다. pause 수정은 OS Read를 중단하거나 pipe를 보유한 자손을 종료하지 않습니다. M6 전체 body·M7/M8·Phase 0 및 직접 native Exit 실기는 미완료이며 일반 push/UI 실행은 보류합니다.
