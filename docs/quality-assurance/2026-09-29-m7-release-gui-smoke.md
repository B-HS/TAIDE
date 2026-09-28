# M7 격리 release 앱 화면·파일 회귀 부분 실기

## 범위

전용 identifier `net.gumyo.taide.m6release.0ee5f27e64f94daa82e4789556697b3c`의 기존 release `.app`을 `ZDOTDIR`, `CLAUDE_CONFIG_DIR`, `TMPDIR`가 모두 `/private/tmp/taide-m6-isolated.byLYaMWs` 아래를 가리키게 실행했습니다. `TAIDE_PERF=1`을 지정했고 사용자 프로젝트·IDE lockfile 내용·시크릿은 사용하거나 읽지 않았습니다. 이 번들은 종료 단계 계측을 추가하기 전에 빌드된 산출물이므로 M6 직접 Exit 순서 검사에 쓰지 않습니다.

## 관찰

- 첫 창의 `Open Folder`가 나타났고 macOS 폴더 선택기로 임시 `project`를 열자 explorer에 네 파일과 Welcome/Terminal tab이 표시됐습니다. 다크 테마의 첫 화면과 프로젝트 화면을 실제 스크린샷으로 확인했습니다.
- 팔레트 `>performance` 검색에서 `App: Show Performance Snapshot` 명령이 노출됐습니다. 실행 후 일반 앱 화면·표준 출력에는 수치가 표시되지 않았고 당시 release 웹뷰의 검사기는 단축키로 열리지 않았습니다. 따라서 이 실행에서 `perf_snapshot.enabled` 응답과 프론트 표는 확인하지 않았습니다. 후속 전용 계측 앱의 수치는 [단일 세션 실측](2026-09-29-m7-one-session-perf-gui.md)에 분리했습니다.
- 임시 `fixture.ts`가 Monaco editor와 breadcrumb로 열렸고 접근성 값에 원문 `export const fixtureValue = 1;`가 보였습니다. 임시 편집 후 전체 선택·원문 복원·저장을 수행했고 디스크 내용이 원문임을 확인했습니다. 이 관찰은 복잡한 IME/undo·dirty dialog 회귀를 입증하지 않습니다.
- Search tab에서 `fixtureValue` 입력에 `1 results in 1 files`와 해당 줄이 표시됐습니다. Git tab에서는 비저장소 상태와 `Initialize Repository` 버튼이 나타났으며 실제 저장소를 만들지 않았습니다. Settings tab의 Appearance 목차와 `TAIDE Dark` 선택 상태를 접근성 트리·스크린샷으로 확인했고 설정값은 바꾸지 않았습니다.
- `⌘Q` 뒤 앱 프로세스가 exit 0으로 끝났고 IDE listener `57040`과 격리 `57040.lock`이 없어졌습니다. 실제 로그에서 임시 프로젝트 watcher 인덱싱 5엔트리와 `vtsls` 기동을 확인했습니다. 활성 자원 동시 drain, standalone 직접 `Exit`, 원격 WebSocket은 이 실행에서 시험하지 않았습니다.

## 남은 판정

- [ ] [단일 검증 결정](../acknowledge/2026-09-29-m7-one-pass-validation-scope.md)에 맞춰 동일 fixture의 지표별 유효 표본과 계측 출력 경로를 확보합니다. 과거 3회 반복 요구는 적용하지 않습니다.
- [ ] 모든 화면·상태·키보드·테마/로케일·다중 창·시각/접근성 기준선을 각 컴포넌트에 대조합니다.
- [ ] M6 직접 Exit와 활성 원격 서버/WebSocket의 종료 수명은 별도 격리 실행으로 판정합니다.

이 기록은 release GUI의 좁은 경로에 대한 실측이며 M7-C4b·M7-C4c 또는 Phase 0 전체의 통과 판정이 아닙니다.
