# d-57 — 인프라 하드닝 웨이브 1: FSEvents rescan 수용 · 워처 빈 경로 가드 · 시크릿 마스킹 엔진 · 서브프로세스 stderr 마스킹 · 프로세스 그룹 시그널 가드 (2026-09-06)

> `docs/research/2026-09-06-terminal-agent-deep-dive.md` 종합 웨이브 1 중 인프라·보안 5건(W7-1 · W7-10 · T6-F1 · T6-F2 · T6-F8).
> **d-54·d-56 뒤** Rust 단일 에이전트로 착수한다. 메인이 코드로 근거를 재확인했다(§0).

## 0. 근본 원인 (증거)

- **W7-1 rescan 신호 폐기**: `infra/watcher.rs::map_event_kind` 가 `EventKind::Other` 를 `_ => None` 으로 버린다. notify 가 FSEvents 큐 오버플로(대형 checkout·
  npm install·슬립 웨이크)에서 보내는 유일한 신호(`Event::need_rescan()`, Flag::Rescan)가 통째로 사라져 트리·퀵오픈 인덱스·git status 가 수동 새로고침까지 틀린다.
- **W7-10 빈 경로 가드 부재**: `start_watch` 가 `root` 를 검증 없이 `debouncer.watch` 에 넘긴다(`file/capability.rs`·`git/watch.rs` 두 호출처). 빈 경로는 macOS notify 에서
  CoreFoundation URL 생성 실패 뒤 프로세스 전체를 죽이는 종류의 실패다(심층 방어).
- **T6-F1 마스킹이 휴리스틱 둘뿐**: `infra/redact.rs` 는 `bearer <v>` 치환 + "20자 이상 토큰 문자 런 전체 삭제" 만 있어 40자 sha·UUID·긴 경로까지 지운다 → 진단 가치가
  중요한 경로에는 쓸 수 없다(사용처는 AI 프로바이더 에러뿐).
- **T6-F2 stderr 누출**: `domain/git/service.rs` 의 명령 실패 분기가 stderr 를 message·`detail` 인자에 그대로 싣는다(`https://user:<token>@github.com` 리모트 push/fetch
  실패 시 토큰 노출). `domain/lsp/commands.rs` 설치 실패 tail 이 `emit_install_progress`(→ `native-notification-provider.tsx` OS 알림 본문)와 `AppError` 양쪽에 그대로
  들어간다(`_authToken=` 포함 흔함). 로그는 `tauri_plugin_log` 로 디스크에 남는다.
- **T6-F8 시그널 가드 부재**: `domain/lsp/commands.rs::kill_toolchain_process_group` 이 `kill -TERM -{pid}` 를 검증 없이 fork 실행한다. pid 0/1 이면 TAIDE 자신의 그룹 전체·
  시그널 가능한 모든 프로세스를 향한다.

## 1. 수정 방향

### 1.A W7-1 — `fs:rescan-required` 이벤트

- `infra/watcher.rs`: `start_watch` 콜백 시그니처를 `Fn(WatchNotification)` 으로 바꾸고 `WatchNotification::Changes(Vec<FsChange>) | RescanRequired` 를 준다. 배치에
  `need_rescan()` 이벤트가 하나라도 있으면 `RescanRequired` 를 **먼저** 1회 보낸 뒤 나머지 변경을 보낸다. 프로젝트당 최소 간격 `RESCAN_MIN_INTERVAL_MS`(2_000) 는 순수
  함수 `should_emit_rescan(last, now)` 로.
- `domain/file/capability.rs`: `RescanRequired` → 새 이벤트 `FsRescanRequired { project_id }`(`events.rs`, `fs:rescan-required`) 발행. `domain/git/watch.rs`: 같은 신호에서
  `git:status-changed` + `git:refs-changed` 발행(기존 이벤트 재사용).
- FE `app/providers/ipc-sync-provider.tsx`: `fsRescanRequired` → `TREE.PROJECT(projectId)`·`SEARCH.PROJECT_FILES(projectId)`·`GIT.PROJECT(projectId)`(mutable 스코프 predicate
  재사용)·`FILE.ALL` 광역 무효화. 기존 `fs:changed` 소비자와 `FsChange.kind` 계약은 불변.
