# 아키텍처 감사 T1 정비 — 1차 배치(T1-E·T1-J·T1-B) 수정 계약 (2026-08-18)

> 감사 보고서 정본: `docs/quality-assurance/2026-08-18-architecture-audit.md`(§6.2 T1 묶음).
> 사용자 결정(2026-08-18): T1 정비 착수 — **저위험 묶음부터**(T1-E·T1-J·T1-B). 대형 3건
> (T1-H 락 IO·T1-I/C13 도메인 경계·T2-E AppError)은 후반. T0 완료(main=eecb493) 위에 얹는다.
> 감사 하중 주장은 2단 종합에서 재검증 완료(보고서 §2). 이 배치 착수 전 핵심만 재확인.

## 1. 1차 배치 범위 (저위험 3묶음)

### T1-E 계약 검증 테스트 (강제 장치 신설 — 코드 무변경 위주, 10건)
문서·정의에만 있고 기계가 강제하지 않는 파리티를 테스트로 고정. **X-B 테스트 신설 3건 포함**.
- **X1#8**: 이벤트 4목록 파리티 테스트(events.rs 25 / collect_events! 25 / fanout_remote_events! 24 /
  bindings.ts 25 집합 비교). **fanout 24 는 HotExitFlushRequested·AgentExternalOpen(T0 #14) 의도적
  제외 — 예외 목록을 테스트에 명시**(현재 주석뿐).
- **X1#9**: 커맨드 파리티 기준선을 생성물 bindings.ts → `collect_commands!` 로 이동(현재 셋 다 179
  일치하나 강제 없음. "낡은 bindings=낡은 dispatch" 통과 차단).
- **R5#3·R5#9**: Settings 필드 집합·테마 토큰 200개 비교 테스트(Rust↔TS 미러 드리프트 감지).
- **X1#6**: sync `schemaVersion` 게이트 실효화 — 상수 자기비교(무력)를 필드 화이트리스트 기반 미지
  필드 감지로, 또는 게이트 무력 사실을 doc 과 정합(정책 유지 시).
- 원격 거부 라우팅 테스트: deny 헬퍼 직접 호출이 아니라 dispatch() 경로로 검증(현재 동어반복).

### T1-J 프로젝트 종료 자원 회수 (C14, 9건)
`project_close`(project/commands.rs)가 현재 layouts·watchers·git_watchers 3개만 회수. T0 가 이미
#2(dirty_layouts)·#21(pty kill_project)·#15(asset scope — **롤백, T1 이월**) 를 부분 처리. 나머지 통합.
- **R4#7**: `GitStore`(projectId→repo_root)·`TreeStore`(전체 트리 캐시) 회수(같은 폴더 재오픈 시 옛
  캐시 부활 + 앱 수명 메모리 잔존).
- **X1#7 재도전(asset scope — T0 #15 롤백분)**: Tauri asset scope 가 add-only 라 forbid 로는 불가.
  **근본안: 앱 레벨 `register_uri_scheme_protocol` 로 asset 읽기를 자체 프로토콜로 재등록**하고 열린
  프로젝트 root 집합을 상태로 검사(닫으면 집합에서 제거 → 즉시 반영). 위험도 중(프로토콜 재구현) —
  구현 난이도 높으면 이 항목만 T1 2차로 분리하고 보고.
- **R7#9**: LSP kill 동기화(`LspProcHandle::kill` 이 AtomicBool 만 세우고 50ms 폴링 → 앱 종료 시
  고아 언어 서버. 터미널은 동기 kill — 비대칭 해소).
- **R7#14**: `restart_count` 정상 가동 후 미리셋 → 누적 3회로 자동 재시작 영구 중단 수정.
- **R7#17**: `LspInstallStore` 해제 보장. **R8#16**: shell_integration 임시 디렉터리 정리.
- 소유권 계약 명문화: `architecture.md §6.3`(Drop 이중화)에 project 종료 회수 목록을 정본화(T1-I 의
  `ProjectClosed` 훅 기반 자동 attach/detach 는 C13 묶음이라 후반 — 여기선 명령형 회수 완성).

