# CLI 명령 테스트의 agent IPC 모듈 mock 누수

## 대상 파일

- `src/entities/agent/agent.commands.test.ts`
- `src/entities/agent/agent.ipc.ts`
- `src/entities/layout/tab-path-change.test.ts`

## 관찰과 원인

`main` CI #86 frontend job은 2,741건 통과·171건 실패·미처리 오류 25건이었습니다. 전체 로그 아카이브에서 첫 연쇄 실패는 `agent.commands.test.ts` 다음의 `file.query.test.ts`였고, `SyntaxError: Export named 'releaseWaitMarker' not found in module .../agent.ipc.ts`가 반복됐습니다. 앞선 테스트가 `agent.ipc`를 일부 export만 제공하는 `mock.module`로 프로세스 전체에 대체했기 때문입니다. 같은 로그에 `installAgentHooks` export 누락도 2건 있었으며, 별도의 `tab-path-change.test.ts`도 같은 모듈을 부분 대체하고 있었습니다.

## 수정과 검증

`agent.commands.test.ts`의 `agent.ipc`와 `sonner` 전역 모듈 대체를 제거했습니다. 생성 `commands.agentCliStatus/Install/Uninstall` 및 `toast.success/error`를 테스트별 `spyOn`으로 교체해 공통 preload의 `beforeEach(mock.restore)`가 다음 테스트 전에 복원하게 했습니다. `tab-path-change.test.ts`의 사용하지 않는 `agent.ipc` 모듈 대체도 제거했습니다. CLI 상태 조회 실패·설치 성공·실패·dangling 심링크 계약은 유지했습니다.

- 로컬 Bun 1.4.2: 관련 3파일 27건과 모듈 대체 제거 후 관련 2파일 31건 통과, `bun run typecheck`·대상 Prettier·`git diff --check` exit 0.
- CI와 로컬 Bun 버전이 다르므로 최종 판정은 수정 커밋의 `main` CI frontend job으로 합니다.
