# CI 프론트엔드 테스트의 성능 IPC 모듈 mock 누수

## 대상 파일

- `src/entities/app/perf.ipc.test.ts`
- `src/shared/testing/dom-preload.ts`

## 관찰과 원인

`main` CI #85의 frontend job에서 타입·lint·형식 검사는 통과했지만 단위 테스트는 2,741건 통과·171건 실패·미처리 오류 25건으로 끝났습니다. 처음에는 성능 IPC 테스트의 전역 `mock.module('@tauri-apps/api/core')`가 원인이라고 판단했습니다. [Bun 공식 문서](https://bun.sh/docs/test/mocks)상 `mock.restore()`는 `mock.module()`의 override를 복원하지 않으므로 이 mock 자체는 제거해야 했지만, CI #86에서도 동일한 171건이 실패해 주원인이라는 판단은 기각됐습니다. 로컬 Bun 1.4.2의 대상 2파일 10건은 수정 전에도 통과했으며, CI는 Bun 1.3.14와 Ubuntu 환경입니다.

## 수정과 검증

모듈 전체 대체를 제거하고 생성 `commands.perfSnapshot` 메서드만 각 테스트에서 `spyOn`으로 교체했습니다. 공통 preload의 `beforeEach(mock.restore)`가 이 spy를 다음 테스트 전에 복원합니다. 원격 미러의 무호출, 네이티브 gate on/off, IPC 오류 수용을 유지했습니다. CI #86 로그 아카이브의 최초 실패는 별도로 `agent.ipc.ts`의 `releaseWaitMarker` export 누락이었으며, 후속 수정은 `docs/bug/2026-09-29-agent-command-test-module-mock-leak.md`에 기록합니다.

- `bun test src/entities/app/perf.ipc.test.ts src/entities/file/file.query.test.ts`: 10건 통과.
- `bun run typecheck`: exit 0.
- 대상 Prettier·`git diff --check`: exit 0.
- CI #86 frontend job: 동일한 171건 실패. 이 변경만으로는 전체 CI를 복구하지 못했습니다.