### T1-B Settings 타입 좁히기 (C6, 10건)
Rust `Settings` 의 `Option<String>` 열거형 필드가 프론트 `as` 9지점·기본값 이중정의·필드 6중 미러의
진원지. **specta union 노출로 근본 해소**.
- **핵심**: editorRenderWhitespace·editorCursorStyle·editorCursorBlinking·terminalCursorStyle·aiProvider
  를 Rust 에서 `as const` union(백엔드 관행 enum) 으로 좁혀 specta 가 `"a" | "b"` union 을 생성 →
  프론트 `as` 9지점(editor-pane·untitled-pane·app-file-pane·terminal-session·settings-view·
  inline-completion) 제거.
- **R5#5**: 기본값 이중정의 제거(Rust `default_true()` vs settings-view `?? fallback` 드리프트 —
  Rust 기본값을 단일 출처로, 프론트는 bindings 값 소비). **R5#4**: 폰트 범위 3벌(Rust 미보정 / 8-32 /
  6-48) 통일 + sanitize 클램프.
- **X1#18**: specta `_Serialize`/`_Deserialize` 분할 7타입의 필수성 차이(ResolvedTheme 5필드) 문서 정합.
- 비전수 배열 2건(R5#11)·SearchMatchRowData(F1#8) 유도 전환.

## 2. 실행 구조

- **Phase R1(Rust 계약 테스트 단독, sonnet+xhigh)**: T1-E 전량(파리티 테스트 신설·기준선 이동·게이트
  실효화). 코드 변경 최소, 테스트 위주. 신설 테스트가 현재 상태에서 통과하는지(=이미 정합) 확인하고,
  불일치 발견 시 보고(문서만 있고 코드가 어긋난 것은 별도).
- **Phase R2(Rust 자원 회수 단독, R1 순차)**: T1-J — GitStore/TreeStore/LSP kill/restart_count/
  InstallStore/shell temp 회수 + project_close 통합. asset scope 재등록은 난이도 평가 후 진행/분리 보고.
- **Phase R3(Rust Settings 타입 단독, R2 순차)**: T1-B Rust — 열거형 union 좁히기·기본값 단일화·폰트
  범위·bindings 재생성. locale 4곳(있으면).
- **Phase F 병렬(sonnet+xhigh, R3 후)**: F1=Settings `as` 9지점 제거(editor-pane·untitled-pane·
  app-file-pane) / F2=terminal-session·settings-view·inline-completion `as` 제거 + 폰트 범위 프론트
  반영. bindings union 소비.
- **Phase D 통합(단독)**: 문서(architecture §6.3·data-model·ipc-contract)·qa6 T1 재검 절 + verify + vite build.
- **Phase E 검토(별도 Workflow)**: 4렌즈(opus+xhigh — 정확성: 자원 회수 순서·asset 프로토콜 재등록
  경합 / 계약: 파리티 테스트가 실제 드리프트를 잡는지) → 적대적 → 수정 → 메인 2차 → 커밋(dev).

## 3. 범위 밖 (T1 2차 이후)

- T1-A 잔여(monaco 키바인딩·키맵 브리지 프로바이더)·T1-C 서버상태·T1-D 레지스트리 정리(17건, 최대)·
  T1-F 레이어 이동·T1-G 인프라 하드닝·T1-H 락 IO(위험 높음)·T1-I 도메인 경계(C13)·T1-K 원격 기본거부.
- **T1-H·T1-I 착수 시 각각 동시성 위험·순환 절단 위험 재고지**(사용자 결정 — T1 포함이나 후반·신중).
- X-A 배선(살리기/지우기)·T2 전체·T2-E AppError(별도 캠페인).

## 4. 완료 조건

- `bun run verify` 전체 + vite build. 신설 파리티 테스트 통과·en⊆required·bindings union 반영.
- 4렌즈+적대적+메인 2차. 초점: 파리티 테스트가 인위적 드리프트를 실제로 잡는지(테스트의 테스트)·
  project_close 자원 회수 순서·재오픈 정합·asset 프로토콜 재등록(도입 시) 경합·Settings union 하위호환
  (구 settings.json 의 미지 값)·기본값 단일화가 기존 동작 무변경.