- 재현 불가(오버플로 유발 곤란)라 단위 테스트로만 잠근다: `should_emit_rescan` / 배치 분해(rescan 이벤트 + 일반 이벤트 혼합) / FE 무효화 키 집합.
- 문서: `ipc-contract.md` 이벤트 표·`explorer-sidebar.md` 워처 절.

### 1.B W7-10 — 워처 등록 빈 경로 가드

- `start_watch` 진입부에서 `root.as_os_str().is_empty()` 면 `AppError::localized(AppErrorKind::Validation, "error.watcher.emptyRoot", …)` 로 즉시 반환(로케일 키 ×3 추가).
  두 호출처의 상위 검증이 이미 막고 있어도 심층 방어임을 문서에 표기. 테스트 1종.

### 1.C T6-F1 — 명명 패턴 시크릿 마스킹 엔진

- `infra/redact.rs` 에 `mask_known_secrets(text) -> String` 신설. `regex` 는 이미 의존성(신규 0). `OnceLock<Regex>` 하나에 **자격증명 계열만** 명명 그룹으로 합친다:
  GitHub(`gh[pousr]_…`, `github_pat_…`), GitLab(`glpat-…`), OpenAI(`sk-…`), Anthropic(`sk-ant-…`), AWS access key(`AKIA…`), Google API(`AIza…`), Slack(`xox[baprs]-…`),
  Stripe(`sk_live_`/`rk_live_`), npm(`npm_…`), JWT(`eyJ….eyJ….…`), URL userinfo(`://user:secret@` → 비밀번호만), `Authorization: Bearer <v>`,
  `(?i)(api[_-]?key|secret|token|password|passwd)=<v>` 의 값. 매치 범위를 정렬·병합 후 뒤에서부터 `[redacted:<name>]` 로 치환. IP·전화·MAC·긴 식별자는 넣지 않는다.
- 오탐 회귀 테스트 필수: 40자 sha·UUID·`refs/heads/x`·`node_modules/…` 긴 경로·일반 URL 이 **그대로 생존**. 각 패턴 양성 테스트 1종 이상.
- 기존 `mask_provider_error` 는 존치(내부에서 `mask_known_secrets` 를 먼저 적용해도 된다 — 기존 테스트 유지 조건).

### 1.D T6-F2 — 서브프로세스 stderr·알림 본문 마스킹

- `domain/git/service.rs` 실패 분기: `stderr` 를 만든 직후 `mask_known_secrets` 를 통과시켜 message·`detail` 양쪽에 같은 값을 쓴다.
- `domain/lsp/commands.rs` 설치 실패: `tail` 조립 직후 마스킹(`emit_install_progress` message·`AppError` 공통).
- `domain/notification/commands.rs::notification_notify` 진입부: title·body 마스킹(이후 어떤 카테고리가 늘어도 자동 적용).
- 테스트: git stderr 의 `https://user:ghp_xxx@github.com` 가 message·detail 에서 마스킹 / LSP tail 의 `_authToken=…` 마스킹 / notification 본문 마스킹.

### 1.E T6-F8 — 프로세스 그룹 시그널 가드

- `kill_toolchain_process_group(pid)`: `MIN_SIGNALABLE_PGID`(2) 미만이면 no-op + `log::warn!`. 호출부에서 시그널 직전 `child.try_wait()` 로 생존을 확인해 이미 죽은 pid(재사용
  창)에는 보내지 않는다. fork 제거(`killpg`)는 nix 직접 의존 결정 뒤로.

### 1.F 범위 외

T6-F3(AI 프로바이더 본문 마스킹 — 결정 필요)·T6-F4/F5/F6/F7/F9 · W7-2~W7-9(결정 또는 d-54 이후 별도).

## 2. 실행 계획

- 구현 wf(opus·xhigh): Rust 단일 에이전트(1.B → 1.C → 1.D → 1.E → 1.A Rust) → TS 에이전트(1.A FE + bindings 확인 + 문서). 종료 조건은 d-54 와 동일.
- 검토 렌즈에 "마스킹 오탐이 진단을 죽이지 않는가" 를 추가.

## 3. 기록

- (대기)
