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

### R 단계 — Rust 전량 (1.B → 1.C → 1.D → 1.E → 1.A Rust)

변경 파일 8종.

| 파일 | 내용 |
|------|------|
| `src-tauri/src/infra/watcher.rs` | §1.B 빈 경로 가드(`error.watcher.emptyRoot`), §1.A `WatchNotification`(`Changes`/`RescanRequired`)·`RESCAN_MIN_INTERVAL_MS`(2_000)·`should_emit_rescan`·`notifications_for_batch`, 콜백 시그니처 `Fn(WatchNotification)` |
| `src-tauri/src/infra/redact.rs` | §1.C `mask_known_secrets` + `SECRET_PATTERN`(`OnceLock<Regex>` 1개, 명명 그룹 13종)·`SECRET_GROUP_NAMES`. `mask_provider_error` 는 존치하되 내부에서 이 마스크를 먼저 적용 |
| `src-tauri/src/domain/git/service.rs` | §1.D `git_command_failed(command, stderr)` — stderr 마스킹 후 message·`detail` 에 같은 값 |
| `src-tauri/src/domain/lsp/commands.rs` | §1.D `toolchain_install_failure_message`(tail 마스킹, 알림·AppError 공통), §1.E `MIN_SIGNALABLE_PGID`=2·`should_signal_process_group` 가드 + 호출부 `child.try_wait()` 생존 확인 |
| `src-tauri/src/domain/notification/commands.rs` | §1.D `masked_notification_text` — `notification_notify` 진입부에서 title·body 마스킹 |
| `src-tauri/src/domain/file/capability.rs` | §1.A `RescanRequired` → `FsRescanRequired` 발행, `Changes` → 기존 `FsChanged` 경로 그대로 |
| `src-tauri/src/domain/git/watch.rs` | §1.A `RescanRequired` → status+refs 둘 다 무효화, `classify_git_changes` 추출 |
| `src-tauri/src/events.rs` · `src-tauri/src/lib.rs` | `FsRescanRequired`(`fs:rescan-required`) 선언 + `collect_events!`·`fanout_remote_events!` 등재, 이벤트 수 단언 24 → 25 |

- **테스트 19종 추가** (전체 1503 → 1522, 통합 테스트 30종 불변).
  - watcher 5종: 빈 경로 거부 / rescan 우선 순서(혼합 배치) / rescan 단독 배치 / 억제된 rescan / `should_emit_rescan` 간격.
  - redact 7종: 발급자 12케이스 양성(표 기반) / URL userinfo / `Authorization: Bearer` / `key=value` / 다중 매치 / **오탐 회귀 8종(40자 sha·UUID·`refs/heads/**`·`node_modules` 긴 경로·일반 URL·pathspec 문구·`npm_config_registry`·일반 로그)** / 무시크릿 원문 보존.
  - git 2종: 실패 stderr 의 message·detail 동시 마스킹(`terminal prompts disabled` 는 생존) / `classify_git_changes` 집계.
  - lsp 3종: tail 의 `_authToken=` 마스킹(원인 문구 생존) / 출력 없을 때 종료 코드만 / pgid 0·1 거부.
  - notification 2종: 제목·본문 마스킹 / 무시크릿 원문 보존.
- **검증**: `cargo fmt --all -- --check` OK · `cargo clippy --workspace --all-targets -- -D warnings` 경고 0 · `cargo test --workspace` 1552 passed / 0 failed · `bun run typecheck` OK · `bun run lint` 0 errors(기존 경고 11 유지) · `bun run format:check` OK.
- **bindings.ts**: `cargo test` 가 재생성. `events.fsRescanRequired = makeEvent<FsRescanRequired>("fs:rescan-required")` + `export type FsRescanRequired = { projectId: ProjectId }` 추가(그 외 변경은 `notification_notify` doc 한 줄). 최초 1회는 `include_str!` 로 컴파일 시점에 박힌 낡은 bindings 때문에 이름 대조 테스트가 실패하고, 재실행하면 통과한다.

#### 이탈 · 판단

