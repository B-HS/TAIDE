# LSP reader 수명 검증 경계

## 대상 파일과 리포트

`crates/taide-infra/src/lsp_proc.rs`의 stdout/stderr task는 wait worker가 ReaderTask로 소유합니다. 정상 종료는 두 reader를 함께 드레인하고 500ms 대기 초과는 abort 후 join합니다. owner Drop에서는 abort를 요청합니다.

- [x] 정상 완료·EOF 지연·poll 전 owner Drop·드레인 중 부모 취소·실제 배선 순서를 검사했습니다. 새 5건과 기존 관련 18건이 통과했습니다.
- [x] 추가 synthetic sh 자식의 stdout 프레임과 stderr tail이 exit callback 전에 전달되는 1건이 통과했습니다. 변경 후 서로 다른 검사 총 24건입니다.
- [ ] 실제 LSP 서버·grandchild가 OS stdout/stderr pipe를 계속 보유하는 상태와 앱 종료/재시작 실기를 M7에서 확인합니다.
- [x] 후속 [일반 LSP wait QA](2026-09-27-lsp-process-wait-lifecycle.md)에서 wait worker 소유·Drop 종료 요청·kill/reap 직렬화와 store/정상 coordinator의 실제 완료 대기를 확인했습니다. 대기 future Drop 뒤 재대기할 수 있습니다.
- [ ] LSP 설치/직접 native Exit 전체 gate·PTY thread 종료와 runtime 오류 경로는 M6 잔여 항목입니다. 정상 wait 완료와 best-effort Drop을 구분합니다.

## 상세

Tokio JoinHandle을 값으로 timeout에 넘겼던 stderr reader는 timeout 이후 Drop으로 detach될 수 있었습니다. stdout은 handle을 저장하지 않았습니다. 두 reader 모두 owner Drop에서 abort를 요청하도록 바꾸고, 정상 부모 경로에서는 mutable handle로 timeout을 기다린 뒤 필요하면 abort·await해 캡처/pipe 회수가 끝난 것을 확인합니다. child exit flag는 기존 위치에서 먼저 세우고 exit callback은 두 reader 회수 뒤 전달합니다.

Drop 자체는 await할 수 없으므로 이 경로를 동기 join으로 표현하지 않습니다. 부모 취소 검사에서는 runtime이 계속 실행되는 조건에서 캡처 해제 handshake까지 기다렸습니다. 실제 앱 종료에서 같은 OS 결과가 확인됐다는 근거는 아닙니다.

500ms는 EOF 드레인 대기 한도이지 전체 완료의 절대 상한이 아닙니다. `on_message`가 synchronous non-yield 작업에 갇히면 Tokio abort가 실행 중 poll을 강제 중단할 수 없고 join이 더 기다릴 수 있습니다. 현재 직접 변경한 부분은 async reader/EOF 회수이며 실제 callback 정책을 바꾸지 않았으므로, blocking/non-yield callback을 도입하거나 종료 지연을 관찰할 때 별도 fixture와 명시적인 실행 격리/종료 계약을 먼저 검증합니다.

synthetic pending task로 EOF 지연을 검사했고 실제 짧은 sh 자식은 고정 프레임·tail을 쓰고 스스로 종료했습니다. 사용자 프로세스·외부 설치기·실제 LSP 서버·앱·시크릿/키링에는 접근하지 않았습니다. M6 전체·M7·M8 gate는 미완료로 유지합니다.
