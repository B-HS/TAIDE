# CI #88 프런트엔드 테스트 7건 실패

## 대상 파일

- `src/widgets/app-shell/app-shell.test.tsx`
- `src/widgets/app-sidebar/app-sidebar.test.tsx`, `src/widgets/window-chrome/status-bar-content.test.tsx`
- `src/widgets/auxiliary-window-shell/auxiliary-window-shell.test.tsx`
- `src/widgets/terminal-pane/terminal-session.test.tsx`

## 리포트

`main`의 CI #88(`8d5b83c`)은 Rust 작업이 성공하고 frontend에서 2,945건 통과·7건 실패했습니다. 전체 로그는 GitHub 실행 `36538448680`의 `logs_98940531656.zip`에서 확인했습니다. Bun 1.3.14의 기본 `bun test`는 테스트 파일 사이에서 `mock.module` 등록을 되돌리지 않으며, 실제 코드 결함과 달리 파일 간 테스트 fixture가 서로 덮인 것이 실패 원인이었습니다.

## 상세

- `app-shell.test.tsx`가 실제 `AppSidebar`·`StatusBarContent` export를 프로세스 전역 `mock.module`로 대체해 해당 컴포넌트를 직접 검사하는 다섯 테스트가 실패했습니다. 이 두 대체만 조립 테스트의 `renderShell` 내부 `spyOn(...).mockImplementation(...)`으로 옮겼습니다.
- 보조 창 SCM 테스트는 `git_status` 실패 문구로 패널 mount를 추정했지만 다른 테스트의 `git.ipc` 목이 상태 조회를 성공시켜 문구가 사라졌습니다. 해당 프로젝트의 Git status cache를 고정해 실제 Git 패널의 commit 입력을 확인하도록 변경했습니다.
- 터미널 재시작 테스트는 layout seed에 `gcTime: Infinity`만 주어 mount refetch가 다른 파일의 layout IPC 목을 읽고 cwd를 null로 바꿀 수 있었습니다. layout key에 `staleTime: Infinity`도 지정해 첫 spawn과 재시작이 같은 fixture를 사용하게 했습니다.
- 공식 Bun 문서의 파일별 `--isolate`를 CI와 같은 1.3.14로 한 번 실험했지만 기존 테스트 34건이 각 파일의 숨은 교차 의존성을 드러내 실패했습니다. 전체 테스트 설정 변경은 이번 7건 수정보다 범위가 크므로 적용하지 않았습니다.

## 검증과 남은 게이트

- Bun 1.3.14에서 대상 4파일 20/20 통과, 전체 비격리 실행 2,950 pass·2 fail입니다. 남은 두 실패는 로컬 sandbox의 loopback 서버 제한을 받는 `remote-reconnect.test.ts`이며 동일 두 테스트를 권한 허용 환경에서 2/2 통과시켰습니다.
- `bun run typecheck`, 변경 파일 Prettier 검사, `git diff --check`가 통과했습니다. CI #88은 과거 commit 결과이므로 이 수정의 원격 합격 근거가 아닙니다. 새 `main` CI 성공 전에는 릴리스 태그를 생성하지 않습니다.
