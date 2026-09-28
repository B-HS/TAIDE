# M6 watcher 이후 callback 종료 경계 대조

## 대상 파일

- `crates/taide-runtime/src/app_services.rs`, `crates/taide-runtime/src/exit_drain.rs`
- `src-tauri/src/lib.rs`, `src-tauri/src/domain/ide/server.rs`
- `crates/taide-ide/src/store.rs`

## 리포트

현재 AppServices는 AppState·TaskSupervisor·IDE/remote/agent/LSP/terminal 등 21개 상태·포트를 한 객체로 조립합니다. Tauri composition root는 여섯 callback 조립 함수와 setup의 감독 작업을 유지합니다. 정상·직접 ExitDrain은 감독 작업, 설치 lease, LSP, AI owner, PTY와 이번에 연결한 file/Git watcher callback/thread 완료를 기다립니다. 이는 등록된 자원의 정상 종료 조건이며 모든 OS callback이나 외부 자손의 완료 증거는 아닙니다.

## 새로 확인한 경계

IDE 서버의 `handle_connection`은 WebSocket 연결별 `JoinSet`에 RPC 작업을 두고 연결이 끝나면 하위 작업을 abort합니다. diff/save 도구는 store에 pending 응답을 등록한 뒤 await하며 취소 시 ID 제거 owner가 없습니다. 서버 전체 `stop_server`는 pending을 drain하지만 연결 단절만으로는 실행되지 않습니다. [버그 기록](../bug/2026-09-28-ide-connection-pending-owner.md)에 파일·심볼과 미검증 fixture를 구분했습니다.

설치기 JD~JG의 OS 오류/그룹 밖 자손, PTY JP/JQ의 SIGHUP 무시/OS·Windows 분기, 실제 native Exit·GUI callback은 여전히 별도 gate입니다. 설치된 `portable-pty 0.9.0`의 분리된 Unix killer가 SIGHUP만 보내는 사실은 [원천](https://docs.rs/portable-pty/0.9.0/src/portable_pty/lib.rs.html)에서 확인했고, 자체 PTY에 한정한 강제 종료 정책은 사용자 선택 대기 중입니다. 이번 문서는 읽기 전용 대조 결과이며 제품 실행·코드 변경·전체 M6 완료 판정은 하지 않습니다.