1. **`AppErrorKind::Validation` 부재** — `error.rs` 의 kind 는 `Io/NotFound/InvalidArgument/Forbidden/Internal` 5종뿐이라 §1.B 가 지정한 `Validation` 대신 **`InvalidArgument`** 를 썼다(계약이 허용한 "기존 유사 kind").
2. **로케일 키 3종 미추가** — `error.watcher.emptyRoot` 의 en/ko/ja 값은 후속 TS·문서 단계 담당이라 키 이름만 코드에 확정했다. `src-tauri/resources/locales/{en,ko,ja}.json` 의 `error.watcher.registerFailed` 바로 위에 넣으면 된다(현재 카탈로그에 없으므로 fallback 문자열이 노출된다).
3. **테스트 가능성을 위한 순수 함수 추출 5개** — `git_command_failed` · `toolchain_install_failure_message` · `masked_notification_text` · `classify_git_changes` · `notifications_for_batch`. 각각 네트워크(git push), `AppHandle`, 실제 FSEvents 오버플로 없이 마스킹·순서 계약을 결정적으로 잠그기 위한 최소 분리이며, 호출부 동작·출력 문자열은 이전과 동일하다.
4. **`bearer` 그룹은 `Authorization: Bearer <v>` 헤더 형태로 한정** — 접두 헤더 없는 맨 `Bearer <v>` 까지 잡으면 "bearer of bad news" 류 산문이 오탐이 된다. 프로바이더 본문 경로에서는 기존 `mask_bearer_values` 가 계속 그 형태를 처리한다.
5. **`RESCAN_MIN_INTERVAL_MS` 는 `infra/watcher.rs` 모듈 상수** — `WATCH_DEBOUNCE_MS`(constants.rs)와 달리 다른 모듈·문서 계약이 참조하지 않는 워처 내부 스로틀이라 파일 안에 두었다.
6. **`map_event_kind` 는 손대지 않았다** — rescan 은 `EventKind::Other`(`notify` fsevent backend, `Flag::Rescan` 동반)로 오므로 `need_rescan()` 로 판정한다. 기존 `_ => None` 이 그대로 남아야 rescan 이벤트가 가짜 `Modified` 그룹을 만들지 않는다.

#### 남은 작업 (F 단계)

- FE `app/providers/ipc-sync-provider.tsx` 의 `events.fsRescanRequired` 구독·광역 무효화(§1.A) — **현재 구독자가 없어 이벤트는 발행되지만 소비되지 않는다.**
- 로케일 키 `error.watcher.emptyRoot` ×3.
- 문서: `ipc-contract.md` 이벤트 표 · `explorer-sidebar.md` 워처 절.

### F 단계 — 프론트 + 로케일 + 문서 (§1.A FE · §1.B 로케일 · 문서)

R 단계가 남긴 3건을 모두 닫았다. Rust 로직은 손대지 않았다(유일한 예외는 로케일 필수 키 목록 1줄 — 아래 이탈 2번).

| 파일 | 내용 |
|------|------|
| `src/app/providers/ipc-sync-provider.tsx` | §1.A FE — `rescanInvalidations(projectId)`(순수 함수) + `useTauriEvent(events.fsRescanRequired, …)` 구독 |
| `src/app/providers/ipc-sync-provider.test.ts` | `rescanInvalidations` 테스트 3종 |
| `src-tauri/resources/locales/{en,ko,ja}.json` | §1.B `error.watcher.emptyRoot` ×3 (알파벳 순서상 `error.watcher.registerFailed` 바로 위) |
| `src-tauri/src/domain/locale/service.rs` | 위 키를 `MESSAGE_NAMESPACES` 의 `error` 네임스페이스에 `"watcher.emptyRoot"` 1줄 등재(이탈 2번) |
| `docs/ipc-contract.md` | file 도메인 이벤트 목록에 `fs:rescan-required` 1항 + 배치 절 "d-57 인프라 하드닝 웨이브 1"(발행 조건·순서·최소 간격·FE 무효화 집합·`.git` 워처의 기존 이벤트 재사용·로케일 키·마스킹 3지점) |
| `docs/features/explorer-sidebar.md` | §2.3 워처 절에 rescan 항목 |
| `docs/features/git.md` | §7 에 `git_command_failed` stderr 마스킹 1항 |
| `docs/features/lsp.md` | §2 에 툴체인 설치 실패 tail 마스킹 · 프로세스 그룹 시그널 가드 2항 |
| `docs/debugging.md` | §7 진단 팁에 `[redacted:<name>]` 표기 해설 |

