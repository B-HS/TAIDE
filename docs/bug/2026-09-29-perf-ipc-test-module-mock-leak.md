# CI 프론트엔드 테스트의 성능 IPC 모듈 mock 누수

## 대상 파일

- `src/entities/app/perf.ipc.test.ts`
- `src/shared/testing/dom-preload.ts`

## 관찰과 원인

`main` CI #85의 frontend job에서 타입·lint·형식 검사는 통과했지만 단위 테스트는 2,741건 통과·171건 실패·미처리 오류 25건으로 끝났습니다. 로그의 미처리 오류는 `Forbidden`·`denied`였고, 성능 IPC 테스트가 `@tauri-apps/api/core`의 `invoke`를 `mock.module`로 프로세스 전체에 바꾼 뒤 실패 응답을 남기는 코드와 일치했습니다. [Bun 공식 문서](https://bun.sh/docs/test/mocks)상 `mock.restore()`는 `mock.module()`의 override를 복원하지 않으므로, 다른 파일의 테스트까지 오염될 수 있습니다. 로컬 Bun 1.4.2의 대상 2파일 10건은 수정 전에도 통과했으며, CI는 Bun 1.3.14와 Ubuntu 환경입니다. 따라서 대상 2파일의 로컬 성공을 CI 전체 성공으로 해석하지 않습니다.

## 수정과 검증

모듈 전체 대체를 제거하고 생성 `commands.perfSnapshot` 메서드만 각 테스트에서 `spyOn`으로 교체했습니다. 공통 preload의 `beforeEach(mock.restore)`가 이 spy를 다음 테스트 전에 복원합니다. 원격 미러의 무호출, 네이티브 gate on/off, IPC 오류 수용을 유지했습니다.

- `bun test src/entities/app/perf.ipc.test.ts src/entities/file/file.query.test.ts`: 10건 통과.
- `bun run typecheck`: exit 0.
- 대상 Prettier·`git diff --check`: exit 0.
- 최종 판정은 수정 커밋의 `main` CI frontend job 결과로 합니다.
