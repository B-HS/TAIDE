# 터미널 IPC 테스트의 전역 invoke mock 누수

## 대상 파일

- `src/entities/terminal/terminal.ipc.test.ts`
- `src/app/providers/ipc-settings-sync.test.tsx`
- `src/widgets/terminal-pane/terminal-session.test.tsx`

## 관찰과 원인

`main` CI #87의 frontend job은 2,934건 통과·18건 실패·미처리 오류 27건이었습니다. 전체 로그에서 `terminal.ipc.test.ts`의 전역 `mock.module('@tauri-apps/api/core')`가 설치된 뒤, 다른 테스트의 생성 IPC 명령까지 `project not open: prj-1`로 거절됐습니다. 이는 해당 테스트가 raw `invoke` 거절 경로를 검증하려고 만든 응답입니다. 같은 값이 설정 이벤트·IDE 동기화·터미널 세션 테스트에서 반복됐고, 후속 렌더링 검사까지 오염시켰습니다.

## 수정과 검증

전역 모듈 대체를 제거했습니다. 테스트마다 실제 Tauri `Channel`과 `invoke`가 사용하는 `window.__TAURI_INTERNALS__` 경계만 실패 응답으로 설정하고 종료 후 원래 값으로 복원합니다. 이로써 테스트 대상인 `spawnPty`의 raw 오류 정규화는 유지하면서 다른 파일의 모듈 구현은 바꾸지 않습니다.

- CI와 동일한 Bun 1.3.14로 전체 `bun run test` exit 0. 해당 과정에서 Happy DOM의 비치명적 CSS 로딩 경고 2건은 남았습니다.
- 마지막 fixture 정리 후 Bun 1.3.14의 대상 2건 통과, `bun run typecheck`·대상 Prettier·`git diff --check` exit 0.
- Linux CI 전체 판정은 수정 커밋의 `main` CI 결과로 합니다.