- **무효화 집합**: `TREE.ROWS(projectId)` · `SEARCH.PROJECT_FILES(projectId)` · `GIT.PROJECT(projectId)`(+ 기존 `isGitQueryScopeMutable` predicate 재사용) · `FILE.ALL`. 계약 §1.A 가 적은 `TREE.PROJECT` 는 실제 키 이름이 `QUERY_KEY.TREE.ROWS` 다(`shared/constants/query-key.ts`). 순수 함수 `rescanInvalidations` 가 `{ queryKey, matchesQueryKey? }` 목록을 반환하고 핸들러가 그대로 `invalidateQueries` 에 넘긴다 — 집합 자체를 테스트로 잠그기 위한 분리다.
- **테스트 3종 추가** (`bun test src/app/providers` 43 pass / 0 fail): 4종 키 집합·순서 / git 항목만 predicate 를 달고 rev 불변 스코프(`commit-files`·`show`)를 제외한다 / 도메인 전역 접두사(`TREE.ALL`·`SEARCH.ALL`·`GIT.ALL`)를 쓰지 않아 타 프로젝트 캐시를 건드리지 않는다.
- **검증**: `bun run typecheck` OK · `bun run lint` 0 errors(기존 경고 11 유지) · `bun run format:check` OK · `bun test` 2337 pass / 0 fail(그중 `src/app/providers` 43 pass) · `cargo fmt --all -- --check` OK · `cargo clippy --workspace --all-targets -- -D warnings` 경고 0 · `cargo test --workspace` 1522 passed / 0 failed. `cargo test` 재실행 후 `git diff -- src/shared/api/bindings.ts` 는 R 단계와 동일(F 단계에서 bindings 재변동 0).

#### 이탈 · 판단

1. **`TREE.PROJECT` → `TREE.ROWS`** — 계약이 적은 이름이 코드에 없다(§1.A 표기 정정). 프로젝트 스코프 키는 `QUERY_KEY.TREE.ROWS(projectId)` 하나뿐이라 그대로 대체했다.
2. **"Rust 수정 금지" 의 불가피한 예외 1줄** — `src-tauri/src/domain/locale/service.rs` 의 `MESSAGE_NAMESPACES` 에 `"watcher.emptyRoot"` 를 등재했다. 이 목록은 내장 카탈로그의 필수 키 정본이고 `en_메시지의_모든_키는_required_message_keys에_포함된다` 테스트가 카탈로그와의 일치를 강제하므로, 로케일 키만 추가하면 `cargo test` 가 실패한다(실제로 1회 실패를 확인한 뒤 등재했다). 로직 변경이 아니라 §1.B 로케일 작업의 등록 절차다.
3. **`FILE.ALL` 은 무효화이지 제거가 아니다** — 같은 접두사 아래의 핫엑시트 미러(`FILE.MIRRORS`/`UNTITLED_MIRRORS`)가 rescan 으로 버려지면 안 되므로, `projectClosed` 의 `removeQueries` 와 달리 `invalidateQueries` 만 쓴다(재조회될 뿐 데이터는 유지). 이 판단을 JSDoc 에 남겼다.

#### 남은 작업 · 한계 (F 단계 이후)

- **rescan 이 트리를 완전히 교정하지는 못한다**: `tree_rows` 는 트리 스토어의 현재 상태를 재직렬화할 뿐 이미 캐시된 디렉토리를 디스크에서 다시 읽지 않는다(`domain::tree::service::plan_root_read` 는 캐시에 없는 디렉토리만 계획한다). 따라서 `TREE.ROWS` 무효화로 확실히 교정되는 것은 퀵오픈 인덱스(매번 새 walk)·열린 파일 내용·git 상태이고, **이미 펼쳐 둔 디렉토리의 목록**은 뒤따르는 `fs:changed` 나 명시적 `tree_refresh` 전까지 오버플로 이전 상태로 남는다. 완전 교정은 Rust 쪽(rescan 시 트리 스토어 캐시 무효화) 또는 FE 가 펼친 디렉토리마다 `tree_refresh` 를 도는 후속이 필요하다 — 이번 계약 범위 밖이라 코드 JSDoc·`ipc-contract.md`·이 절에 한계로 명시만 했다.

### R + F 총괄

- **d-57 전체 완료**: §1.A(rescan 이벤트 Rust + FE 배선) · §1.B(빈 경로 가드 + 로케일 3종) · §1.C(`mask_known_secrets`) · §1.D(git stderr · LSP tail · 알림 본문 마스킹) · §1.E(프로세스 그룹 시그널 가드). §1.F 범위 외 항목은 손대지 않았다.
- **표면 변화**: 신규 이벤트 1종(`fs:rescan-required`) · 신규 로케일 키 1종(×3 언어) · 신규 커맨드 0 · 기존 페이로드 변경 0. `bindings.ts` 는 `events.fsRescanRequired` 와 `FsRescanRequired` 타입 추가(+ `notification_notify` doc 한 줄).
- **테스트 22종 추가**(R 19 + F 3). 전체 검증: `cargo fmt --all -- --check` · `cargo clippy --workspace --all-targets -- -D warnings` · `cargo test --workspace` · `bun run typecheck` · `bun run lint` · `bun run format:check` · `bun test` 전부 통과.
