# IPC 계약 (view ↔ Rust)

> 아키텍처 §4 의 상세. 타입 생성은 ADR-0011(tauri-specta), 전송 원칙 근거는
> `docs/research/tauri-v2.md`·`performance-memory.md`. **이 문서의 목록이 command·event 의 정본이며,
> 구현 시 추가·변경은 이 문서를 먼저 갱신한다.**
>
> **실측(2026-09-04, 사용성 배치 4 웨이브 1 — `notification_notify`·`notification_open_system_settings`·
> `project_set_display`·`layout_open_tab_in_split` 신규 반영)**: command **183종**
> (raw 3종 포함 **186종**), 원격 분할은 `REMOTE_ALLOWED_COMMANDS`(160) ⊎ `REMOTE_DENIED_COMMANDS`(26).
> event 는 **24종**이다 — 프로젝트 표시 변경은 기존 `ProjectListChanged` 재발행이라 신규 0 이었으나,
> 웨이브 1 리뷰(F-1)가 `terminal:command-finished` 1종을 더했다. 아래는 그 직전(d-50 S8) 실측 기록이다.
>
> **실측(2026-08-29, d-50 S8 — `layout_apply_path_change` 신규 반영)**: command **179종**
> (raw 3종 포함 **182종**), 원격 분할은 `REMOTE_ALLOWED_COMMANDS`(158) ⊎ `REMOTE_DENIED_COMMANDS`(24).
> 아래는 그 직전(d-42) 실측 기록이다.
>
> **실측(2026-08-25, d-42 §3 item d — `search_list_files` 신규 반영)**: command **178종** —
> `src/shared/api/bindings.ts` 의 `__TAURI_INVOKE("...")` 전수(raw 3종 제외) =
> `src-tauri/src/domain/remote/dispatch.rs` 의 `IMPLEMENTED_JSON_COMMANDS` 배열 원소 수와 정확히
> 일치(파리티 테스트 `bindings와_dispatch_테이블은_커맨드_이름_집합이_일치한다` 가 강제). raw 채널
> 커맨드 3종(specta 밖, 아래 "raw 커맨드" 절)까지 합치면 총 **181종**. event 는 **23종**
> (`src-tauri/src/events.rs` 의 `#[tauri_specta(event_name = ...)]` 전수). 원격 dispatch 는 이
> 181종을 `REMOTE_ALLOWED_COMMANDS`(157) ⊎ `REMOTE_DENIED_COMMANDS`(24) 로 완전 분할한다(§원격
> dispatch 정책) — 신규 `search_list_files` 는 형제 커맨드(`search_run`/`search_replace`/
> `search_cancel`, 전부 허용)와 동일하게 허용 테이블에 등재했다(팔레트 퀵오픈이 이미 `tree_rows`
> 로 원격에서도 파일 목록을 조회할 수 있었으므로 민감도 축이 다르지 않다). 아래는 d-42 이전
> (177/180/156/24) 실측 기록이다.
>
> **d-27(2026-08-20) 반영 실측**: command **177종**. **d-38(2026-08-25)** 이 커맨드 총수는 그대로 두고 분할만 옮겼다 — 키링 자격증명을
> 바꾸거나 지우는 4종(`ai_set_token`/`ai_clear_token`/`sync_connect`/`sync_disconnect`)을 허용에서
> 거부로 이동해 `REMOTE_ALLOWED_COMMANDS` 160 → **156**, `REMOTE_DENIED_COMMANDS` 20 → **24**
> 가 됐다(상세는 §원격 dispatch 정책의 "d-38" 절). d-27 배치가 신규 커맨드 1종(`project_list_recent`
> — 프로젝트 영속 기록 전수 조회, Welcome 화면 "최근 프로젝트" 전용)을 더해 176 → **177** 이
> 됐다(허용/거부 분할은 신규 커맨드가 `REMOTE_DENIED_COMMANDS` 로만 등재돼 `REMOTE_ALLOWED_COMMANDS`
> 는 160종 그대로, `REMOTE_DENIED_COMMANDS` 만 19 → **20** 이 됐다) — d-38 이전 상태다. X-A 배치가
> 중복 커맨드 5종(`ide_start`/`ide_stop`/`remote_start`/`remote_stop`/`window_open_auxiliary`)을
> 제거하고 신규 커맨드 1종(`lsp_report_reinitialize_failure`)을 더해 180 → 176 이 됐던 것이 d-27
> 이전 상태다(§"X-A 배선 + 소규모 잔여 청소 배치" 절 참조). 그 이전 실측(180/183/25/163/20)은 더
> 앞선 상태다.

## 1. 공통 규칙

- 3종 패턴: **query**(조회, 부수효과 금지) / **mutation**(의도 전달 → Rust 가 상태 변경 + 이벤트 발행)
  / **event**(Rust→view 변경 알림, 소형 payload). 스트림은 **Channel**.
- 네이밍: command = `{domain}_{action}` snake_case, event = `{domain}:{event-kebab}`.
- 전송 매체 선택 (research 확정):

| 데이터 | 매체 |
|--------|------|
| 일반 조회/조작 | command (`Result<T, AppError>`) |
| 대용량 단발(파일 본문 > 수 MB) | `ipc::Response`(ArrayBuffer) 또는 Channel 청크 |
| 스트림(pty 출력·LSP 메시지·검색 결과) | `ipc::Channel` — pty 는 `InvokeResponseBody::Raw` |
| 상태 변경 알림(저빈도) | event — 대형 데이터 금지, "변경됨" 신호만 |

- 이벤트 payload 에는 가능하면 `revision`(단조 증가)을 실어 늦게 도착한 이벤트로 인한 stale 갱신을
  차단한다(revision gate — performance research §3).
- 에러: 전 command 는 `Result<T, AppError>` — `AppError` 는 `{code, message}` 형태의 6변종
  adjacently-tagged enum(`Io`·`NotFound`·`InvalidArgument`·`Forbidden`·`Internal` 은 `message: string`,
  `Localized` 는 `message: LocalizedError { kind, key, args, fallback }` — 로케일 카탈로그의
  `error.<domain>.<slug>` 키로 번역되는 사용자 노출 에러. `kind` 는 앞의 5변종과 같은 값 집합의
  `AppErrorKind` 라 `code` 기반 분기는 그대로 유지된다)
  (thiserror + Serialize + specta Type, 코드는 `error.rs` 중앙화). 프론트 경계(`shared/api/unwrap-result.ts`
  의 `IpcError`, `entities/file/file.raw.ts` 의 `RawFileReadError`)는 `Localized` 를 `code`(항상
  `AppErrorKind` 5종 중 하나 — `Localized` 자체는 `code` 로 노출되지 않는다)·`message`(fallback)·
  `localeKey`·`localeArgs` 로 정규화한다. 표시는 `shared/lib/ipc-error-message.ts` 의
  `describeIpcError`(일회성 — toast)/`useIpcErrorMessage`(지속 렌더 — 언어 전환에 반응)가 전담: `localeKey`
  가 활성 카탈로그에 존재하면 `t(localeKey, localeArgs)`, 아니면 `message` 폴백. 이관 경과·매핑 표는
  `docs/acknowledge/2026-08-24-d34-apperror-campaign-contract.md` 참고.
- view 구독은 `useTauriEvent`(취소 플래그 내장 — StrictMode 이중 마운트 안전) 훅만 사용.
  Channel 은 소비 위젯이 소유하고 unmount 시 대응 detach command 호출.

## 2. 타입 생성 (ADR-0011)

- 모든 command 에 `#[specta::specta]`, 이벤트는 `tauri_specta::Event` derive.
- `src/bindings.ts` 는 debug 빌드에서 재생성·커밋. entities 레이어만 bindings 를 import 하고
  widgets/features 는 entities 경유(FSD).

## 3. command·event 목록 (도메인별 정본)

각 상세 시맨틱은 해당 `features/*.md` 참조. (C)=Channel 파라미터 포함.

### app (최상위 도메인 — 이전 판 `app_get_info` 누락)

- query: `app_get_info() → AppInfo{ name, version, platform, arch }`. 원래 용도는 부팅 시
  About/타이틀바 표시였으나, **T2-D/F/G 배치(2026-08-20)가 유일한 프론트 소비 경로였던
  `entities/app/{app.ipc,app.query,app.type}.ts` 를 dead code 로 제거**해 현재 프론트 소비처는
  0 이다(백엔드 등록·원격 dispatch 표면만 유지). About/타이틀바 UI 자체가 코드베이스에 없다.
- `app_file_read`/`app_file_write` 는 Wave I 신설(§"Wave I 계약 확정 추가" 절)이라 그쪽에서 다룬다.
- **`app:ready(version)` 는 X-A 배치(2026-08-19)에서 제거됐다** — `.emit()` 호출 지점이 애초에 하나도
  없었고(정찰 확인) 프론트도 구독하지 않아, `events.rs`·`collect_events!`·`fanout_remote_events!` 에만
  등록된 채 발행된 적 없는 죽은 이벤트였다(T2-F dead 후보 판정 확정 — X1#2,
  `docs/acknowledge/2026-08-19-xa-wiring-cleanup-contract.md` §1.2).

### project (`layout-shell.md`)

- query: `project_list`, `project_get`, `project_get_active`
  (`project_get_active` 는 구현 중 추가 — 부팅 시 view 가 활성 프로젝트를 알 방법이 없었다.
  `project:activated` 이벤트는 전환 시점에만 오므로 초기 조회용 query 가 별도로 필요하다.)
- query(신규, d-27): `project_list_recent() → Project[]` — 영속 프로젝트 기록 전수(현재 세션에
  열려 있지 않은 프로젝트 포함)를 `last_opened_at` 내림차순으로 반환한다. `project_list`(현재
  세션의 `ProjectRef[]`만)와 달리 디스크의 `projects/<id>/project.json` 전부를 훑고, 루트가 사라진
  기록은 `rootMissing: true` 로 포함한다(제외하지 않는다). Welcome 화면 "최근 프로젝트" 목록
  전용이며 **원격 dispatch 는 거부**(§"원격 dispatch 정책" 참조, `RemoteDenialPolicy::
  LocalProjectHistoryExposure`) — `project_list`는 원격 허용인 것과 대조적으로, 닫혀 있어 원격
  세션이 알 필요 없는 로컬 프로젝트 히스토리까지 노출하기 때문이다.
- mutation: `project_open(path)`, `project_close(id)`, `project_activate(id)`, `project_reorder(ids)`
  — `project_open`(재열기 포함)·`project_activate` 는 대상 `Project.last_opened_at`(밀리초
  epoch, IPC 시간 규칙)을 갱신하고 `project.json` 에 영속화한다(d-27, `Project` 타입 자체가
  `#[serde(default)]` 로 이 필드를 얻었으므로 IPC 시그니처는 무변경 — 반환 타입 `Project` 안의
  필드 하나가 늘었을 뿐이다).
- mutation(신규, 사용성 배치 4): `project_set_display(projectId, patch: ProjectDisplayPatch)` —
  사이드바 표시 오버라이드(아이콘·1~4자 라벨·`lane1..lane12` 색)를 프로젝트별로 저장한다. `patch`
  의 세 축은 각각 **생략 = 유지 / 빈 문자열 = 해제 / 그 외 = 교체**(설정 도메인의
  `merge_clearable_string` 과 같은 3상태 규약)이고, 규격 위반은 `InvalidArgument` +
  `error.project.displayInvalid` 로 거부되며 그 호출의 다른 축도 적용되지 않는다(부분 적용 없음).
  값은 `projects/<id>/project.json`(정본)과 `session.json` 의 `ProjectRef` 미러 **양쪽**에 저장되고
  (`docs/data-model.md` §20), 완료 시 **신규 이벤트 없이** 기존 `project:list-changed` 를 재발행한다
  — `project_list`/`ProjectRef` 가 `display` 를 실어 나르므로 모든 창·원격 세션이 이 이벤트만으로
  갱신된다. 원격 dispatch **허용**(`project_reorder` 와 같은 등급 — 같은 두 로컬 파일을 쓰고
  원격 세션이 이미 볼 수 있는 것 외의 경로를 드러내지 않는다).
- 조회 반환 타입 변화: `project_list` 의 `ProjectRef` 와 `project_get`/`project_list_recent` 의
  `Project` 에 `display?: ProjectDisplay` 가 늘었다(`#[serde(default)]` → specta optional).
  시그니처는 무변경이다.
- event: `project:opened`, `project:closed`, `project:activated`, `project:list-changed`
  (**`project:focus-kind-changed` 는 X-A 배치(2026-08-19)에서 제거됐다** — 소비자가 0 이면서
  레이아웃 변이 18종마다 무조건 발행돼 이벤트 트래픽만 2배로 만들었다(X1#12,
  `docs/acknowledge/2026-08-19-xa-wiring-cleanup-contract.md` §1.2) — `FocusKind` 타입·
  `layout::service::focus_kind` 도 함께 제거됐다)

### layout (`tabs.md`)

- query: `layout_get(projectId)`
- mutation: `layout_open_tab(projectId, kind, title, target, preview)`(이전 판은 `target?` 뒤에
  `title`·`preview` 두 인자가 빠져 있었다 — 정정), `layout_close_tab(tabId)`,
  `layout_activate_tab(tabId)`, `layout_move_tab(tabId, paneId, index)`,
  `layout_split(paneId, edge: DropEdge, tabId)`,
  `layout_open_tab_in_split(request: OpenTabInSplitRequest)`(사용성 배치 4 신설 — 아래 절 참조),
  `layout_resize(paneId, sizes)`,
  `layout_focus_pane(paneId)`, `layout_pin_tab(tabId, pinned)`, `layout_set_preview(tabId, preview)`
  (7.7 후속 — 아래 절 참조), `layout_reopen_closed(projectId)`,
  `layout_set_view_state(tabId, viewState)`, `layout_set_dirty(tabId, dirty)`,
  `layout_set_terminal_session(tabId, sessionId)`, `layout_open_untitled(projectId, target)`,
  `layout_convert_untitled(tabId, path)`(이 4종은 이전 판에 전혀 없던 기재 누락 — 정정),
  `layout_move_tab_to_window(tabId, target: TabWindowTarget)`,
  `layout_set_shell_view(projectId, patch: ShellViewPatch)`(둘 다 Wave I — 아래 절 참조),
  `layout_apply_path_change(projectId, change: TabPathChange) → TabPathChangeResult`
  (d-50 S8 신설 — 아래 절 참조)
- **`layout_open_tab` 의 `File` kind 선검증(사용성 배치 3, 2026-09-04)**: `kind` 가
  `TabKind::File { path }` 이면 탭을 만들기 **전에** `root_guard::resolve_owning_project` 로
  경계를 확인하고(루트 밖이면 종전대로 `error.path.outsideOpenProjects` = `Forbidden`),
  이어 `std::fs::metadata` 가 파일이 아니면 **탭을 열지 않고**
  `error.file.notFound`(`NotFound`, arg `path`)를 반환한다. 팔레트 퀵오픈이 스테일 인덱스의
  경로를 넘겼을 때 탭은 열리고 본문만 `file_open` 의 `No such file or directory (os error 2)` 를
  보여주던 흐름을 커맨드 경계에서 끊는다 —
  `docs/bug/2026-09-04-quick-open-stale-index-not-found.md`. `layout::service::open_tab` 은
  검증하지 않는다 — 순수 인메모리 트리 그대로이고, 레이아웃 서비스 테스트는 가짜 경로로 탭을 연다.
  존재 검사 자체는 `infra::root_guard::ensure_existing_file(resolved, display_path)` 에 있고,
  **파일 탭을 여는 두 진입점이 모두 이것을 호출한다**: (1) 이 커맨드(UI·원격 세션 전부 — 원격은
  같은 커맨드 함수를 dispatch 하므로 자동 적용), (2) IDE 도구 핸들러의 `openFile`
  (`ide::server::resolve_open_file_target`) — 도메인 경계상 다른 도메인의 커맨드 함수를 부를 수 없어
  `ide::service::ensure_path_within_any_project` 로 경계를 본 뒤
  `layout::service::open_tab_and_finish` 를 직접 부르므로, 같은 게이트를 자기 쪽에서 한 번 더 탄다
  (에러는 그 핸들러 관례대로 `ToolError(RPC_INVALID_PARAMS)` 로 변환된다).
- event: `layout:changed(projectId, revision)` — **`revision` 발행 규약(X1#11 실사, X-A 배치)**:
  프로젝트별 독립 카운터(전역이 아니다)로, `layout::types::ProjectLayout::revision`(`u32`, 새
  레이아웃은 0)이 `layout::service::*` 의 레이아웃을 바꾸는 모든 함수(`open_tab`/`close_tab`/
  `set_view_state`/... 전부, `layout_set_view_state` 포함)에서 정확히 1씩 증가한다.
  `layout::commands::finish_mutation`(모든 레이아웃 커맨드의 공용 응답 경로)이 그 증가된 값을 그대로
  실어 발행하므로 이벤트 페이로드는 항상 그 프로젝트의 최신 revision 이다. `revision` 은
  `layout.json` 에 영속되어(`#[serde(default)]` 로 구버전 파일도 0부터 시작) 앱 재시작을 관통해
  단조 증가한다 — 창(윈도우)이 마지막으로 관측한 revision 을 보관했다가 `revision <= lastSeen` 인
  이벤트를 무시하면 멀티윈도우 stale 갱신 방지 게이트가 된다(프론트 게이트 구현은 F1 후속).
  **클라이언트 규약(d-51 F7 · 감사 §1-5)**: 위 "마지막 관측 revision" 게이트를 통과한 이벤트라도,
  `QUERY_KEY.LAYOUT.DETAIL` 캐시의 revision 이 이미 그 이상이면 리페치하지 않는다
  (`isLayoutEchoAlreadyInCache`). 모든 레이아웃 뮤테이션은 새 `ProjectLayout` 을 **응답으로도**
  돌려주고 `applyFreshLayout` 이 그것을 캐시에 쓰므로, 뮤테이션을 일으킨 창에서 이 이벤트는 순수
  에코이며 `layout_get` 왕복을 한 번 더 할 이유가 없다. 캐시가 이벤트보다 뒤처져 있으면(타 창의
  변경, 또는 아직 도착하지 않은 자기 응답) 종전대로 무효화한다.

### file (`editor.md`)

- query: `file_open(path)` (내용+언어+크기 정책 판정), `file_read_raw(path)`(Response —
  뷰어 모드/대형, raw 커맨드 — 아래 절)
- mutation: `file_save(path, content)`, `file_create(path, isDir)`, `file_rename(from, to)`,
  `file_delete(path)`(휴지통), `file_copy(from, to)`
- **d-50 S2 계약 추가(2026-08-29)** — 커맨드 이름·인자·개수는 모두 불변이라 dispatch 테이블·원격
  정책 분류·커맨드 수 변동은 없다. 바뀐 것은 응답 타입 1필드와 두 mutation 의 거절 조건이다.
  - `OpenedFile` 에 **`encodingLossy: boolean` 순증**(bindings 재생성). 파일 바이트가 UTF-8 이
    아니어서 `String::from_utf8_lossy` 가 U+FFFD 로 손실 디코딩한 경우 `true` 이고, 이때 크기
    티어(`tier`)와 무관하게 **`readOnly` 가 강제**된다 — 손실된 텍스트를 그대로 되쓰면 원본 바이트가
    영구 파손되기 때문이다(감사 §4-A-3). 프론트는 이 필드로 열람 전용 배너 문구를 크기 사유
    (`editor.readOnlyLargeFile`)와 인코딩 사유(`editor.readOnlyLossyEncoding`)로 가른다. 인코딩
    왕복(원 인코딩으로 디코딩·재인코딩) 지원은 백로그.
  - `file_rename`·`file_copy` 는 **목적지에 이미 항목이 있으면 거절**한다 —
    `error.file.destinationExists`(`InvalidArgument`). 이전에는 rename 이 목적지를 조용히
    덮어쓰고 copy 는 파일 덮어쓰기·디렉토리 병합을 했다(감사 §4-A-2 · §4-B A2). LSP
    `WorkspaceEdit` 의 `RenameFile.options.overwrite = true` 도 이제 거절된다(§5 이월 참조).
  - 유일한 예외는 **대소문자만 다른 개명**이다. 루트 가드 정규화가 대소문자 무시 파일시스템에서
    디스크 실표기를 돌려주므로 `readme.md → README.md` 가 `from == to` 로 도착해 조용히 no-op
    이던 것을(감사 §4-A-1) 요청 표기를 되붙여 수행한다. 같은 항목이면 임시 이름
    (`persist::temp_sibling` 형태 — 워처가 이미 그룹에서 걸러낸다)을 경유한 2단 rename 이라,
    프론트에는 요청한 개명 하나로 보인다.
- Hot Exit(기능 확장 3차 — §"기능 확장 3차" 참조): mutation `file_mirror_dirty(projectId, path,
  content) → number | null`(반환값은 Rust 가 쓰기 시점에 직접 stat 한 디스크 mtime baseline — 원래
  시그니처는 프론트가 `diskModifiedMs` 인자로 baseline 을 넘겼으나 Wave B 하드닝의 미러 부활 수정에서
  빠졌다, 아래 "기능 확장 3차 계약 확정 추가 (Hot Exit — B1 Rust)" 절 참조), `file_clear_mirror(projectId,
  path)`, `file_prune_mirrors(projectId, keepPaths)`, `file_mirror_untitled(projectId, tabId, content)`,
  `file_clear_untitled_mirror(projectId, tabId)`, `file_prune_untitled_mirrors(projectId,
  keepTabIds)`(재시작 후 레이아웃 기준 authoritative GC — untitled 탭이 열려 있거나 닫힘 스택에
  남아 있지 않으면 미러 삭제), `file_flush_complete()`(**원격 세션에서는 거부** — §"원격 dispatch
  정책" 참조, 데스크톱 자신의 종료만 재개 가능); query `file_list_mirrors(projectId)`
  → `MirrorEntry[]`, `file_list_untitled_mirrors(projectId)` → `UntitledMirrorEntry[]`
  - 모든 project-scoped 미러 커맨드(`file_list_mirrors`·`file_prune_mirrors`·
    `file_mirror_untitled`·`file_list_untitled_mirrors`·`file_clear_untitled_mirror`·
    `file_prune_untitled_mirrors`)는 `projectId` 가 현재 열린 프로젝트인지
    `root_guard::project_root` 로 검증하고, `tabId` 를 파일명 컴포넌트로 쓰는 두 커맨드는 추가로
    `root_guard::ensure_safe_component` 로 경로 탈출 문자(`/`·`\`·`..`)를 거부한다(경로 조작 방지).
- event: `fs:changed(paths[], kind, fromApp)` (watcher — debounce·배치·echo 플래그; 실제 필드명은
  `origin` 이 아니라 `fromApp: boolean` — 정정). `kind` 는 `FsChangeKind` = `'Created' | 'Modified' |
  'Removed' | 'Renamed'` — 기존 타입 그대로 항상 실려 있었다(신설 아님). `fromApp: true` 인 그룹의
  대부분은 `file_save`/`search_replace`(원자적 쓰기라 watcher 는 이를 `Created`/`Renamed`/
  `Modified` 여러 그룹으로 관측한다)에서 나오므로, 트리 구조까지 바뀌는 연산(생성/이름변경/삭제)만
  선택적으로 refresh 를 유지하고 싶은 소비자는 `fromApp` 단독이 아니라 `kind !== 'Modified'` 를
  함께 봐 스킵 범위를 좁힐 수 있다 — 이 필드가 그 판단에 쓰라고 존재한다.
- **부팅 워처 재부착 직후 합성 발신(d-25, `docs/acknowledge/2026-08-20-boot-watcher-defer-contract.md`)**:
  `domain::project::commands::restore_project_watchers` 가 복원 프로젝트의 파일 워처를 재부착한 직후, 그 프로젝트에
  **열린 `File` 탭 경로가 있으면** `fs:changed(paths: <그 경로들>, kind: 'Modified', fromApp: false)`
  를 실제 디스크 변경 여부와 무관하게 1회 합성 발신한다 — attach 지연 구간(창 표시~이 프로젝트
  워처 attach 완료)에 외부에서 바뀌었을 수 있는 파일의 `FILE.CONTENT` 캐시(`staleTime: Infinity` +
  `refetchOnWindowFocus: false`라 이 이벤트 없이는 무기한 정체)를 보정하기 위함. 신규 이벤트·필드
  없음 — 기존 watcher 방출과 동일한 페이로드 형태를 재사용한다.
  `app:hot-exit-flush-requested(timeoutMs)` (Hot Exit — 종료 인터셉트)
- **`fromApp` echo 마킹 완성(X-A 배치, 2026-08-19)**: T0 감사 시점까지 `fromApp` 은 watcher 가 항상
  `false` 로만 채우는 상수였다(X1#10). `AppState::self_writes`(`infra::self_write::
  SelfWriteTracker`)가 `file_save`/`file_create`/`file_rename`/`file_delete`/`file_copy`/
  `search_replace` 의 성공한 쓰기 경로마다 짧은 TTL(2초)로 마킹하고,
  `domain::file::capability::attach_watcher` 가 watcher 그룹의 모든 경로가 그 마킹과 일치할 때만
  `fromApp: true` 로 확정한다 — 묶음 안에 앱이 쓰지 않은 경로가 하나라도 섞이면 전체를 외부 변경으로
  보수적으로 처리한다(오마킹 방지가 마킹 누락보다 항상 우선). 마킹 누락(TTL 만료·경로 불일치)은
  기존과 동일한 `false` 로 안전하게 낙착된다.
- **Phase E 후속 수정(2026-08-19)**: 최초 구현은 `file_save`/`search_replace` 경로에서 목표(자기
  저장마다 트리 재조회 스킵)를 달성하지 못했다 — `write_atomic`(temp-sibling 생성 후 rename)의
  임시 파일이 watcher 그룹에 그대로 섞여 마킹과 불일치했고, 하나의 저장이 만드는 여러 그룹
  (Created/Modified/Renamed) 이 경로당 1회뿐인 마킹을 서로 나눠 가져 최대 한 그룹만 true 가 될 수
  있었다. 두 가지로 수정했다: (1) `infra::persist::is_temp_sibling` 이 watcher 단계
  (`infra::watcher::group_relevant_changes`)에서 `write_atomic` 의 temp-sibling 경로 자체를
  그룹에서 제외하고, (2) `infra::self_write::resolve_from_app` 이 이제 **한 디바운스 배치 전체**
  (`infra::watcher::start_watch` 의 콜백이 그룹 하나가 아니라 `Vec<FsChange>` 를 통째로 넘긴다)를
  받아 같은 경로가 여러 그룹에 나타나도 마킹 판정을 한 번만 소비하고 모든 그룹에 동일하게
  적용한다.

### tree / search (`explorer-sidebar.md`)

> 이전 판의 `tree_expand`/`tree_collapse`/`tree_node`·`tree:changed` 이벤트·`search_start`/`searchId`
> 는 전부 코드에 없는 이름이었다(정정).

- query: `tree_rows(projectId, offset, limit: number | null) → TreeRowPage` — `limit` 이 X-A 배치
  (2026-08-19)에서 `u32` → `Option<u32>` 로 바뀌었다(§1.3(7)). `None`(TS 쪽 `null`/생략)은 "offset
  이후 전량" 을 뜻한다 — 이전에는 프론트가 `TREE_ROWS_UNBOUNDED_LIMIT = 4_294_967_295`(`u32::MAX`)
  라는 센티널 값을 보내 같은 의미를 흉내냈다. Rust 시그니처는 이번에 바뀌었지만 프론트가 여전히
  그 센티널을 보내도 값 자체(`u32::MAX`)는 그대로 유효하므로 당장 깨지지는 않는다 — 센티널을 실제
  `null` 로 바꾸는 소비 전환은 이 문서 범위 밖(F1 후속).
- mutation: `tree_toggle(projectId, path)`, `tree_reveal(projectId, path)`,
  `tree_refresh(projectId, dir)` — 셋 다 갱신된 `TreeRowPage` 를 **반환값으로 직접** 돌려준다. 트리
  갱신을 알리는 별도 이벤트는 없다(옛 `tree:changed` 는 실재하지 않는다 — 정정).
  - **디스크 읽기 시점**(d-50 S7, 2026-08-29): 세 커맨드 모두 필요한 디렉토리를 `begin_mutation`
    guard 를 잡기 **전에** 블로킹 풀에서 읽고, 잠금 안에서는 그 결과를 꽂기만 한다(§2 H-4).
    따라서 반환 페이지는 "guard 를 잡은 시점"이 아니라 "그 직전 읽은 시점"의 디스크를 반영한다 —
    같은 디렉토리에 대한 두 갱신이 겹치면 나중에 도착한 호출이 더 오래된 목록을 쓸 수 있다.
    자기 교정은 기존과 같다: 뒤따르는 `fs:changed` 가 다시 `tree_refresh` 를 부른다.
    응답 형태(전체 트리 재직렬화)는 이번 배치에서 **불변** — 재설계는 백로그.
  - **`tree_refresh` 는 사라진 하위의 상태를 함께 버린다**(d-50 S7): 갱신된 목록에 없는 자식
    디렉토리 아래의 펼침 상태·캐시를 경로 접두사(컴포넌트 단위 — `src-old` 는 `src` 의 하위가
    아니다)로 정리하고, 자식이 없어진 자식 디렉토리는 캐시된 목록만 버린다. 삭제한 폴더와 같은
    이름으로 다시 만들 때 옛 자식이 되살아나던 것(감사 §4-B C2)의 근본 정리다.
- mutation(C): `search_run(projectId, owner, sessionId, query: SearchQuery,
  onMatch: Channel<SearchFileMatches>) → number`(정정: 이전 판이 말한 Rust 발급 `searchId` 는 없다 —
  `sessionId` 는 **호출자(프론트)가 검색 표면마다 하나씩 발급**해 넘기고, 반환값은 매치 총수다.
  같은 `(owner, sessionId)` 의 새 실행은 이전 실행을 자동 취소한다 —
  `docs/acknowledge/2026-08-15-wave-d-search-nav-contract.md` §3.4), `search_cancel(owner, sessionId)`
  - **채널 페이로드는 파일 단위 배치**(d-50 S1a, 2026-08-29): 이전에는 매치 1건 = 채널 메시지 1건
    (`SearchMatch { path, line, ... }`)이라 1만 매치가 1만 메시지 + 1만 번의 `path` 재직렬화였다.
    지금은 파일 1개 = 메시지 1건인 `SearchFileMatches { path, matches: SearchLineMatch[] }` 이며,
    `SearchLineMatch` 는 옛 `SearchMatch` 에서 `path` 만 뺀 형태다(나머지 필드·단위 규약 동일).
  - **순서 계약**: 한 파일 안의 `matches` 는 항상 소스 오름차순(프론트의 컨텍스트 줄 중복 제거가
    이 전제에 의존한다). **파일 간 도착 순서는 비결정**이다 — 스캔이 `ignore` 크레이트의
    `build_parallel()` 병렬 워크이기 때문이며, 안정 정렬이 필요한 소비처는 스스로 정렬해야 한다
    (프론트는 VS Code 와 같이 "찾는 대로 표시"라 정렬하지 않는다).
  - **크기 상한**: `constants::REFUSED_FILE_BYTES`(50MB) 이상인 파일은 스캔하지 않는다 — `file_open`
    이 열기를 거절하는 것과 같은 경계라 "열 수 있는 파일 = 검색되는 파일"이 유지된다. 바이너리
    판별은 앞 8KB 만 먼저 읽어 수행하고, 통과한 파일만 전량을 읽는다.
- mutation: `search_replace(projectId, query: SearchQuery, replacement, paths?) →
  SearchReplaceResult { changedFiles, replacedMatches, skipped: ReplaceSkippedFile[],
  skippedCount }`(이전 판은 `projectId` 인자가 빠져 있었다 — 정정). `skipped`/`skippedCount` 는
  d-50 S1a 신설(감사 §4-B C10) — 치환하지 못한 파일을 사유(`ReplaceSkipReason`:
  `tooLarge`·`binary`·`notUtf8`·`unreadable`·`writeFailed`)와 함께 돌려준다. `skipped` 목록은
  `REPLACE_SKIP_REPORT_LIMIT`(50)개까지만 싣고 `skippedCount` 가 전체 수다. **매치가 없었을 뿐인
  파일은 스킵이 아니며 세지 않는다.**
- Wave D: `TabKind::SearchEditor { query }`(신규 tab variant, "Search Editor") — 결과 목록은 저장하지
  않고 쿼리만 레이아웃에 영속화한다. 복원 시 같은 `search_run` 으로 재검색한다(대량 `SearchMatch[]`
  를 레이아웃 JSON 에 싣지 않기 위함).
- query: `search_list_files(projectId) → string[]`(d-42, 2026-08-25 신규) — 프로젝트 root 이하 전체
  파일의 **절대 경로**를 반환한다(`TreeRow.path` 와 동일 규약). 팔레트 파일 퀵오픈 전용 인덱스 —
  `tree_rows`(트리에 이미 로드된 항목만)와 달리 `domain::search::service::build_walk`(`search_run`과
  동일 워커)로 프로젝트 전체를 매번 새로 순회해, 트리에서 한 번도 확장되지 않은 폴더의 파일도 찾는다
  (`command-palette.md` §3 요구사항 — `docs/acknowledge/2026-08-25-d42-e2e-defects-contract.md` §3
  item d). `respect_gitignore` 인자가 없다 — 항상 `false` 로 순회한다(`IGNORED_DIR_NAMES`만 제외),
  트리가 보여주는 파일과 퀵오픈이 찾는 파일의 집합을 항상 일치시키기 위함(`search_run`/
  `search_replace` 는 쿼리별 `respect_gitignore` 토글이 있어 다르다). 결과에 **상한이 없다** —
  `search_run` 의 `SEARCH_MATCH_LIMIT`·`tree_rows` 의 페이지네이션과 달리 의도적 결정이다:
  퀵오픈 인덱스는 상한을 두는 순간 상한 밖 파일이 검색 불가가 되어(자르면 정합성 파괴) 결함
  d 를 다른 형태로 재도입한다. 절단 없는 전체 목록이 규약이며, 매칭·표시 상한은 FE
  (`FILE_RESULT_LIMIT`)가 담당한다(d-42 검토 L2-3 판정 — 계약 §4).
  **비-UTF8 경로는 제외한다(사용성 배치 3, 2026-09-04)**: 예전에는 `to_string_lossy` 로 손실
  변환해 U+FFFD 가 섞인 **디스크에 없는 경로**를 목록에 실었고, 그 행을 고르면 눈에 보이는
  파일이 "찾을 수 없음"으로 실패했다. 이제 `to_str()` 이 `None` 인 엔트리를 버린다 — 경로를
  `String` 으로 나르는 `TabKind::File`·`OpenedFile` 규약상 어차피 열 수 없는 파일이므로, 빼는
  것이 정직한 목록이고 "이 목록의 모든 경로는 열 수 있다"는 계약(위 `layout_open_tab` 선검증의
  전제)이 유지된다.

### git (`git.md`)

> 커맨드 **41종**(query 16 · mutation 25). Wave C(`docs/acknowledge/2026-08-15-wave-c-git-contract.md`)
> 가 기존 27종에 13종(3-way 충돌 해소·hunk/line 단위 stage·커밋 상세·파일 히스토리·revert·tag·원격
> 브랜치 checkout)을 더했다. 이전 판은 이 13종 전부와 `git_current_user`가 누락돼 있었고, **`git_refs`
> 라는 query 커맨드는 애초에 코드에 존재한 적이 없다**(정정 — refs 변경 "알림"은 `git:refs-changed`
> 이벤트가 담당하고, 실제 목록은 `git_branches`/`git_tags`/`git_stash_list` 를 각각 호출해 받는다).

- query: `git_status(projectId)`, `git_gutter(projectId, path)`,
  `git_blame_range(projectId, path, from, to)`,
  `git_diff_file(projectId, path, mode: DiffMode, beforePath?)`(d-50 S3 — `beforePath` 는 **원본(왼쪽)
  측만** 결정하고 오른쪽은 항상 `path` 다. 개명은 두 축의 경로가 다르므로 — 스테이지된 개명은 HEAD
  항목이 옛 경로에, 스테이지되지 않은 개명은 인덱스 항목이 옛 경로에 있다 — 양쪽을 새 경로로 읽으면
  왼쪽이 비어 파일 전체 추가로 보인다(감사 §4-B B11). `null` 이면 양쪽 모두 `path` 를 쓰는 기존
  동작이며, 개명이 아닌 모든 행에 대해 그것이 정답이다),
  `git_diff_staged_text(projectId) → StagedDiffText{ diffText, truncated, skippedFiles, usedFallback }`
  (Wave G — `ai.md` §4/§7 의 AI 커밋 메시지 생성 전용 소비처, unified diff 텍스트 + 32KiB 상한 +
  바이너리/lock 파일 제외; `usedFallback` 은 Wave H 신설 — staged 델타 0건이면 HEAD↔워킹트리(untracked
  포함) 전체 변경으로 폴백했다는 표시, 동일 제외·상한 규칙이 폴백 diff 에도 적용된다),
  `git_show_file(projectId, rev, path)`, `git_log(projectId, skip, take)`,
  `git_ahead_behind(projectId)`(**예약** — UI 미구현, 프론트 호출자 0. X-A 배치에서 중복 커맨드
  제거 검토 대상이었으나 대체 표면이 없어 유지가 확정됐다),
  `git_remotes(projectId)`, `git_stash_list(projectId)`, `git_branches(projectId)`,
  `git_current_user(projectId) → string | null`, `git_tags(projectId) → TagInfo[]`(Wave C),
  `git_conflict_sides(projectId, path) → ConflictSides{ base?, ours?, theirs?, workdir }`(Wave C —
  index conflict entry 의 stage 1/2/3 blob + workdir 현재 내용, base 없는 add/add 충돌은
  `base: null`), `git_commit_files(projectId, rev) → CommitFile[]`(Wave C — 부모 tree 대비 변경 파일,
  병합 커밋은 first-parent 기준·초기 커밋은 empty tree 대비), `git_file_log(projectId, path, skip,
  take) → LogEntry[]`(Wave C — 파일 단위 히스토리, `--follow` rename 추적은 범위 밖)
- mutation: `git_init(projectId)`, `git_stage(projectId, paths)`, `git_unstage(projectId, paths)`,
  `git_discard(projectId, paths)`, `git_commit(projectId, message, opts: CommitOptions{ amend?,
  stageAll? })`, `git_push(projectId)`, `git_pull(projectId)`, `git_fetch(projectId)`(**예약** — UI
  미구현, 프론트 호출자 0),
  `git_undo_last_commit(projectId)`(**예약** — UI 미구현, 프론트 호출자 0), `git_branch_create(projectId, name, checkout)`,
  `git_branch_checkout(projectId, name)`, `git_branch_delete(projectId, name, force)`,
  `git_stash_push(projectId, message?)`, `git_stash_apply(projectId, index)`,
  `git_stash_drop(projectId, index)`, `git_discard_hunk(projectId, path, hunkStart, hunkEnd)`,
  `git_stage_hunk(projectId, path, hunkStart, hunkEnd)` / `git_unstage_hunk(projectId, path, hunkStart,
  hunkEnd)`(Wave C — git2 `Repository::apply` 로 선택 hunk 만 index 에/에서 적용, stage/unstage 대칭),
  `git_stage_lines(projectId, path, lineStart, lineEnd)` / `git_unstage_lines(projectId, path,
  lineStart, lineEnd)`(Wave C — hunk 가 아니라 선택 라인 단위로 patch 재합성), `git_resolve_conflict(
  projectId, path, content)`(Wave C — workdir 기록 + index stage 0 으로 충돌 자동 해소),
  `git_revert_commit(projectId, rev) → RevertOutcome{ conflicted, conflictedPaths[], conflictedAbsPaths[] }`
  (Wave C — 충돌 시 conflicted 상태로 착지해 위 3-way 흐름과 연계; `conflictedAbsPaths` 는 절대경로
  동봉본으로, 호출부(`commit-graph.tsx`)가 파일 도메인이 절대경로를 요구하는 `onOpenFile` 에 넘긴다),
  `git_tag_create(projectId, name, target, opts:
  TagCreateOptions{ message?, annotated? })` / `git_tag_delete(projectId, name)`(Wave C — annotated
  가 기본값, `message` 없으면 lightweight), `git_checkout_remote_branch(projectId, remoteRef)`(Wave C
  — 로컬 추적 브랜치 생성 + upstream 설정 + checkout, 동명 로컬 브랜치가 있으면 에러 없이 그냥
  checkout)
- event: `git:status-changed`, `git:refs-changed` — **`git:operation-progress`/`git:operation-finished`
  이벤트는 코드에 존재하지 않는다**(이전 판의 기재 오류, 정정).
- **부팅 워처 재부착 직후 합성 발신(d-25)**: `domain::project::commands::restore_project_watchers` 가 복원 프로젝트의
  git 워처를 재부착한 직후, `git:status-changed` 를 실제 git 상태 변경 여부와 무관하게 1회 합성
  발신한다(위 `fs:changed` 항목과 같은 attach 공백 보정 — 이번엔 `entities/git/git.query.ts` 의
  `GIT.PROJECT` 캐시가 대상). `git:refs-changed` 는 함께 발신하지 않는다 — 프런트가 두 이벤트를
  동일한 `QUERY_KEY.GIT.PROJECT(projectId)` 무효화로 매핑해 둘째 발신은 중복이었다. 이 무효화는
  `isGitQueryScopeMutable` predicate 를 거치지 않는 광역 prefix 무효화라 rev-immutable 스코프
  (`git_show_file`/커밋 diff 등, `staleTime: Infinity`)까지 함께 재조회되지만, `invalidateQueries`
  는 그 순간 실제로 마운트되어 있는 쿼리에만 재요청을 일으키므로(비활성 프로젝트·닫힌 탭은
  no-op) 비용은 활성 프로젝트가 화면에 띄우고 있는 쿼리로 한정된다.

**`git_discard_hunk` 의 좌표 계약**: hunk 경계는 `git_gutter` 가 반환한 `GutterHunk { start, end }`
를 그대로 넘긴다. Rust 쪽에서 두 함수가 **같은 diff 옵션과 같은 경계 계산 헬퍼**를 공유하므로
프론트가 받은 좌표와 어긋날 수 없다. 이 불변식이 깨지면 엉뚱한 줄이 되돌려진다. (`git_stage_hunk`/
`git_unstage_hunk`/`git_stage_lines`/`git_unstage_lines` 는 파라미터 형태는 같지만 diff 기준선이
다르므로 — 예: `git_stage_hunk` 는 index↔workdir, `git_gutter`/`discard_hunk` 는 HEAD↔workdir-with-index
— 이 불변식을 그대로 확장 적용하지 않는다.)

**커밋 diff 의 에디터 탭 승격(손 QA 1차 수정, 2026-08-18)**: 커밋 상세 패널에서 파일을 클릭하면
패널 내부가 아니라 `layout_open_tab` 으로 에디터 탭(`TabKind::Diff`, `rev` 필드 있음)을 연다 —
신규 git 커맨드는 없다. 탭이 그 안에서 부르는 데이터도 `git_show_file(rev, path)` /
`git_show_file(parentRev, beforePath ?? path)` 이 두 기존 커맨드 그대로다(`rev`/`parentRev` 는 각각
커밋의 `id`/첫 부모, `beforePath` 는 rename 파일의 커밋 이전 경로 — `origPath`). 상세는 아래 손 QA
절과 `git.md` §"커밋 diff" 참조.

### ai (Wave G 신설 — `ai.md`)

> Wave G 이전부터 자동완성(auto-tab) 커맨드 6종이 존재했으나 이 문서에 정본 항목이 없었다 — 이번에
> 처음 문서화하며, 신규 3종·리네임 1건을 함께 반영한다. 상세 시맨틱은 `ai.md`.

- query: `ai_token_status()` → `AiTokenStatus{ollamaCloud, codex, omlx}`,
  `ai_list_models(provider)` → `AiModelInfo[]`
- mutation: `ai_set_token(provider, token)`, `ai_clear_token(provider)`
- mutation(취소 가능, `requestId` 기반 — 셋이 `AiRequestStore` 하나를 공유):
  `ai_inline_complete(request: AiInlineCompleteRequest)`(auto-tab, FIM-우선/chat-폴백) ·
  **`ai_inline_edit(request: AiInlineEditRequest)`**(신규) · **`ai_commit_message(request:
  AiCommitMessageRequest)`**(신규) · **`ai_request_cancel(requestId)`**(`ai_inline_cancel` 에서
  리네임 — auto-tab 전용이던 취소가 세 커맨드 공용으로 일반화됐다). 세 커맨드 모두 `AiTextResponse
  {requestId, text}` 하나를 공유해 응답한다(d-37 — 이전에는 이름만 다른 `AiInlineCompleteResponse`/
  `AiInlineEditResponse`/`AiCommitMessageResponse` 3타입이었다).
- `AiInlineEditRequest{requestId, provider?, model?, selection, instruction, language, filePath,
  prefix, suffix}` / `AiCommitMessageRequest{requestId, provider?, model?, diffText, recentCommits}`
  — `provider`/`model` 을 생략하면 `Settings.aiProvider`/`aiModel` 로 폴백한다(`ai.md` §1). 텍스트
  응답은 둘 다 실패가 아니라 `text: null`(빈 응답/취소)로 표현될 수 있다. (`AiInlineCompleteRequest`
  는 `provider`/`model` 이 필수다 — 폴백 대상이 아니다.)
- 원격 dispatch: `ai_set_token`/`ai_clear_token` 을 제외한 6종 허용, 이 둘은 **d-38(2026-08-25)에서
  거부로 전환**(`RemoteDenialPolicy::CredentialStoreTampering`) — §"원격 dispatch 정책" 참조.

### terminal (`terminal.md`)

- mutation(C): `pty_spawn(opts: PtySpawnOptions, onData) → sessionId`, `pty_attach(sessionId, onData)
  → subscriptionId`(둘 다 raw 커맨드 — 아래 절)
- mutation: `pty_write(sessionId, data)`, `pty_resize(sessionId, cols, rows)`, `pty_kill(sessionId)`,
  `pty_set_paused(sessionId, paused)`, `pty_detach(sessionId, subscriptionId)`(Wave I 채널 다중화 —
  세션당 여러 창이 구독할 수 있게 되면서 신설. 세션/구독이 이미 없으면 에러가 아니라 already-detached
  로 취급한다)
- query: `shell_profiles`, `terminal_sessions(projectId)`, `resolve_terminal_path(path, cwd)`,
  `pty_default_options(projectId, cwd?)`(7.7 후속 — 아래 절 참조)
- **pause 는 구독과 독립인 세션 상태다(클라이언트 규약, d-51 F5 — 표면 변경 없음)**:
  `pty_set_paused` 는 세션의 reader 스레드를 세우고 `pty_detach` 는 그것을 풀지 않는다(구독자 목록만
  건드린다). 따라서 **구독을 끊는 쪽이 자기가 올린 pause 를 내려야 하고, attach 하는 쪽은
  `pty_set_paused(false)` 로 재동기해야 한다** — 그러지 않으면 detach 시점의 pause 가 남아 자식
  프로세스가 영구히 멈춘다(감사 §4-B D5). 데스크톱 view 는 attach 이펙트의 cleanup·설정 양쪽에서 이를
  수행한다(`terminal-pane.tsx`).
- **`terminal_sessions` 로스터는 프론트가 직접 갱신한다(d-51 F5)**: 쿼리가 `staleTime: Infinity` 라
  스폰(추가)·`terminal:exited`(running=false)·고아 kill(제거)을 캐시에 즉시 써야 재부착 판정이
  사실과 맞는다 — 상세는 `docs/features/terminal.md` §3.1.
- event: `terminal:exited(sessionId, ...)`, `terminal:cwd-changed(sessionId, cwd)`,
  `terminal:command-finished(sessionId, cwd, exitCode, durationMs)`,
  `agent:state-changed(projectId, agents: DetectedAgent[])` — **`terminal_report_cwd` 라는 mutation
  은 코드에 없다**(정정: cwd 보고는 프론트→Rust mutation 이 아니라 Rust→view 이벤트
  `terminal:cwd-changed` 로 흐른다)
- **`terminal:command-finished`(배치 4 웨이브 1 리뷰 F-1, 신규)**: `pty_spawn` 의 reader 콜백이
  `extract_latest_cwd` 와 같은 자리에서 `infra::shell_integration::extract_command_markers` 로 OSC 133
  `C`(출력 시작)·`D`(종료)를 스캔해, 세션별 `Instant` 로 실제 경과 시간을 재고 `D` 마다 발행한다.
  `C` 를 못 본 `D` 는 발행하지 않는다(측정 불가 ≠ 즉시). 프론트 트래커
  (`features/terminal/terminal-osc133.ts`)가 아니라 여기서 재는 이유는 pane 이 활성 탭만 렌더해
  **탭이 배경이면 xterm 인스턴스와 트래커가 통째로 언마운트**되기 때문이다 — 그 상태로 완료된 명령은
  알림이 아예 안 뜨고, 나중에 재부착하면 스크롤백 재생이 같은 `C`/`D` 를 수 ms 간격으로 다시 파싱해
  실행 시간이 0 에 수렴한다. 소비자는 `native-notification-provider.tsx` 하나뿐이다.
- **`terminal:cwd-changed`/`resolve_terminal_path` 배선 완성(X-A 배치, 2026-08-19)**: 이 문서가
  기술해온 흐름은 T0 감사 시점까지 타입·커맨드만 존재하고 실제로 배선되지 않은 죽은 표면이었다
  (`resolve_terminal_path` 호출자 0, `TerminalCwdChanged` 발행 지점 0). `infra::shell_integration`
  의 zsh `_taide_precmd`/bash `_taide_prompt` 훅이 기존 OSC 133 명령 경계 마커와 나란히
  `\e]7;$PWD\e\\`(OSC 7 번호를 재사용하되 `file://host/path` 형식·퍼센트 인코딩 없이 순수 경로만 —
  유일한 소비자가 이 모듈 자신의 파서라 실제 터미널 에뮬레이터 호환을 맞출 필요가 없다)를 매 프롬프트
  렌더링마다 내보내고, `infra::shell_integration::extract_latest_cwd` 가 pty 원시 출력 청크에서 이를
  스캔한다. `terminal::commands::pty_spawn` 의 `on_data` 콜백이 이 값을 `SessionEntry.cwd`(스폰 시점
  cwd 로 초기화됨)와 비교해 실제로 바뀐 경우에만 `TerminalCwdChanged` 를 발행한다(precmd 는 `cd` 여부
  와 무관하게 매 명령마다 실행되므로, 비교 없이 그대로 발행하면 명령마다 이벤트가 튄다). 청크 경계에서
  잘린 시퀀스는 이번 청크에서 감지되지 않을 뿐(다음 프롬프트가 같은 cwd 를 다시 보고해 자연히
  회복) — "최소 배선" 원칙에 따른 의도된 한계다. `resolve_terminal_path(path, cwd)` 커맨드 시그니처
  자체는 무변경(항상 `cwd: String` 을 인자로 받는 순수 함수) — 프론트가 이 이벤트로 세션별 cwd 를
  보관했다가 호출 시 넘기는 소비 배선은 이 문서 범위 밖(F2 후속).

### task (Wave E 추가 — `tasks.md`)

- query: `detect_tasks(projectId)` → `Task[]`(`{ label, command, source, cwd }`) — 프로젝트 루트를
  훑기만 하는 부수효과 없는 조회. `source` 는 `npm`/`make`/`cargo`. 실행은 별도 command 없이
  프론트가 `Task.command` 를 기존 `pty_write` 경로로 포커스(또는 신규) 터미널 탭에 흘려보낸다
  (`terminal.md` §9 재사용 — Run Selected Text 와 동일 전달 경로).

### lsp (`lsp.md`)

> `lsp_detect_servers`·`lsp_resolve_root`·`lsp_install`·`lsp_install_cancel` 4종과 `lsp:install-progress`
> 이벤트는 이 캠페인(Wave A~I) 이전(2026-08-11 전후)부터 있었지만 이 문서에 실린 적이 없었다 —
> 이번에 처음 문서화한다.

- mutation(C): `lsp_spawn(request: LspSpawnRequest, onMessage) → sessionId`
- mutation: `lsp_send(sessionId, message)`, `lsp_stop(sessionId, root?, owner)`,
  `lsp_restart(sessionId)`, `lsp_install(serverId)`(서버 바이너리 자동 다운로드·설치, 진행률은
  `lsp:install-progress` 이벤트로), `lsp_install_cancel(serverId)`
- query: `lsp_sessions(projectId)`, `lsp_detect_servers() → LspServerDetection[]`(설치된 LSP 바이너리
  감지), `lsp_resolve_root(serverId, filePath) → string | null`
- event: `lsp:session-status-changed`, `lsp:install-progress(serverId, phase, receivedBytes?,
  totalBytes?, message?)`

### theme / settings (`theme-system.md`)

- query: `theme_get(themeId)`, `theme_get_current(systemTheme)`, `theme_list`, `settings_get`
- mutation: `settings_set_theme(themeId)`, `settings_update(patch)`
  (구현 시 정정: 초안의 `theme_set(id)` 을 **`settings_set_theme`** 로 확정.
  선택된 테마 id 는 `Settings` 가 소유하는 값이라 mutation 도 settings 도메인에 두는 것이 맞다.
  theme 도메인은 읽기 전용(로드·해석)만 담당한다. `theme_get_current` 는 view 부팅 시
  "현재 설정의 테마"를 한 번에 받기 위한 query 로 추가. **손 QA 1차 수정(2026-08-18)**:
  `settings_set_theme` 이 이제 `follow_system_theme` 을 `false` 로 함께 끈다 — 이전에는 이 플래그가
  켜진 채로 테마를 골라도 `theme_get_current` 가 계속 OS 테마로 재해석해 선택이 조용히 무시됐다.
  아래 손 QA 절·`data-model.md` §Settings 참조.)
- mutation(7.5-D 추가): `theme_save(theme)`, `theme_delete(themeId)`
  — 테마 편집기(`theme-system.md` §7.3)용. 내장 테마 덮어쓰기를 거부하고,
  id 에 경로 구분자(`/ \ .`)가 있으면 거부한다(경로 탈출 방지).
- event: `theme:changed`, `settings:changed`

**`theme_get_current` 가 `systemTheme` 을 받는 이유(7.5-D 정정)**: `followSystemTheme` 판단은
Rust 가 하되 OS 다크/라이트 감지는 view 가 센서 역할을 한다. Rust 가 웹뷰 없이 OS 테마를
읽으려면 플랫폼 분기가 늘어나는데, view 에는 이미 `prefers-color-scheme` 이 있다.
locale 의 `locale_get_current(systemLanguage)` 도 같은 이유로 같은 형태다.

**페이로드 확장(7.10-W7)**: `Theme`·`ResolvedTheme` 에 `tokenColors?: TokenColorRule[]`(원본
TextMate 룰 전량 — 없으면 필드 자체가 생략) 필드가 추가됐다. `ResolvedTheme` 에는 추가로
`syntaxOverrides: string[]`(자식 테마가 스스로 명시한 `syntax` 키 목록 — 프론트가 오버레이 합성
근거로 쓴다)이 붙는다. colors/syntax/terminal 의 토큰 집합·타입은 불변이다 — `theme_get`/
`theme_get_current`/`theme_save` 세 커맨드 모두 이 확장된 형태를 주고받는다(자동 생성 타입은
`bindings.ts`, 상세 스키마는 `docs/theme-system.md` §2.1).

### snippet (Wave F 신설 — `editor.md` §10)

- query: `snippet_list` → `SnippetFile[]`(`{ fileName, snippets: Record<string, SnippetEntry> }`,
  `SnippetEntry = { prefix, body, description?, scope? }` — `prefix`/`body`/`description` 은
  `string | string[]`). `paths.snippets_dir()` 아래 `<languageId>.json` + `*.code-snippets` 를
  스캔하고, 파싱 실패 파일은 스킵 + `log::warn!`(테마 목록과 동일한 견고성 원칙 — 하나가 깨져도
  나머지는 계속 로드).
- mutation: `snippet_save(fileName, content)` → `SnippetFile`, `snippet_delete(fileName)`.
  `fileName` 은 `theme_save` 와 동일한 경로 탈출 방지(`/`·`\`·`..` 거부)에 더해 확장자
  화이트리스트(`.json`/`.code-snippets`)까지 강제한다 — 리스트 스캔(`snippet_list`)과 같은
  predicate 를 공유시켜 "저장은 되는데 목록에 안 보이는 파일"이 생기지 않게 하기 위함
  (`theme_save` 대비 한 단계 더 엄격). `content` 는 JSON 스키마 검증(`SnippetFile.snippets` 형태)
  후 **재직렬화 없이 원문 그대로** 저장 — 프론트가 조립한 문자열이 그대로 디스크에 남는다.
- Rust 는 스니펫을 캐시하지 않는다 — 프론트 TanStack Query(`QUERY_KEY.SNIPPET`)가 캐시를 소유하고
  save/delete 성공 시 invalidate 로 다시 받는다. 파일 워처(외부 편집 즉시 반영)는 1차 제외.
- `system_open_app_data_path(kind)` 의 `AppDataPathKind` 에 `"snippets"` 추가(아래 7.10 항목 갱신).
- settings: `editorSemanticHighlighting`(기본 true) · `editorFormatOnType`(기본 false) ·
  `editorFormatOnPaste`(기본 false) · `emmetEnabled`(기본 true) 필드 4종 추가(전부
  `SettingsPatch`/`emptySettingsPatch`/sync 동반 — 상세는 `data-model.md`).

### locale (7.5-H 신설 — `acknowledge/2026-08-06-i18n-and-session-findings.md`)

- query: `locale_list`, `locale_get(localeId)`, `locale_get_current(systemLanguage)`.
  `locale_get` 은 **T2-D/F/G 배치(2026-08-20)가 유일한 프론트 소비처였던
  `entities/locale/locale.ipc.ts` 의 `getLocale` 을 dead code 로 제거**해 현재 프론트 소비처가
  0 이다(백엔드 등록만 유지 — `locale_list`/`locale_get_current` 는 계속 소비됨).
- 메시지는 **flat dotted key**(`"common.cancel"`). 중첩 객체가 아니다 —
  사용자 언어팩이 키 단위로 부분 오버라이드하기 쉬운 구조를 택했다.
- 사용자 팩은 `{app_data}/locales/*.json`. `extends` 로 내장(en/ko/ja)을 상속하고 바꾼 키만 담는다.
- 내장 카탈로그 정본은 `src-tauri/resources/locales/{en,ko,ja}.json`(`include_str!`+OnceLock
  1회 파싱 — T2-I 외부화). required key 계약(`MESSAGE_NAMESPACES`)은 `locale/service.rs` 코드에
  남는다. 키·언어 추가 시 이 4곳(스키마+3 JSON)을 동기한다 — en=ko=ja·en⊆required 파리티
  테스트가 기계 강제한다.

### font (7.5-D 신설)

- query: `font_list` → `FontFamily { name, monospaced }[]`
- `fontdb` 로 시스템 폰트를 열거한다. `monospaced` 플래그로 에디터·터미널 목록을 기본 필터링한다.

### agent (`agent-integration.md`)

> 커맨드 9종. 이전 판은 `release_wait_marker(marker)` 1건만 실었으나 **이 이름의 커맨드는 코드에
> 없다** — 실제 이름은 `agent_release_marker`(정정)이고, 나머지 8종(감지·CLI 설치·hooks 설치)도
> 통째로 누락돼 있었다.

- query: `agent_list(projectId) → ProjectAgents`(pty 프로세스 트리에서 에이전트 감지, unix 1s/
  Windows 2s 폴링), `agent_cli_status() → CliInstallStatus`(`taide` CLI 심링크 설치 여부, macOS
  전용), `agent_hooks_status(projectId, agentName) → AgentHooksStatus`,
  `agent_pending_external_opens() → ExternalOpenRequest[]`(drain — "기능 확장 1차" 절 참조)
- mutation: `agent_release_marker(marker)`(외부 에디터 왕복 탭 닫힘 → 마커 삭제,
  `agent-integration.md` §2.2), `agent_cli_install()` / `agent_cli_uninstall() → CliInstallStatus`
  ("기능 확장 1차" 절 참조), `agent_hooks_install(projectId, agentName)` /
  `agent_hooks_uninstall(projectId, agentName) → AgentHooksStatus`
- event: `agent:state-changed(projectId, agents: DetectedAgent[])`,
  `agent:external-open(request: ExternalOpenRequest{ path, waitMarker? })`(콜드스타트 argv·
  single-instance 유입 — **이전 판의 `editor-bridge:open-file` 이라는 이벤트 이름은 코드에 존재하지
  않는다**, 정정: 실제 이벤트명은 `agent:external-open`)

### ide — Claude Code IDE MCP 연동 (`agent-integration.md` §3, 신설 도메인 — 이전 판 전체 누락)

- query: `ide_get_status() → IdeStatus{ running, port, connected, clientCount }`
- mutation: `ide_set_selection(input: IdeSelectionInput{ projectId, path, text, startLine,
  startCharacter, endLine, endCharacter, isEmpty })` / `ide_clear_selection()`,
  `ide_publish_diagnostics(projectId, items: IdeDiagnostic[])`
  (LSP 진단을 MCP `getDiagnostics` 도구로 노출), `ide_resolve_diff(requestId, outcome: "saved" |
  "rejected" | "tabClosed", content)`, `ide_resolve_save(requestId, saved)`,
  `ide_notify_at_mention(path, lineStart, lineEnd)`
  (**`ide_start()`/`ide_stop()` 는 X-A 배치(2026-08-19)에서 커맨드로는 제거됐다** — 유일한 도달 경로가
  이미 `settings_update` 의 `ideIntegrationEnabled` 토글(현 `SettingsToggleObservers` 배선 —
  T1-I 에서 `apply_integration_toggles` 를 조립부 등록 관찰자로 재구조화, 도달 경로 동일)과 부팅
  시 자동 시작(`lib.rs`)뿐이었다(X1#13,
  `docs/acknowledge/2026-08-19-xa-wiring-cleanup-contract.md` §1.2). `ide_start` 는 내부 함수로
  남아 그 두 경로가 계속 호출하고, `ide_stop`(내부적으로 `stop_server` 를 부르는 얇은 래퍼였다)은
  통째로 삭제됐다 — 두 경로 모두 이미 `stop_server` 를 직접 부르고 있었다)
- event: `ide:status-changed(status: IdeStatus)`, `ide:diff-requested(requestId, projectId, oldPath,
  newPath, newContents, tabName)`(Claude 의 `openDiff` 도구 호출 → TAIDE 가 diff 탭을 연다 —
  `TabKind::ClaudeDiff`, "7.7 계약 확정 추가" 절 참조), `ide:save-requested(requestId, projectId,
  path)`, `ide:close-tab-requested(tabName, requestId: string | null)`(Claude 가 `close_tab`/
  `closeAllDiffTabs` 로 스스로 diff 탭을 닫을 때 — `requestId` 는 "7.7 계약 확정 추가" 절에서 이미
  다룬 pending 레지스트리 정리용 필드)

### plugin (`plugins.md`)

- query: `plugin_list`, `plugin_read_grammar(pluginId, languageId) -> string`(7.10-W7 신설 — grammar
  파일 본문을 온디맨드로 가져온다. `plugin_list` 응답에 본문을 인라인하지 않는 이유는 매니페스트
  스캔마다 수 MB 를 흘리지 않기 위함이다. 프론트는 highlighter 생성 시 이 query 로 각 플러그인의
  language 기여를 fetch 해 shiki `LanguageRegistration` 으로 주입한다 — 실패한 플러그인은 스킵 +
  경고)
- mutation: `plugin_reload`, `plugin_install(sourcePath) → LoadedPlugin`,
  `plugin_uninstall(pluginId) → LoadedPlugin[]`(둘 다 Wave I 신설 — "Wave I 계약 확정 추가" 절
  참조). **이전 판의 `plugin_set_lsp_enabled(pluginId, lspId, enabled)` 는 코드에 존재하지 않는
  커맨드였다**(기재 오류, 정정 — 삭제).

### vsix (7.10-W5 신설 — `vsix-theme-import.md`)

- query: `vsix_extract_themes(vsixPath)` → `VsixThemeExtractionResult { extension, themes[] }`.
  `extension`(`VsixExtensionInfo { displayName, publisher, version }`)은 `extension/package.json`
  에서 그대로 읽는다(임포트 출처 표기용 — extension.vsixmanifest XML 은 파싱하지 않는다, 신규
  의존성 회피). `themes[]`(`VsixExtractedTheme { label, uiTheme, rawJson, includeChain[] }`)의
  `rawJson`/`includeChain[].rawJson` 은 **파싱하지 않은 원본 문자열**이다 — 변환(색상 매핑·jsonc
  주석 제거)은 `scripts/convert-vscode-theme.ts` 의 기존 로직을 재사용하는 프론트 몫이다(Rust 는
  추출만). 손상되거나 확장 루트를 벗어나는 개별 테마 항목은 전체 호출을 실패시키지 않고
  건너뛴다(`plugins.md` §3 "하나가 깨져도 나머지는 계속 로드된다"와 동일한 견고성 원칙).
  **`vsixPath` 는 `resolve_within_open_project`(프로젝트 루트 가드)를 거치지 않는다** — 프론트
  `dialog` 로 사용자가 직접 고른 파일 경로이고 Rust 는 그 파일을 읽기만 한다(쓰기 없음). 이는
  `project_open(path)` 가 이미 사용자 선택 경로를 루트 가드 없이 받는 것과 같은 성격이다(프로젝트
  루트를 정하는 진입점 자체이므로 검증할 "루트"가 아직 없다). **원격 dispatch 는 Wave I 에서
  허용→거부로 전환됐다** — §"원격 dispatch 정책" 참조.
- **grammar 는 다루지 않는다**: `vsix_extract_themes` 는 `contributes.themes` 만 읽고
  `contributes.grammars`(TextMate 문법)는 추출하지 않는다 — grammar 는 `contributes.languages`
  (언어 id·확장자)와 쌍으로 가져와야 의미가 있고, TAIDE 의 확장자→언어id 매핑
  (`LANGUAGE_ID_BY_EXTENSION`)이 Rust 컴파일 타임 상수라 런타임 오버레이화가 선행돼야 한다 —
  W7 에서도 범위 밖(`docs/backlog.md`). (Wave I 의 `vsix_import_plugin` 은 grammar/language 기여를
  다룬다 — 아래 "Wave I 계약 확정 추가" 절.)
- 도메인 분리 근거: `domain/plugin` 은 TAIDE 자체 선언적 확장 체계(`{app_data}/plugins/*/
  taide-plugin.json`, 부팅 시 스캔, ADR-0010 "코드 실행 없음")이고 `domain/vsix` 는 VS Code
  확장(.vsix) 이라는 **별개 포맷**을 사용자가 그때그때 선택한 파일에서 1회성으로 추출하는
  기능이다. 스키마·트리거·검증 규칙이 전혀 겹치지 않아(taide-plugin.json 의 경로 탈출 검증은
  플러그인 루트 기준, vsix 는 zip 아카이브 내 `extension/` 루트 기준) `domain/plugin` 에
  얹으면 두 스키마가 뒤섞인다 — 신규 `domain/vsix` 로 분리했다.

### remote (`remote-control.md`)

> 커맨드 5종(X-A 배치 이전엔 7종 — `remote_start`/`remote_stop` 제거, 아래 참조). 이전 판은
> `remote_set_password`/`remote_clear_password` 2종만 개별 절에서 서술했고, 나머지(상태 조회·링크
> 발급·세션 전체 폐기)는 도메인으로 묶여 문서화된 적이 없었다.

- query: `remote_status() → RemoteStatus{ running, port, clientCount, passwordConfigured }`(5초 폴링)
- mutation: `remote_issue_link() → RemoteLinkInfo{ url }`(**원격에서 거부** — §"원격 dispatch 정책"),
  `remote_revoke_sessions()`(모든 세션 즉시 무효화), `remote_set_password(password)` /
  `remote_clear_password()`(둘 다 **원격에서 거부** — "기능 확장 3차 계약 확정 추가 (Remote 비밀번호
  — C1 Rust)" 절 참조)
  (**`remote_start()`/`remote_stop()` 는 X-A 배치(2026-08-19)에서 커맨드로는 제거됐다** — 유일한 도달
  경로가 이미 `settings_update` 의 `remoteAccessEnabled` 토글과 부팅 시 자동 시작(`lib.rs`)뿐이었다
  (X1#13, `docs/acknowledge/2026-08-19-xa-wiring-cleanup-contract.md` §1.2). `ide_start`/`ide_stop`
  과 같은 패턴 — `remote_start` 는 내부 함수로 남고, 얇은 래퍼였던 `remote_stop` 은 삭제됐다)
- event: `remote:state-changed(status: RemoteStatus)` — 발행은 되지만(`remote_start`/`stop_server`
  가 부른다) X-A 시점까지 프론트 구독자가 0 이었다(X1#2 판정: `sync:state-changed` 와 같은 패턴으로
  `ipc-sync-provider.tsx` 에 `queryClient.setQueryData(QUERY_KEY.REMOTE.STATUS, payload.status)` 한
  줄을 추가하는 것이 자연스러워 — `REMOTE.STATUS` 는 이미 5초 폴링 쿼리로 존재한다 —
  **소비 신설을 권장**하고 이벤트는 유지했다. 프론트 배선은 이 문서 범위 밖(F1 후속)).

### sync (설정 gist 동기화 — 신설 도메인, 이전 판 전체 누락. `data-model.md` §6 참조)

- query: `sync_status() → SyncStatus{ connected, hasGist, lastSyncedAt, remoteNewer }`
- mutation: `sync_connect(pat)`(GitHub PAT 로 연결), `sync_disconnect()`, `sync_upload()`(현재 설정을
  gist 로 업로드 — `strip_non_syncable` 로 `remoteAccessEnabled`·`remotePasswordOnlyLogin`·
  `shellOverride`·`remoteAllowedHosts`·`aiOmlxBaseUrl` 를 제외하고 업로드, Wave B §3.1·d-41 —
  업로드 제외 근거: secret gist 는 비공개가 아니라는 리크 모델을 앱 자신이
  `settings.syncSecretGistWarning` 으로 인정하고, `sanitize_optional_url`(settings/service.rs)은
  http/https 스킴만 검증할 뿐 호스트를 제한하지 않는다), `sync_download(force) →
  SyncDownloadResult`(`{ kind: 'applied', status } | { kind: 'conflict', remoteUpdatedAt }` — 로컬이
  더 최신이면 `force` 없이는 충돌로 보고, 다운로드 페이로드도 동일 5필드를 강제 제외한다)
- event: `sync:state-changed(status: SyncStatus)`
- **클라이언트 규약**(d-51 F6 · 감사 §4-B D7): `conflict` 를 받고 사용자가 "원격 가져오기" 를 고르면
  이어지는 `sync_download(true)` 도 **결과를 반드시 보고**한다(성공 toast / 실패 toast). 이 재시도는
  `force` 여도 실패할 수 있고(`RetryGistChanged`·`RetrySyncCompleted` — 둘 다 "다시 시도" 를 요구하는
  오류다), 응답을 버리면 설정이 교체됐는지 아닌지를 알 방법이 사라진다. 팔레트 커맨드
  (`entities/sync/sync.commands.ts`)와 설정 화면(`settings-sync-section.tsx`) 둘 다 이 규약을 따른다.
- `schemaVersion` 게이트(`sync::service::ensure_supported_schema_version`)는 페이로드의
  `schemaVersion` 이 이 빌드의 `SETTINGS_SCHEMA_VERSION`(정책상 `1` 로 동결)보다 **클 때만** 거부한다.
  같은 버전인데 이 빌드가 모르는 필드가 섞인 페이로드는 게이트를 통과하고, 그 필드는 `serde(default)`
  에 의해 조용히 버려진다 — 미지 필드 감지 장치가 아니다(2026-08-18 T1-E, X1#6).

### 7.6 추가 (IDE 핵심 루프)

> git·search 신규 커맨드 목록은 위 `### git`·`### tree / search` 절로 통합했다 — 이 절에 다시 따로
> 적으면 두 출처가 갈라져 이번처럼 문서 부채가 쌓인다(SSOT 유지).

**`git_discard_hunk` 의 좌표 계약**은 위 `### git` 절 말미로 옮겼다.

### 7.7 계약 확정 추가

- system(신설): query `system_usage_get` → `SystemUsage { cpuPercent: number | null, memoryBytes: number }`.
  TAIDE 메인 프로세스만 측정한다(자손 프로세스 미포함). 첫 호출은 `cpuPercent: null`
  (CPU 사용률은 두 번째 샘플부터 유효). 프론트는 `refetchInterval` 폴링으로 주기를 소유하고,
  설정 토글이 꺼지면 호출 자체를 하지 않는다(`enabled`).
- system(7.7 후속): mutation `system_open_path(path)`(기본 앱으로 열기),
  `system_reveal_path(path)`(파일 관리자에서 표시), `system_open_in_browser(path)`(`file://` URL 을
  기본 브라우저로) 3종 신설. 셋 다 실행 전에 경로가 **열린 프로젝트 루트 내부(루트 자신 포함)** 인지
  검증하고, 벗어나면 `InvalidArgument` 를 반환한다. 내부적으로 `tauri_plugin_opener` 의 Rust API
  (`open_path` / `reveal_item_in_dir` / `open_url`)를 호출하므로 capability 권한이 필요 없다.
  프론트는 `@tauri-apps/plugin-opener` 를 import 하지 않는다(§4).
- layout: `layout_set_preview(tabId, preview)` 신설 — `layout_pin_tab` 과 동일한 형태.
  `TabKind::Terminal` 에 `cwd?: string | null` 필드, `TabKind::Diff` 에 `compareWith?: string | null`
  필드 추가(둘 다 기존 데이터와 하위 호환). `TabKind::ClaudeDiff { requestId, path }` variant 신설 —
  **레이아웃 영속 저장에서 제외되는 휘발성 탭**이다(저장 직전 필터링, 사라지는 활성 탭은 인접 탭으로 대체).
  7.7 후속: `layout_reopen_closed` 의 닫은 탭 스택에도 넣지 않는다 — 닫는 순간 pending diff 요청이
  해소되므로 되살리면 수락/거부가 항상 실패하는 좀비 탭이 된다.
  `ide:close-tab-requested` payload 에 `requestId: string | null` 추가 — Claude 가 `close_tab` /
  `closeAllDiffTabs` 로 스스로 diff 탭을 닫는 경로에서 프론트 pending 레지스트리를 정리하기 위한 값이다.
- terminal: `pty_default_options(projectId, cwd)` — `cwd` 가 주어지면 프로젝트 루트 하위인지 검증 후
  사용하고, `null` 이면 기존처럼 project.root 를 쓴다.
- file: `file_delete(path)` 를 실제로 휴지통 이동(`trash` 크레이트)으로 구현
  (기존엔 영구 삭제였고 문서만 휴지통이라 적혀 있었다 — 문서-코드 불일치 해소).
  `file_create(path, isDir)` 는 대상 경로가 이미 존재하면 오류를 반환한다
  (기존엔 폴더 중복 생성이 `create_dir_all` 때문에 조용히 성공했다).
- settings: `editorMinimap`, `showSystemUsage`, `agentStatusBadgeEnabled`, `agentHooksEnabled`,
  `ideIntegrationEnabled`, `ideAutoOpenDiff` 필드 6종 추가(전부 `SettingsPatch`/`emptySettingsPatch` 동반).
  `SETTINGS_SCHEMA_VERSION` 은 1 유지(전 필드 serde default).
- agent: `DetectedAgent` 에 `activity: AgentActivity`(`"idle" | "working" | "awaitingInput" | "unknown"`)
  필드 추가 — 기존 `agent:state-changed` 이벤트 payload 가 그대로 확장된다(신규 이벤트 없음).
  `AgentHooksStatus { installed }` 타입만 추가(커맨드는 후속 단계).
- capabilities: opener 계열 권한(`opener:allow-open-path` / `opener:allow-open-url` /
  `opener:allow-reveal-item-in-dir`)은 7.7 후속에서 **전부 제거**했다. 광역 스코프(`path: "**"`)가
  §4 의 "view 는 fs/shell 플러그인 API 를 직접 호출하지 않는다" 선언과 충돌했기 때문이다 —
  같은 기능은 위 `system_*` 커맨드가 루트 검증을 거쳐 제공한다.

### 7.10 계약 확정 추가 (dead-end UX 스파인)

- system(신설): mutation `system_open_app_data_path(kind)` — `kind` 는 `"plugins" | "themes" | "locales"`
  열거값만 받는다(`AppDataPathKind`). 대상 디렉토리를 `create_dir_all` 로 없으면 만든 뒤
  `tauri_plugin_opener::reveal_item_in_dir` 로 Finder/파일 관리자에 표시한다.
  **`system_reveal_path` 와 달리 `resolve_within_open_project`(프로젝트 루트 가드)를 거치지 않는다** —
  임의 경로 문자열을 프론트에서 받지 않고 열거값 3종(=앱 데이터 디렉토리 3종)으로 입력 자체를
  고정했으므로 경로 탈출이 애초에 불가능한 설계다. 프론트는 플러그인/테마/로케일 설정 패널의
  "폴더 열기" 버튼에서 사용한다. (Wave F: `"snippets"` 추가 — 4종. 설정 SNIPPETS 섹션의
  "Open Snippets Folder" 버튼이 사용한다.)
- git(신설): mutation `git_init(projectId)` — 프로젝트 루트에서 시스템 `git init` 을 실행한다.
  이미 초기화된 저장소에 다시 호출해도 git 자체가 재초기화를 정상 종료(exit 0)로 처리하므로
  에러가 아니라 무해한 동작이다(별도 존재 여부 분기를 두지 않는다). 성공 시 `git:status-changed` /
  `git:refs-changed` 를 발행하고, 캐시된 저장소 루트(`GitStore`)를 무효화해 다음 조회가 새로
  초기화된 저장소를 다시 discover 하게 한다. `git_status` 가 `notARepository` 상태를 반환하는
  화면에서 "저장소 초기화" 버튼으로 이어진다.
- plugin: `LoadedPlugin.error` 타입이 자유 형식 한국어 문자열에서 **안정된 코드 union**
  (`PluginErrorCode = "parse-failed" | "id-mismatch" | "version-mismatch" | "path-escape"`) 으로
  바뀌었다. 프론트는 `settings.pluginError.{parseFailed,idMismatch,versionMismatch,pathEscape}`
  로케일 키로 코드를 문구로 매핑해 렌더링한다(하드코딩된 한국어 메시지를 그대로 노출하던 기존
  동작은 다국어 요구사항과 충돌해 제거).
- plugin(7.10-W7 추가): `PluginErrorCode` 에 grammar 검증 3종 `"grammar-missing" | "grammar-invalid"
  | "grammar-conflict"` 추가(파일 미존재 / JSON 파싱·`scopeName` 누락 / scopeName·languageId 충돌).
  `settings.pluginError.{grammarMissing,grammarInvalid,grammarConflict}` 로케일 키를 en/ko/ja
  4곳(스키마+3언어)에 동기 추가한다.

### 기능 확장 1차 계약 확정 추가 (taide CLI·sticky scroll)

- agent(신설 3종, 디스패치 파리티 137→140): mutation `agent_cli_install` / `agent_cli_uninstall`
  → `CliInstallStatus`(+`dangling: boolean` — 심링크는 있으나 canonicalize 실패 = 재설치 필요).
  macOS 전용(그 외 플랫폼·dev 빌드는 `InvalidArgument` 거부). install 은 멱등(이미 우리 target
  이면 osascript 생략), uninstall 은 `remove_file` → `PermissionDenied` 시에만 osascript,
  `read_link` 가 `taide-cli` 로 끝나지 않으면 거부(타 도구 심링크 보호), 사용자 취소(exit 1 +
  "-128")는 조용한 no-op. query `agent_pending_external_opens` → `ExternalOpenRequest[]`(drain) —
  콜드스타트 argv 와 single-instance 콜백이 적재한 pending 열기 요청을 프론트가 부팅 시 1회 +
  `AgentExternalOpen` 이벤트 수신 시마다 소진한다(이벤트 payload 직접 처리 금지 — 중복 재오픈 방지).
- settings: `Settings`/`SettingsPatch` 에 `editorStickyScrollEnabled: boolean`(기본 true),
  `aiOmlxBaseUrl: string | null`(QA6 후속 1차 — sanitize: http/https 만, trailing slash 제거,
  빈 문자열 패치 = 명시 해제) 추가.

### 기능 확장 2차 계약 확정 추가 (system usage breakdown)

- system(신설): query `system_usage_breakdown` → `SystemUsageProcess[]`.
  `SystemUsageProcess { pid: number, kind: SystemUsageProcessKind, label: string,
  cpuPercent: number | null, memoryBytes: number }`,
  `SystemUsageProcessKind = "app" | "terminal" | "lsp" | "agent" | "other"`.
  TAIDE 프로세스(root pid)와 그 자손 프로세스 전체를 반환한다(터미널 pty·LSP 서버·감지된 에이전트
  포함). pid 분류는 **도메인 라벨 우선**(터미널 foreground pid·LSP 서버 pid·에이전트 pid — 각
  도메인이 이미 알고 있는 매핑)이고, 도메인이 모르는 pid 는 `classify_process_name` 프로세스명
  폴백으로 분류한다(`SHELL_PROCESS_NAMES` → terminal, `KNOWN_LSP_BINARY_NAMES` → lsp, 그 외 other).
  `system_usage_get` 과는 **완전히 분리된 별도 `System` 인스턴스**로 갱신한다 — 두 커맨드가 하나의
  `System` 을 공유하면 두 폴러의 위상차가 sysinfo 의 200ms 최소 갱신 간격보다 좁을 때 앱 pid 의
  CPU 델타가 붕괴한다. 처음 등장한 pid(이전 breakdown 호출에서 본 적 없는 pid)는
  `cpuPercent: null` 로 반환한다(sysinfo 는 한 번도 refresh 하지 않은 pid 의 `cpu_usage()` 를 0.0 으로
  주는데, 이는 진짜 idle 과 구분이 안 되므로 `system_usage_get` 의 `has_previous_sample` 게이트와
  같은 원칙을 pid 단위로 적용한 것). 프론트는 모달이 열려 있는 동안만(`enabled`) `refetchInterval` 로
  폴링하고 `gcTime: 0` 이라 모달을 닫으면 캐시를 버리지만, 백엔드 `System` 인스턴스 자체는 앱
  생애주기 동안 유지되므로 이미 알려진 pid 는 재오픈 즉시 유효한 값을 돌려준다(모달을 반복해 열어도
  값이 붕괴하지 않는다). 디스패치 파리티: 140→141.

### 기능 확장 3차 계약 확정 추가 (Hot Exit — B1 Rust)

> 계약: `docs/acknowledge/2026-08-14-hotexit-remote-password-contract.md` §3.1. 상세 스키마·판정
> 기준은 `docs/data-model.md` §6.

- file(신설 8종, 디스패치 파리티 138→146 — `file_prune_untitled_mirrors` 는 QA6 결함 수정으로
  후속 추가, 아래 "QA6 확정 결함 수정" 참조): query `file_list_mirrors(projectId)` →
  `MirrorEntry { path, content, savedAtMs, diskModifiedMs, conflict }`(디스크 현재 mtime 을
  baseline 과 비교해 Rust 가 `conflict` 를 판정 — 원본이 사라진 항목은 목록에서 제외, 유령 복원
  금지), `file_list_untitled_mirrors(projectId)` → `UntitledMirrorEntry { tabId, content,
  savedAtMs }`. mutation `file_mirror_dirty(projectId, path, content) → number | null`(**Wave B
  하드닝에서 정정**: B1 원안은 프론트가 `diskModifiedMs` 인자로 baseline 을 넘기는 4번째 인자
  구조였으나, 미러 부활 버그 수정(Wave B 하드닝 패키지 §3.2 "미러 부활 A안" — `docs/acknowledge/
  2026-08-15-wave-b-hardening-contract.md`) 과정에서 baseline 을 프론트가 신뢰해 넘기는 대신 Rust
  가 쓰기 시점에 직접 stat 해 반환값으로 돌려주는 구조로 바뀌었다 — 인자에서 빠지고 반환값
  `number | null` 이 됐다), `file_clear_mirror(projectId,
  path)`, `file_prune_mirrors(projectId, keepPaths)`(`keepPaths` 밖의 경로 미러 일괄 삭제 —
  프로젝트 열 때 GC), `file_mirror_untitled(projectId, tabId, content)`,
  `file_clear_untitled_mirror(projectId, tabId)`, `file_flush_complete()`(아래 종료 플러시 완료
  신호 — 인자 없음).
- event(신설): `app:hot-exit-flush-requested(timeoutMs)` — `WindowEvent::CloseRequested` 를
  가로채 emit. view 는 모든 dirty 모델을 미러 mutation 으로 밀어넣은 뒤 `file_flush_complete` 를
  호출해야 실제 종료가 재개된다. `timeoutMs`(= `HOT_EXIT_FLUSH_TIMEOUT_MS`, 2.5초)는 Rust 상수를
  이벤트로 실어 보내 view 가 같은 매직넘버를 따로 들지 않게 한다 — 미응답 시 Rust 가 타임아웃
  폴백으로 강제 종료한다(하드행 방지).
- layout: `TabKind::Untitled` 를 `is_volatile` 대상에서 제외 — untitled 탭이 레이아웃에 영속되고
  닫힌 탭 undo 스택에도 쌓인다(이전에는 재시작 시 소실). `ClaudeDiff` 만 남은 유일한 volatile
  kind.

### 기능 확장 3차 계약 확정 추가 (Remote 비밀번호 — C1 Rust)

> 계약: `docs/acknowledge/2026-08-14-hotexit-remote-password-contract.md` §3.2. 파일:라인 근거는
> 세션 정찰 `recon3-remoteAuth.md`.

- remote(신설 2종, 디스패치 파리티 145→147): mutation `remote_set_password(password)` — 비밀번호를
  `salt$sha256(salt+password)` 형식으로 해시해 keyring(`SecretAccount::RemoteAccess`)에 저장하고
  기존 세션을 전부 무효화한다(신규 의존 0 — 저장은 `settings.json` 이 아니라 keyring, gist 동기화
  화이트리스트에도 없음). `remote_clear_password()` — 비밀번호를 keyring 에서 삭제하고 링크만으로
  접속하던 현행으로 되돌리며, 마찬가지로 기존 세션을 전부 무효화한다. **두 커맨드는 원격 세션에서
  호출하면 항상 거부된다** — `dispatch.rs` 의 `match` arm 이 실제 핸들러를 부르지 않고
  `AppError::Forbidden` 을 즉시 반환한다(원격 세션이 자신의 게이트를 바꾸는 것을 막기 위함).
  파리티 테스트(`bindings와_dispatch_테이블은_커맨드_이름_집합이_일치한다`)는 이름 집합만 보므로
  `IMPLEMENTED_JSON_COMMANDS` 에는 정상 등록되어 있다.
- `RemoteStatus` 에 `passwordConfigured: boolean` 추가(5초 폴링에 편승 — 별도 쿼리 불필요).
  `Settings.remotePasswordOnlyLogin: boolean`(기본 false, sync 화이트리스트 비포함)은 기존
  `settings_get`/`settings_update` 로 그대로 노출된다(Remote 전용 커맨드 신설 불필요). 단
  `settings_update` 는 원격 세션에서도 호출 가능한 커맨드이므로, `dispatch.rs` 의 `settings_update`
  arm 이 원격 요청의 patch 에서 `remotePasswordOnlyLogin` 필드를 `None` 으로 스트립한 뒤 위임한다
  (원격 세션이 자기 게이트 모드는 못 바꾸되 그 외 설정은 정상 변경 가능 — `remoteAccessEnabled` 는
  자가 차단=자기 접근 상실이라 스트립 대상에서 제외). **Wave B 하드닝에서 확장(정정)**: 스트립
  대상이 `remotePasswordOnlyLogin` 1필드에서 `remoteAllowedHosts`·`shellOverride` 를 더한 **3필드**
  로 늘었다 — gist 인바운드(§sync)의 `shellOverride` 미필터가 RCE 급 결함으로 확인되면서
  (`docs/acknowledge/2026-08-15-wave-b-hardening-contract.md` §2·§3.1), `settings_update` 원격 경로도
  같은 필드를 동일 원칙으로 스트립하도록 맞췄다. **d-41(2026-08-25)에서 재확장**: 저장된 OMLX 키의
  전송 대상을 정하는 `aiOmlxBaseUrl` 이 더해져 **4필드**가 됐다. 자세한 허용/거부/스트립 전체 그림은
  §"원격 dispatch 정책" 참조.
- 인증 흐름 재배치(`auth_middleware`, 예외 0개였던 종전 구조에 최초로 예외 추가):
  1. Origin/Host 검사(그대로, Origin 헤더 부재는 허용 — 브라우저 GET 내비게이션 대응).
  2. 유효한 세션 쿠키(`taide_remote_session`, 이제 `HashMap<digest, 만료 Instant>` 로 저장되어
     `REMOTE_SESSION_TTL_MS`(7일)가 지나면 자동 무효화)면 통과.
  3. `GET`/`POST /__taide/login` 은 인증 없이 핸들러로 통과.
  4. `?t=<link_token>` 유효 시: **비밀번호 미설정**이면 (구현상) 토큰을 nonce 로 소모한 뒤 그
     nonce 를 즉시 세션으로 승격해 원 요청을 계속 처리 — 바깥에서 관찰되는 동작은 종전과 동일한
     "링크 클릭 = 즉시 세션". **비밀번호 설정됨**이면 nonce 쿠키(`taide_remote_login_nonce`,
     `REMOTE_LOGIN_NONCE_TTL_MS`=5분)를 발급하고 로그인 폼으로 303.
  5. 그 외: `remotePasswordOnlyLogin` on 이면 로그인 폼으로 303, off 면 401.
  - `consume_link_token` 은 더 이상 세션을 직접 발급하지 않는다 — "토큰 검증·소모 → nonce 발급"
    으로 책임이 좁혀졌고, "nonce 검증 → 세션 발급"은 신설 `promote_nonce_to_session` 이 맡는다
    (기존 세션 승격 테스트 3건이 이 2단 호출로 갱신됨).
- 로그인 `POST`: Origin **필수**(부재 시 즉시 403 — 미들웨어의 관대한 검사와 별개로 CSRF 방어를
  이중화), 잠금 중이면 비밀번호 검증 없이 429 + 남은 초, `remotePasswordOnlyLogin` 이 아니면 nonce
  쿠키 필수, `service::verify_password`(constant-time) 로 검증. 실패 시 지수 백오프 잠금
  (`REMOTE_LOGIN_MAX_ATTEMPTS`=5회 초과부터, `REMOTE_LOGIN_LOCKOUT_BASE_MS`=1초씩 배가,
  `REMOTE_LOGIN_LOCKOUT_MAX_MS`=60초 상한 — 전역 카운터, 세션이 아니라 서버 단위). 성공 시
  세션 쿠키 발급 + nonce 쿠키 제거 + `/` 로 303. axum 이 `form` feature 없이 빌드되어
  (`default-features = false, features = ["http1", "tokio", "ws"]`) `Form` 추출기를 쓸 수 없으므로
  `application/x-www-form-urlencoded` 바디는 `server.rs` 가 신규 의존 없이 직접 파싱한다.
- `login_page.rs`(신설): self-contained HTML(인라인 `<style>`, 스크립트 0 — CSP
  `script-src 'none'; connect-src 'none'`). 문자열은 앱의 `locale::service::builtin_by_id` 를
  그대로 재사용(로그인 페이지 전용 복제본을 만들지 않음) — `settings.language` 로 선택하되
  `"system"`이거나 en/ko/ja 가 아니면 영어로 대체(로그인 페이지는 OS 로케일을 알 방법이 없는
  순수 Rust 서버 렌더라 프론트의 `resolve_language` 흐름과 다름). 팔레트는 중립 다크 기본 +
  `prefers-color-scheme: light` 오버라이드. `X-Forwarded-Proto` 헤더가 `https` 가 아니면
  "이 연결은 암호화되지 않았습니다" 고지를 보여준다(서버 자체는 TLS 를 종단하지 않음 — 루프백
  평문 바인드가 전제, 터널 뒤에 있을 때만 이 헤더로 판별 가능).

### QA6 확정 결함 수정 (Hot Exit·Remote 비밀번호, 2026-08-14)

> 위 두 절("기능 확장 3차 계약 확정 추가")의 초판 구현에서 발견된 결함을 원인 해결한 기록.

- **경로 탈출**: `file_list_mirrors`/`file_prune_mirrors`/`file_mirror_untitled`/
  `file_list_untitled_mirrors`/`file_clear_untitled_mirror` 5종이 `projectId` 검증 없이 바로
  `buffers_dir(project_id)` 를 조립해, 열린 프로젝트가 아닌 임의 `projectId`(`"../.."` 등)로
  호출하면 앱데이터 밖 경로를 읽고/쓰고/삭제할 수 있었다(`file_mirror_dirty`/`file_clear_mirror`
  만 `ensure_within_root` 를 거치던 비대칭). 5종 모두 `root_guard::project_root` 로 "열린
  프로젝트인가"를 먼저 확인하도록 수정했고, `tabId` 를 파일명 컴포넌트로 쓰는 2종
  (`file_mirror_untitled`·`file_clear_untitled_mirror`)에는 신설
  `root_guard::ensure_safe_component`(경로 구분자·`.`/`..` 거부)를 추가했다.
- **원격 종료 제어**: `HotExitFlushRequested` 이벤트가 `fanout_remote_events!` 목록에 있어 원격
  브라우저에도 전파되었고, `file_flush_complete` 는 원격 디스패치가 허용되어 있어 dirty 모델이
  없는 원격 세션이 먼저 회신하면 데스크톱의 플러시가 끝나기 전에 `app.exit(0)` 이 실행될 수
  있었다. 이벤트를 fanout 목록에서 제거하고, `file_flush_complete` 는
  `remote_set_password`/`remote_clear_password` 와 같은 패턴으로 `dispatch.rs` 의 `match` arm 이
  `AppError::Forbidden` 을 즉시 반환하도록 막았다(`IMPLEMENTED_JSON_COMMANDS` 에는 파리티
  유지를 위해 그대로 남긴다).
- **비밀번호 게이트 fail-open**: `server.rs::is_password_configured` 가 키링 조회 오류를 `Ok(None)`
  과 동일 취급해(`.ok().flatten()`), 키체인 접근이 일시 실패하면 비밀번호가 설정돼 있어도
  링크 토큰만으로 즉시 세션이 발급됐다. `Ok(None)` 만 "미설정"으로, 그 외(`Ok(Some)`·`Err`)는 전부
  "설정됨"으로 fail-closed 하도록 수정.
- **⌘Q 가 hot-exit 핸드셰이크를 우회**: macOS 앱 메뉴의 `PredefinedMenuItem::quit` 은
  `NSApplication terminate:` 로 직행해 `WindowEvent::CloseRequested` 를 전혀 발생시키지 않는다
  (tao 의 macOS 델리게이트에 `applicationShouldTerminate:` 훅이 없음 — `applicationWillTerminate`
  가 곧장 `RunEvent::Exit`, 가로챌 지점이 없다). Quit 항목을 커스텀 `MenuItem`(accelerator
  `CmdOrCtrl+Q`)으로 교체해 클릭 시 메인 윈도우의 `close()` 를 호출하도록 바꿔, X 버튼과 동일한
  `CloseRequested` 경로를 타게 했다.
- **심볼릭 링크 경로의 미러 소실**: `mirror_dirty` 가 `MirrorFile.path` 에 `ensure_within_root` 가
  돌려준 canonicalize 된 실경로를 저장해, 심볼릭 링크로 열린 탭의 경로(프론트가 보내는 keepPaths·
  복원 비교 기준)와 어긋나 `prune_mirrors` 가 열려 있는 탭의 미러를 지워버렸다. 저장 위치(파일명
  해시)는 canonicalize 된 경로를 계속 쓰되, `MirrorFile.path`(비교용 필드)는 호출자가 IPC 로 보낸
  원래 경로 문자열을 저장하도록 분리(`storage_path`/`display_path` 파라미터 분리).
- **untitled 미러 GC 부재**: `prune_mirrors` 는 `buffers/` 최상위 파일만 훑어 `untitled/` 하위
  디렉터리를 건드리지 않고, 프론트의 `pruneUntitledContents` 는 재시작 후 비어 있는 메모리
  레지스트리에만 의존해 재시작 이후 닫힌 untitled 탭의 미러가 영구 잔존했다. 신설
  `file_prune_untitled_mirrors(projectId, keepTabIds)` 가 복원된 레이아웃(열린 탭 + 닫힌 탭
  undo 스택)을 authoritative keep 목록으로 삼아 프로젝트 활성화 시 `file_prune_mirrors` 와 함께
  스윕한다.
- **`remote_status` 5초 폴링마다 키링 접근**: `RemoteStatus.passwordConfigured` 조회가 매 폴링마다
  OS 키링을 때렸다. `RemoteStore` 에 `password_configured: bool` 캐시를 추가해 앱 부팅 시 1회,
  `remote_set_password`/`remote_clear_password` 호출 시에만 갱신하고 폴링은 캐시만 읽는다.
- **로그인 nonce 슬롯 1개**: `pending_login_nonce: Option<(Vec<u8>, Instant)>` 단일 슬롯이라 두
  기기가 연달아 링크를 발급받으면 먼저 발급받은 기기의 nonce 가 무효화돼 올바른 비밀번호를
  입력해도 실패로 집계됐다(잠금 카운터 오염). `session_token_digests` 와 같은 방식의
  `pending_login_nonces: HashMap<digest, 만료 Instant>` 로 교체해 여러 개의 진행 중 로그인을
  동시에 허용한다.
- **미러 쿼리 캐시 미동기화**(`FILE.MIRRORS`/`FILE.UNTITLED_MIRRORS`, `staleTime: Infinity`): 저장·
  view-disk·prune 시점에만 무효화되고 500ms 디바운스·flush 쓰기에는 캐시 갱신이 없어, `EditorPane`
  이 `key` 없이 같은 pane 안에서 재사용되는 탭 왕복(A→B→A)에서 (a) 프로젝트 활성화 이후 쓰인
  미러가 캐시에 없어 복원되지 않거나 (b) 캐시된 **오래된** 미러가 최신 편집 위에 재적용되는 두
  경로로 미저장 편집이 소실될 수 있었다. `editor-pane.tsx`/`untitled-pane.tsx` 의 미러 쓰기 경로를
  `persistMirror` 로 통일해, IPC 쓰기가 성공할 때마다 `queryClient.setQueryData` 로 해당 엔트리를
  직접 갱신한다(매 디바운스마다 전체 목록을 `invalidateQueries` 로 재조회하지 않고 캐시만 갱신 —
  I/O 비용 없이 신선도 유지).
- **flush 콜백이 쓰기 완료를 기다리지 않음**: `mirror-flush-registry` 의 `flushAllMirrors` 는
  `await` 를 지원하는데, 등록된 flush 콜백(`editor-pane.tsx`/`untitled-pane.tsx`)이
  `void mirrorDirty(...).catch(...)` 로 Promise 를 버리고 즉시 반환해 `HotExitFlushProvider` 가
  실제 쓰기 완료 전에 `file_flush_complete` 를 호출할 수 있었다. flush 콜백을 `async` 로 바꿔
  `persistMirror`(내부에서 `mirrorDirty`/`mirrorUntitled` 를 `await`)를 반환하도록 수정.

### Wave I 계약 확정 추가 (셸·워크스페이스 — 멀티 윈도우·Zen·AppFile·플러그인)

> 계약: `docs/acknowledge/2026-08-16-wave-i-shell-workspace-contract.md`. 상세: `layout-shell.md`
> §7, `window-chrome.md` §5, `tabs.md` §3.1/§4.4, `plugins.md` §6, `data-model.md` §8.

- **window(신설)**: mutation `window_set_fullscreen(fullscreen)`(호출한 창 자체가 대상 —
  `window: tauri::Window` 가 Tauri 에 의해 자동 주입되므로 라벨 파라미터가 없다, `file_flush_complete`
  와 동일 패턴). 당초 이 절엔 `window_open_auxiliary(projectId, windowSlot) → AuxiliaryWindowInfo{
  label, projectId, windowSlot }`(Rust 가 `editor-<n>` 라벨을 발급)도 있었으나, **X-A 배치
  (2026-08-19)에서 중복 커맨드로 제거됐다** — 코어 로직(`window::commands::open_auxiliary_window`)은
  그대로 남아 아래 `layout_move_tab_to_window` 의 `newAuxiliary` 경로와 부팅 시 보조 창 복원
  (`lib.rs`)이 내부적으로 계속 쓰지만, 그 둘과 표면이 완전히 겹치는 독립 IPC 커맨드로서는 더 이상
  노출되지 않는다(X1#13, `docs/acknowledge/2026-08-19-xa-wiring-cleanup-contract.md` §1.2).
  `AuxiliaryWindowInfo` 타입도 이제 어떤 커맨드 시그니처에서도 도달하지 않아 `bindings.ts` 에
  생성되지 않는다(순수 Rust 내부 타입) — 프론트에서 이 타입·`windowOpenAuxiliary` 를 참조하던 곳은
  X-A 이전에도 0 이었다.
- **layout(추가)**: mutation `layout_move_tab_to_window(tabId, target: TabWindowTarget)` —
  `TabWindowTarget = {kind:'main'} | {kind:'newAuxiliary'} | {kind:'existing', slot}`, 대상 보조
  창이 없으면(`newAuxiliary`) `open_auxiliary_window` 코어를 내부적으로 재사용해 새로 연다. 이동으로
  빈 보조 창은 서버에서 자동으로 닫힌다(`cleanup_emptied_auxiliary_windows`) — 단 탭을 그냥
  닫아서(이동이 아니라) 비게 된 보조 창은 이 경로를 타지 않으므로 자동으로 닫히지 않는다(프론트
  `auxiliary-window-shell.tsx` 가 자기 트리가 비면 스스로 `getCurrentWindow().close()` 하는 것으로
  대칭을 완성한다). `layout_set_shell_view(projectId, patch: ShellViewPatch{zen?, sidebarCollapsed?})`
  — main 창 전용 표시 상태(§`ShellViewState`, `data-model.md` §8).
- **app(신설 도메인)**: query `app_file_read(target: AppFileTarget)` → `string`, mutation
  `app_file_write(target, content)` → (성공 시 `settings:changed` 를 함께 발신하는 것은 `target.kind
  === 'settings'` 일 때만 — Prompt 타깃은 순수 쓰기). `AppFileTarget = {kind:'settings'} |
  {kind:'prompt', id: PromptTemplateId}`(`'auto-tab-default' | 'inline-edit-default' |
  'commit-message-default'`, `ai.md` §5 의 기존 파일과 동일 화이트리스트). 두 커맨드 모두
  `root_guard` 의 프로젝트 루트 검증 대상이 **아니다** — 경로 자체를 Rust `AppPaths` 가 유도하고
  프론트/레이아웃에는 절대 경로가 노출되지 않는다. `app_get_info` 도 같은 `app` 도메인에 속한다
  (§"app" 절 — 이 커맨드는 Wave I 이전부터 있었다).
- **settings(이벤트 실제 배선)**: `event: settings:changed` 는 이 문서에 이미 등재돼 있었지만 Wave I
  이전에는 실제로 발신되지 않았다(§2 확정 사실 7 — "load_settings 는 부팅 1회, 설정 변경 이벤트
  없음"). `settings::commands::apply_and_broadcast` 가 `settings_update`·`sync_download`·
  `app_file_write`(Settings 타깃) 세 진입점의 공용 코어가 되면서 셋 다 이제 실제로 이 이벤트를
  전 창(+원격, `lib.rs` 의 `fanout_remote_events!` 목록에 등록됨) 발신한다. 프론트는
  `ipc-sync-provider.tsx` 에서 이 이벤트로 `SETTINGS.CURRENT` 를 직접 `setQueryData` 한다(무효화가
  아니라 페이로드를 즉시 반영).
- **plugin(추가)**: mutation `plugin_install(sourcePath)` → `LoadedPlugin`(디렉토리 또는
  `.zip`/`.vsix` 아카이브 — 아카이브는 `infra::archive::extract_hardened_zip` 으로 추출, 이미 설치된
  id 면 거부), `plugin_uninstall(pluginId)` → `LoadedPlugin[]`(빌트인 보호 없음).
- **vsix(추가)**: mutation `vsix_import_plugin(vsixPath)` → `LoadedPlugin` — VS Code 확장의
  `contributes.languages`/`contributes.grammars` 를 읽어 `taide-plugin.json` 을 합성한 뒤
  `plugin_install` 과 동일한 검증·등록 경로로 착지시킨다(`plugins.md` §6). 기존
  `vsix_extract_themes` 는 그대로 있지만 **원격 dispatch 는 이번에 허용→거부로 전환**(아래 참조).
- **원격 dispatch 명시 거부(신규 6종, 당시 기준)**: `window_open_auxiliary`(X-A 배치(2026-08-19)에서
  커맨드 자체가 제거되며 이 거부 목록에서도 함께 빠졌다 — 아래 "원격 dispatch 정책" 절이 정본)·
  `window_set_fullscreen`·`layout_move_tab_to_window`·`plugin_install`·`plugin_uninstall`·
  `vsix_import_plugin` — 전부
  "원격 세션은 로컬 디스플레이/파일시스템 다이얼로그가 없다"는 기존 `file_flush_complete`/
  `remote_issue_link` 류 거부와 같은 근거. **`vsix_extract_themes` 도 이번에 허용→거부로 전환**했다
  — 이전에는 원격에서도 임의 로컬 파일 경로를 zip 으로 열어 읽을 수 있었다(§2 확정 사실 8, 제한적
  임의 파일 읽기 표면). `plugin_list`/`plugin_reload` 는 원격에서도 그대로 허용(읽기 전용). 전체
  거부 목록 전수는 §"원격 dispatch
  정책" 참조.
- **채널 다중화(내부 구현, 새 IPC 표면 아님) — pty 는 다중 창 구독, LSP 는 창별 독립 세션(정정,
  T1 3차 배치 R7#6)**: `pty_attach` 는 세션당 구독자 목록을 유지해 여러 창이 같은 pty 세션을 동시에
  구독할 수 있다 — 출력은 전 구독자에 브로드캐스트되고 `send` 가 실패한(창이 닫힌) 구독자는 다음
  브로드캐스트에서 자동 제거된다. **`lsp_spawn`(reuse 경로)은 이 문서의 이전 판이 서술한 것과 달리
  다중 창 구독을 제공하지 않는다** — `find_reusable_entry` 가 세션을 재사용 후보로 내줄 조건에
  `channels.contains_key(owner)`(호출자 자신이 이미 그 세션의 유일한 구독자일 것)를 포함하므로,
  같은 프로젝트/서버라도 *다른* 창의 `lsp_spawn` 은 항상 그 창만의 새 `SessionEntry` 를 받는다 — 같은
  창 안에서(재-spawn, 같은 창의 여러 탭)만 재사용된다. 서로 다른 두 창이 하나의 JSON-RPC 연결을
  공유하면 각 창의 LSP 클라이언트가 독립된 JS 렐름이라 커넥션당 1회여야 하는 `initialize` 를 두 번
  보내고(언어서버가 두 번째를 거부해 그 창의 클라이언트가 영구히 capability 없는 상태로 남는다)
  요청 id 충돌로 다른 창의 대기 요청을 잘못 resolve 할 수 있어, 실현 대신 이 서술을 정정하는 쪽으로
  결정했다(`docs/acknowledge/2026-08-19-audit-t1-batch3-contract.md` §1.0 결정 1 — 창별 독립 세션
  정본화). 대신 **하나의 창 안에서 여러 워크스페이스 루트가 한 세션을 공유**하는 것은 지원한다
  (R7#7) — `lsp_spawn` 은 `spec.shares_sessions` 가 켜진 서버에 한해 같은 창이 이미 연 세션에 새
  루트를 `roots`(루트별 참조 카운트) 로 추가하고 `workspace/didChangeWorkspaceFolders` 알림을 보낸다.
  프론트(`lsp-session-registry.ts`)는 세션 캐시 키에 root 를 포함(`${projectId}::${serverId}::${root}`)
  해 두면서도, 백엔드가 같은 `session_id` 를 반환하면(=합류 허용) 새로 만든 throwaway 클라이언트를
  버리고 기존 세션 그룹에 합류시킨다. 커맨드 시그니처·응답 타입은 변경되지 않았다(진짜 IPC 표면
  변경은 아래 "T1 정비 3차 배치" 절 참조).

### 손 QA 1차 발견 6건 수정 (2026-08-18)

> 계약: `docs/acknowledge/2026-08-18-hand-qa-fix-contract.md`. 사용자 실기 QA 6건 중 IPC 표면에
> 영향을 준 4건(터미널 링크·와일드카드·파일 열기 블로킹·커밋 diff)을 이 절에 담는다. 부트 테마·
> peek 트위스티 2건은 IPC 변경이 없다(`window-chrome.md`/`docs/bug` 참조).

- **system(신설)**: mutation `system_open_external_url(url)` — `http://`/`https://` 접두(ASCII
  대소문자 무시) 화이트리스트 + 제어문자·공백 거부만 검증하고(스킴 파싱용 `url` 크레이트는 도입하지
  않았다), 통과하면 `tauri_plugin_opener::open_url` 로 OS 기본 브라우저를 연다. 터미널
  (`terminal.md` §"링크")의 xterm `WebLinksAddon` 이 ⌘/⌥ 클릭으로 인식한 URL 이 유일한 호출부다.
  **정정(사용성 배치 3, 2026-09-04)**: 호출부는 프론트 단일 진입점 `entities/system/external-url.ts`
  의 `openExternalUrl` 하나로 바뀌었고, 그 뒤에 터미널 링크 3종(플레인 매처·OSC 8 `linkHandler`·
  파일 링크 제외)과 렌더된 마크다운 앵커(`app/providers/external-link-provider.tsx`)가 붙는다.
  Rust 쪽에서도 `infra::navigation_guard::open_new_window_externally` 가 같은 화이트리스트
  (`infra::external_url::validate_external_url`, `domain::system::commands` 에서 이동)를 통과시킨 뒤
  `tauri_plugin_opener::open_url` 을 직접 부른다 — 커맨드 계약 자체는 불변이다
  (`terminal.md` §6.1 · `architecture.md` §4.1).
  **원격에서는 명시 거부**(`system_open_external_url` 행 — 위 "원격 dispatch 정책" 표) — 원격
  세션이 데스크톱 자신의 OS 브라우저를 대신 열게 할 수는 없다는, `remote_issue_link` 자가 확장
  거부와 같은 근거.
- **layout**: `TabKind::Diff` 에 옵션 필드 3종 `rev`/`parentRev`/`beforePath`(전부
  `#[serde(default)]` — 구 `layout.json` 은 필드 없이도 그대로 역직렬화된다) 추가. `rev` 가 있으면
  그 diff 는 워킹트리/스테이지 비교가 아니라 **커밋 diff**(`git_show_file(rev, path)` vs
  `git_show_file(parentRev, beforePath ?? path)`)를 뜻한다 — 전용 `TabKind` 신설은 검토에서
  기각됐고(Wave C L0-2, `docs/acknowledge/2026-08-18-hand-qa-fix-contract.md` §3) 기존 variant
  확장으로 처리했다. 신규 커맨드는 없다 — 위 `### git` 절의 "커밋 diff 의 에디터 탭 승격" 참조.
- **settings**: `remote_allowed_hosts` 항목에 `*.` 접두 와일드카드를 허용한다(RFC 6125, 선두
  1레이블만 매칭 — `*.example.com` 은 `foo.example.com` 은 매칭하지만 `example.com` 자신·
  `a.b.example.com` 은 매칭하지 않는다). sanitize(`is_valid_allowed_host`)는 맨몸 `*`·접미사가
  1레이블뿐인 와일드카드(`*.com`)·선두가 아닌 와일드카드(`foo.*.com`)·레이블에 섞인 와일드카드
  (`*foo.com`)를 전부 걸러낸다. 매칭(`remote::service::host_matches_allowed_entry`)은
  `is_allowed_host`(원격 Host 헤더 검증)와 `is_insecure_connection`(HTTPS 강제 예외) 양쪽이 공유하는
  단일 함수다 — `ends_with` 계열 유사 도메인(`evil-trycloudflare.com` 이 `*.trycloudflare.com` 을
  통과하는 함정)을 구조적으로 배제하려 `split_once('.')` 로 선두 1레이블만 떼어 비교한다.
  `remote_issue_link` 가 쓰는 `format_issue_link_url` 은 **첫 비와일드카드 항목**을 우선하고(와일드
  카드 패턴은 브라우저가 실제로 열 수 있는 호스트가 아니다), 등록된 호스트가 전부 와일드카드면
  루프백 링크로 폴백한다. 프론트 `remote-allowed-hosts-row.tsx` 의 `isValidAllowedHost` 가 같은
  형태 검증을 미러링한다(`remote.allowedHostsDescription` locale 문구도 함께 갱신).
- **lsp**: IPC 표면(커맨드 시그니처)은 변경 없음 — `lsp_stop`/`lsp_restart` 내부의 전역 뮤테이션
  가드 보유 구간이 줄고(가드 하 처리는 동기 북키핑 + `LspStore` 선제거까지만, `shutdown_entry` 는
  가드 밖에서 대기) `shutdown_entry` 의 고정 2s+2s sleep 이 프로세스 종료 폴링(`wait_for_process_exit`,
  상한은 기존 `LSP_SHUTDOWN_TIMEOUT_MS`)으로 바뀌었을 뿐이다. 프론트 `lsp-session-registry.ts` 의
  `releaseLspSession` 도 refCount 0 즉시 dispose 대신 `LSP_SESSION_DISPOSE_GRACE_MS`(5초) 유예 후
  dispose 하도록 바뀌었다(그 사이 재획득하면 타이머 취소 — 파일 전환 시 세션 재사용). 상세는
  `lsp.md` §5·`docs/bug/2026-08-18-lsp-stop-global-lock.md` 참고. **접합부 수정(Phase D + 메인
  2차)**: 유예 도입 직후엔 `flushLspSessionDisposal`(유예를 건너뛰고 즉시 dispose)이 정의만 되고
  어디서도 호출되지 않아, 유예 중이던 세션이 `project_close`(Rust 가 `LspStore` 를 건드리지 않는다)
  나 hot-exit 이후에도 계속 5초를 마저 기다린 뒤에야 정리됐다. Phase D 가 `ipc-sync-provider.tsx` 의
  `events.projectClosed` 핸들러에 `flushLspSessionsForProject(projectId)` 를, `HotExitFlushProvider`
  에 `flushAllLspSessions()`(간접 참조 이유는 아래)를 배선했지만, 그 시점의 `flushLspSessionsForProject`
  는 여전히 유예 타이머가 걸린 세션에만 작용했다 — `events.projectClosed` 는 Tauri IPC 콜백에서
  동기 도착해 React 가 그 프로젝트의 팬을 아직 언마운트하기 전이므로, 도착 시점의 세션은 전부
  `refCount ≥ 1`(유예 타이머 없음)이라 이 배선은 실제로는 항상 아무것도 하지 않는 구조적 no-op
  이었다. 메인 2차에서 `flushLspSessionsForProject` 를 refcount/유예 상태와 무관하게 해당
  프로젝트의 세션을 강제로 즉시 dispose 하도록 근본 수정했다(프로젝트가 닫히는 중이므로 그 세션을
  다시 참조할 팬도 곧 사라짐). 신설 `lsp-session-flush-registry.ts` 를 하나 더 둔 것은
  `hot-exit-flush-provider.tsx`·`ipc-sync-provider.tsx` 가 `lsp-session-registry.ts`(실제 monaco
  worker 를 import 하는 모듈이라 `bun test` 정적 임포트 그래프 해석이 실패한다)를 직접 참조하지
  않게 하기 위해서다 — `lsp-session-registry.ts` 가 자기 모듈 로드 시 `flushAllLspSessionDisposals`
  ·`flushLspSessionsForProject` 를 이 레지스트리에
  스스로 등록한다.

### 원격 dispatch 정책 (기본 거부 · 명시 허용 목록 · 부분 스트립)

> **T1-K(2026-08-19, `docs/acknowledge/2026-08-19-audit-t1k-default-deny-contract.md`)**: 이 절의
> 구조가 "기본 허용(목록에 없으면 통과)"에서 **"기본 거부(명시 등재 전까지 원격 불가)"** 로
> 뒤집혔다. **정책(어떤 커맨드가 허용/거부인지)은 이 배치에서 전혀 바뀌지 않았다** — 바뀐 것은
> 강제 메커니즘뿐이다: 이전에는 새 `match` arm 을 추가하기만 하면 그 커맨드가 자동으로 원격
> 허용됐다(무증상 위험). 이제는 `src-tauri/src/domain/remote/dispatch.rs` 의
> `REMOTE_ALLOWED_COMMANDS`(명시 허용, 157종 — d-42 이전 156종) 또는 `REMOTE_DENIED_COMMANDS`(명시
> 거부, 24종) **둘 중 하나에 이름을 등재해야만** `dispatch()`/`dispatch_raw()` 가 그 커맨드를
> 실핸들러로 위임한다 — 등재를
> 잊으면 `RemoteDenialPolicy::Unclassified` 로 즉시 거부되고, 완전 분할 파리티 테스트
> (`허용_테이블과_거부_테이블은_전체_커맨드를_교집합_없이_정확히_분할한다`)가 등재 누락 자체를
> 컴파일 타임이 아니라 테스트 실패로 잡는다(두 테이블의 합집합이 전체 커맨드 집합과 정확히 같아야
> 하고, 교집합은 0이어야 한다).
>
> 전체 커맨드 집합(181종, d-42 이전 180종) = `dispatch.rs` 의 `IMPLEMENTED_JSON_COMMANDS`(178종, d-42
> 이전 177종, specta/JSON 경로 — 파리티 테스트 `bindings와_dispatch_테이블은_커맨드_이름_집합이_
> 일치한다` 가 `bindings.ts` 와 강제 일치시킨다) + `lib.rs` 의 `RAW_CHANNEL_COMMANDS`(3종 —
> `pty_spawn`/`pty_attach`/`file_read_raw`, collect_commands! 등록은 되지만 specta 핸들러를 우회하는
> raw 채널). `REMOTE_ALLOWED_COMMANDS` 는 157종(d-42 이전 156종)으로 `IMPLEMENTED_JSON_COMMANDS` 와
> **집합이 다르다** — `pty_spawn`/`pty_attach` 는
> `dispatch()` 의 `match` arm 이 실재하는데도 `IMPLEMENTED_JSON_COMMANDS` 에는 없다(그 목록은
> specta/bindings 파리티만 추적하는 다른 축이라서다), 반대로 `file_read_raw` 는
> `IMPLEMENTED_JSON_COMMANDS` 에 없지만 `dispatch_raw()` 의 `match` arm 으로 원격 실행된다 — 그래서
> `REMOTE_ALLOWED_COMMANDS`/`REMOTE_DENIED_COMMANDS` 는 `dispatch()`/`dispatch_raw()` 양쪽이 **공유하는
> 하나의 정책 테이블**로 두고, 두 함수 모두 자기 `match` 를 돌리기 전에 먼저 이 테이블들로 게이트한다.
>
> `dispatch()`/`dispatch_raw()` 진입 순서(3단): **① `REMOTE_DENIED_COMMANDS` 조회 — 매치하면 즉시
> 거부(최우선)** → **② `REMOTE_ALLOWED_COMMANDS` 미등재 시 기본 거부**(`RemoteDenialPolicy::
> Unclassified`) → **③ `enforce_remote_owner_label` 로 `owner` 강제 치환 후 `match` 위임**. 두
> 함수의 `match` 자체의 `_` fallback 도 (①·②를 통과했다면 도달 불가능해야 하지만) 같은
> `Unclassified` 거부로 정합시켜 방어선을 이중화했다.
>
> **d-38(2026-08-25, `docs/acknowledge/2026-08-25-d38-remote-policy-contract.md`)**: 남아 있던 미결
> 정책 3건을 확정했다(사용자 결정 정본: `docs/acknowledge/2026-08-25-post-batch-user-decisions.md`
> §3). ① `ide_publish_diagnostics`/`ide_notify_at_mention` 원격 허용 — **검토 완료, 의도된 허용으로
> 확정**(현행 유지, 코드 변경 없음). ② `agent_hooks_install`/`agent_hooks_uninstall` 의 User 스코프
> 비대칭(install 은 거부, uninstall 은 허용) — **의도된 설계로 확정**(제거는 TAIDE 가 심은 훅을
> 지우는 방향이라 위험을 늘리지 않는다는 논리, 대칭화하지 않는다 — 아래 "스코프 조건부 거부" 항목
> 참조). ③ **키링 자격증명 변경을 원격 거부로 전환**(코드 변경, 이번 배치) — 키링 5계정
> (`infra::secret::SecretAccount`) 중 원격에서 바꾸거나 지울 수 있던 나머지 4계정을 막았다:
> `ai_set_token`/`ai_clear_token`(`AiOllamaCloud`/`AiCodex`/`AiOmlx`, `provider` 인자로 계정을
> 선택)과 `sync_connect`/`sync_disconnect`(`GithubSync` — gist 동기화 PAT)를
> `REMOTE_ALLOWED_COMMANDS` → `REMOTE_DENIED_COMMANDS` 로 옮기고, 신설
> `RemoteDenialPolicy::CredentialStoreTampering` 으로 분류했다(다섯 번째 계정 `RemoteAccess` 는 이미
> `SelfAccessExpansion` 으로 거부돼 있었다 — 감사 R3#7 이 지적한 "키링 5계정 중 원격 거부 1계정뿐"
> 갭이 이번 전환으로 해소돼 5계정 전부 원격 거부가 됐다). 값을 절대 반환하지 않는 상태 조회(`ai_token_status`/`sync_status`
> — 연결 여부만 반환)는 손대지 않고 허용에 남겼다. 이로써 `REMOTE_ALLOWED_COMMANDS` 160 → **156**,
> `REMOTE_DENIED_COMMANDS` 20 → **24**(총 180종은 그대로). 이것은 표면 축소이지 기밀 경계가 아니다 —
> `LocalFilesystemEscape`/`InstallOrProcessExecution` 과 같은 근거로, 인증된 원격 세션은 허용된
> `pty_spawn` 으로 이미 셸을 쥐고 있어 `settings.json` 을 직접 고쳐 동등한 지속성을 확보할 수 있다.
> 닫힌 것은 키링 항목의 write/delete 이며, 저장된 키의 전송 대상을 정하는 `ai_omlx_base_url` 은
> **d-41(2026-08-25, `docs/acknowledge/2026-08-25-d41-omlx-baseurl-strip-contract.md`)** 로 원격
> 게이트 스트립에 편입됐다 — `strip_remote_gated_settings_patch`/`strip_remote_gated_settings`
> (`settings_update`·`app_file_write`)뿐 아니라, 같은 축의 우회 경로였던 `sync_download`
> (`sync/service.rs`의 `strip_non_syncable` — dispatch 스트립 두 함수를 전혀 거치지 않고 곧장
> 호출된다)까지 3경로 모두에서 제거된다.
>
> **d-42(2026-08-25, `docs/acknowledge/2026-08-25-d42-e2e-defects-contract.md`)**: 신규 커맨드
> `search_list_files(projectId) → string[]`(§3 item d — 팔레트 파일 퀵오픈이 `tree_rows` 의 지연
> 로딩에 의존하던 결함 수정) 을 `IMPLEMENTED_JSON_COMMANDS`·`REMOTE_ALLOWED_COMMANDS` 양쪽에 등재
> (형제 `search_*` 3종과 동일 취급). `IMPLEMENTED_JSON_COMMANDS` 177 → **178**,
> `REMOTE_ALLOWED_COMMANDS` 156 → **157**(`REMOTE_DENIED_COMMANDS` 24 는 그대로, 총 180 → **181**).

- **명시 허용(`REMOTE_ALLOWED_COMMANDS`, 157종 — d-42 이전 156종)**: `match` arm 이 실제 핸들러로 위임한다. 예: `git_*`
  전종·`file_*`(아래 예외 제외)·`ai_*` 6종(`ai_set_token`/`ai_clear_token` 제외 — d-38, 아래 표
  참조)·`plugin_list`/`plugin_reload`/`plugin_read_grammar`·
  `remote_status`/`remote_revoke_sessions`·`lsp_confirm_reinitialize`/
  `lsp_report_reinitialize_failure`·`sync_*`(3종 — `sync_status`/`sync_upload`/`sync_download`,
  `sync_connect`/`sync_disconnect` 제외 — d-38)·`search_replace`
  (원격 세션도 파일을 직접 고쳐 쓸 수 있다 — 기존 설계상 허용, 별도 강화 없음)·`search_list_files`
  (d-42 신규 — 원격 세션은 이미 `tree_rows` 로 파일 목록을 볼 수 있어 민감도 축이 다르지 않다)·`theme_save`/
  `theme_delete`·`snippet_save`/`snippet_delete`·`git_init`·`pty_spawn`/`pty_attach`/`file_read_raw`
  (raw 채널 3종 — 아래 "raw 커맨드" 절 참조). `remote_start`/`remote_stop` 은 X-A 배치(2026-08-19)에서
  커맨드 자체가 제거돼 이 목록에서도 빠졌다(§"remote" 절 참조).
- **명시 거부(`REMOTE_DENIED_COMMANDS`, 24종)** — `dispatch()`/`dispatch_raw()` 가 실핸들러를 부르지
  않고 즉시 `AppError::Forbidden` 을 반환한다. `IMPLEMENTED_JSON_COMMANDS` 에는 파리티 유지를 위해
  그대로 남아 있다. 각 항목은 `RemoteDenialPolicy` enum 값 하나로 분류되고(같은 분류를 공유하는
  커맨드는 응답 메시지 문구도 공유한다 — 거부 여부·의미는 그대로, 문구만 변형별로 통합), 이전에는
  커맨드별 `deny_remote_*` 자유문 함수 15개가 이 역할을 했다:

  | 커맨드 | `RemoteDenialPolicy` | 거부 사유(요약) |
  |--------|----------------------|-----------------|
  | `remote_set_password` / `remote_clear_password` / `remote_issue_link` | `SelfAccessExpansion` | 원격 세션이 자기 접속 게이트를 바꾸거나(비밀번호) 새 온보딩 링크를 발급해 접근을 자가 확장하지 못하게(Wave B §6) |
  | `file_flush_complete` | `DesktopExitControl` | 데스크톱 자신의 `CloseRequested` 종료 시퀀스만 재개 가능(Hot Exit) |
  | `window_set_fullscreen` / `layout_move_tab_to_window` / `system_open_external_url` / `system_open_path` / `system_reveal_path` / `system_open_in_browser` / `system_open_app_data_path` / `notification_notify` / `notification_open_system_settings` | `UnreachableDesktopWindow` | 원격 세션에는 대응할 로컬 디스플레이/OS 창이 없음(Wave I·손 QA #12, 2026-08-18) — `tauri_plugin_opener` 로 데스크톱 자신의 화면에 앱 창(기본 앱 열기/Finder·Explorer 표시/OS 기본 브라우저)을 띄우는 넷과, 네이티브 OS 창을 직접 열거나 제어하는 셋이 같은 결론이다. 사용성 배치 4(2026-09-04)의 알림 둘도 같다 — 배너는 데스크톱 머신의 알림 센터에, 시스템 설정 창은 데스크톱 화면에 뜬다. `window_open_auxiliary` 는 이 분류의 일곱 번째 멤버였으나 X-A 배치(2026-08-19)에서 커맨드 자체가 제거됐다(§"X-A 배선 + 소규모 잔여 청소 배치" 절) |
  | `plugin_install` / `plugin_uninstall` / `vsix_import_plugin` / `vsix_extract_themes` | `LocalFilesystemEscape` | 데스크톱 로컬 파일시스템의 임의 경로를 이름으로 받음(Wave I) — 읽기(`vsix_extract_themes`)·쓰기(나머지 셋) 모두 프로젝트 루트 가드 밖 |
  | `agent_cli_install` / `agent_cli_uninstall` | `DesktopCliInterception` | `/usr/local/bin` 심링크·`osascript` 로 데스크톱 CLI 진입점을 설치/관리한다 — 원격 세션 종료 후에도 남는 CLI 실행 백도어가 되며(권한 프롬프트 불가시성은 부수 사유), `agent_hooks_install` User 스코프와 동일 분류(T0 감사 #12·#13, 2026-08-18) |
  | `agent_pending_external_opens` | `SharedSingletonStateRace` | `AgentStore` 의 대기 중 외부 열기 큐(`taide open --wait`)는 세션 구분 없는 단일 큐라 먼저 호출한 쪽이 통째로 비운다. 원격 세션이 드레인하면 `waitMarker` 등록이 원격 realm 의 `agent-wait-marker-registry.ts` 에 남아 데스크톱 탭 종료로는 해제되지 않고, 외부 CLI 프로세스가 앱 종료 전까지 블록된다(T0 감사 #14) |
  | `lsp_install` | `InstallOrProcessExecution` | `plugin_install`/`vsix_import_plugin` 과 동일 계열 — 수백MB 언어서버 아카이브를 데스크톱 로컬에 내려받고 인스톨러 프로세스를 spawn 한다(T0 감사 #16) |
  | `project_list_recent` | `LocalProjectHistoryExposure` | 현재 세션에 열려 있지 않은 프로젝트를 포함해 디스크의 영속 프로젝트 기록 전수를 반환한다 — Welcome 화면 "최근 프로젝트" 전용 로컬 조회이며, `project_list`(현재 열린 세션만 노출)보다 넓게 로컬 파일시스템 경로/이름을 드러내므로 원격 세션에는 불필요·부적절하다(d-27). **기대 동작**: `welcome` 탭은 모든 프로젝트의 기본 레이아웃에 포함되므로 원격 세션에서도 마운트되고, 이 커맨드는 매번 Forbidden 을 받는다 — "최근 프로젝트" 섹션은 원격에서 항상 빈 상태로 렌더되는 것이 의도된 열화다(`WelcomeContainer` 가 `isError` 시 `app.recentProjectsUnavailable` 안내 문구를 보여준다) |
  | `ai_set_token` / `ai_clear_token` / `sync_connect` / `sync_disconnect` | `CredentialStoreTampering` | 키링에 쓰거나 지우는 자격증명(AI 프로바이더 토큰 3종·GitHub 동기화 PAT)은 세션이 끝난 뒤에도 남는다 — 원격 세션이 자기 토큰으로 바꿔치면 이후 데스크톱이 내보내는 `ai_inline_complete`/`ai_inline_edit`/`ai_commit_message`·`sync_upload`/`sync_download` 트래픽이 공격자 계정으로 흐르고, 지우면 지속적인 서비스 거부가 된다 — `agent_hooks_install` User 스코프가 거부되는 것과 같은 "세션을 넘어 남는 백도어" 근거다(d-38, 감사 R3#7: 키링 5계정 중 `RemoteAccess` 만 원격 거부였다). 값을 절대 반환하지 않는 상태 조회 `ai_token_status`/`sync_status`(연결 여부만 반환)는 그대로 허용 유지. 닫힌 것은 키링 항목의 write/delete 이며, 저장된 키의 전송 대상을 정하는 `ai_omlx_base_url` 은 d-41(2026-08-25)로 원격 게이트 스트립(`strip_remote_gated_settings_patch`/`strip_remote_gated_settings`)과 gist 왕복 스트립(`sync/service.rs`의 `strip_non_syncable`) 양쪽에 편입돼 세 경로(`settings_update`·`app_file_write`·`sync_download`) 모두에서 변경 불가로 닫혔다(위 "d-38" 절 참조) |

- **스코프 조건부 거부(`REMOTE_ALLOWED_COMMANDS` 소속, `match` arm 내부에서 분기, 1종)**:
  `agent_hooks_install` 은 `agentName` 으로 `hook_scope_for_agent` 가 결정하는 스코프에 따라
  분기한다 — `HookInstallScope::Project`(`claude`, 프로젝트 루트 하위
  `.claude/settings.local.json` 에 `project_root` 로 루트 가드됨)는 원격에서도 그대로 허용,
  `User` 스코프(`codex`/`gemini`, 홈 디렉터리의 `~/.codex/hooks.json` / `~/.gemini/settings.json` 에
  루트 가드 밖 **command 훅**을 주입)는 `RemoteDenialPolicy::DesktopCliInterception` 으로 거부한다
  (`REMOTE_DENIED_COMMANDS` 테이블에는 없다 — args 조건부 판정이라 커맨드 이름만으로 결정되는 그
  테이블의 형태에 맞지 않는다). User 스코프 훅은 TAIDE CLI 가 모든 훅 이벤트마다 실행하는 셸
  커맨드를 심는 것과 같아, 원격 세션이 종료돼도 살아남는 백도어가 된다는 점에서 `settings_update`
  가 스트립하는 `shellOverride` 와 같은 근거다(T0 감사 #13). `agentName` 을 알 수 없으면(미지원
  이름) 이 분기가 아니라 실핸들러의 `InvalidArgument` 로 위임되어 동일한 에러를 낸다.
  짝 커맨드 `agent_hooks_uninstall` 은 **User 스코프 포함 원격 허용**이다(T1-K 검토에서 명시화 —
  제거 방향은 TAIDE 가 심은 훅 엔트리의 삭제라 백도어 설치 표면이 아니며, 전환 전 정책과 동일).
  install/uninstall 비대칭화는 **d-38(2026-08-25)에서 유지로 확정**됐다 — 대칭화(uninstall 도
  User 거부로 전환)하지 않는다(위 "d-38" 절 참조).
- **부분 스트립(핸들러는 호출하되 민감 필드를 지운 뒤 위임, 2종 — 둘 다 `REMOTE_ALLOWED_COMMANDS`
  소속)**:
  - `settings_update`: patch 에서 `remotePasswordOnlyLogin`·`remoteAllowedHosts`·`shellOverride`·
    `aiOmlxBaseUrl` 4필드를 `None` 으로 스트립한 뒤 위임(`remoteAccessEnabled` 는 자가 차단=자기
    접근 상실이라 스트립 대상에서 제외) — Wave B 하드닝에서 1필드(`remotePasswordOnlyLogin`)에
    나머지 2필드가 추가됐고, d-41 에서 `aiOmlxBaseUrl` 이 더해져 4필드가 됐다(근거는 위
    "d-41(2026-08-25, `docs/acknowledge/2026-08-25-d41-omlx-baseurl-strip-contract.md`)" 단락 참조).
  - `app_file_write`(`target.kind === 'settings'` 일 때만): 파싱된 전체 `Settings` 에서 같은 4필드를
    현재(적용 전) 값으로 되돌려쓴다 — patch 가 아니라 파일 전체 대치라 "지운다"가 아니라 "현재값
    유지"로 구현된다(`settings_update` 와 동급 게이트 — Wave I §3.3). `target.kind === 'prompt'`
    는 스트립 없이 그대로 위임한다.
- **raw 채널 3종**(`pty_spawn`·`pty_attach`·`file_read_raw`)은 specta 파리티 대상이 아니라
  `IMPLEMENTED_JSON_COMMANDS` 목록 자체에는 없지만, `REMOTE_ALLOWED_COMMANDS` 에는 셋 다 등재되어
  `dispatch()`/`dispatch_raw()` 의 별도 `match` arm 으로 **원격에서도 그대로 허용**된다 —
  `IMPLEMENTED_JSON_COMMANDS` 에 없는 것이 원격 차단을 뜻하지 않는다(아래 "raw 커맨드" 절 참조).
- **신규 커맨드 등재 규칙**: `dispatch()`/`dispatch_raw()` 에 새 `match` arm 을 추가하는 것만으로는
  원격 허용이 되지 않는다. `REMOTE_ALLOWED_COMMANDS`(허용) 또는 `REMOTE_DENIED_COMMANDS`(거부,
  `RemoteDenialPolicy` 분류 하나를 골라)에 반드시 이름을 등재해야 하며, 등재를 잊으면 완전 분할
  파리티 테스트가 실패한다 — 후속 정책 결정은 이 두 테이블 중 한쪽에서 다른 쪽으로 이름을 옮기는
  것으로 끝난다(T1-K 의 구조 전환 목적. 실례: d-38 의 키링 자격증명 4종 이동 — 위 "d-38" 절 참조).
  d-38 로 남은 미결 정책(ide 진단·훅 비대칭·키링 게이팅)이 모두 확정돼, 이 표의 각 행은 더 이상
  "재검토 예정"이 아니라 결정 완료 상태다.

### T0 감사 데이터·기능 수정 (2026-08-18)

> 계약: `docs/acknowledge/2026-08-18-audit-t0-fix-contract.md` §2.2~§2.4. 보안(§2.1) 5클러스터는
> 위 "원격 dispatch 정책" 절에 이미 반영돼 있다. 이 절은 **IPC 표면(타입·필드·커맨드 시그니처)에
> 영향을 준 항목만** 담는다 — 내부 구현만 바뀐 항목(watcher `.git` 필터, `search_replace`/`pty_write`
> 락 범위, `write_atomic_preserving_mode`, `delete_theme`/`load_theme` 경로 가드)은 커맨드
> 시그니처·반환 타입이 그대로라 여기 없다(상세는 `docs/bug/2026-08-18-audit-t0-fixes.md`).

- **git — `StatusRow`/`CommitFile` 절대경로 동봉(#18)**: 두 타입 모두 `absPath: string` (+ `origPath`
  가 있을 때만 나란히 채워지는 `origAbsPath: string | null`)이 새로 붙는다. `path`/`origPath` 는
  그대로 저장소-상대 경로(git2 반환값 그대로, git 커맨드에 되돌려 넘길 때 계속 쓴다)이고, `absPath`
  는 Rust 가 `repo.workdir()` 로 조인해 계산한 **파일 도메인(`file_open`/`file_save`) 이 요구하는
  절대경로**다. 소비부(`git-panel.tsx`) 는 "파일 열기"/"경로 복사"/"탐색기에 표시" 3곳에서
  `row.absPath` 로 전환됐다 — git 커맨드(`git_stage`/`git_unstage`/`onOpenChanges`(diff 탭) 등)에는
  계속 상대 `row.path` 를 넘긴다(그 경로들은 항상 이 `StatusRow`/`CommitFile` 이 속한 저장소 기준으로
  해석되므로 절대경로 변환이 필요 없다). `commit-detail-panel.tsx` 의 diff 탭 오픈(`TabKind::Diff`)도
  `git_show_file(rev, path)` 에 상대경로를 그대로 쓴다 — 파일 도메인을 거치지 않기 때문이다.
- **settings — `SettingsPatch` 클리어 가능 문자열 필드 일반화(#22, 사용자 결정 7)**: `shellOverride`·
  `editorFontFamily`·`terminalFontFamily`·`uiFontFamily`·`aiProvider`·`aiModel` 6필드가 `aiOmlxBaseUrl`
  이 먼저 쓰던 3상태 규약(빈 문자열=해제)을 그대로 따른다 — **필드 생략(`null`)=건드리지 않음,
  `""`=명시적으로 `None` 으로 되돌림(폰트 "System Default"·AI Provider/Model 미선택 등), 비어있지
  않은 문자열=그 값으로 설정**. `Option<Option<T>>` 은 배제하고 `Option<String>` 단일 레이어를
  유지한다(구 클라이언트가 이 필드들을 생략하면 여전히 "건드리지 않음"으로 해석되어 하위호환). 프론트
  피커(`settings-view.tsx` 의 폰트 패밀리 콤보박스·AI Provider 전환 시 `aiModel` 리셋)는 값이 없을 때
  `null` 이 아니라 `''` 를 patch 에 실어 보내도록 함께 갱신됐다 — 이전에는 `shellOverride:
  value.trim() || null` 처럼 빈 값을 `null` 로 보내던 자리가 있었는데, 새 규약에서 `null` 은
  "건드리지 않음"이라 그 자리들은 전부 해제가 조용히 무시되는 회귀였다.
- **tree/search — `SearchMatch` 좌표를 UTF-16 코드유닛 기준으로 정정(#23)**: `column`(1-based)·
  `matchStart`/`matchEnd`(0-based, `preview` 문자열 안 오프셋) 세 필드 모두 값의 **단위**가 Rust
  UTF-8 바이트에서 monaco `Position`/JS 문자열 slice 가 쓰는 UTF-16 코드유닛으로 바뀌었다(필드
  이름·타입은 `u32` 그대로). ASCII 텍스트는 두 단위가 같아 값이 그대로였지만, 한글 등 비 ASCII
  문자 뒤의 매치는 이전까지 실제보다 큰(바이트 기준) 좌표를 반환해 하이라이트/커서 이동이 밀렸다.
- **lsp — 백그라운드 크래시 재시작 후 상태를 `Crashed` 로 보고(#24)**: `handle_process_exit` 의
  자동 재시작(`RESTART_BACKOFF_LIMIT` 이내)이 프로세스 재기동에 성공해도 `lsp:session-status-changed`
  는 이제 `status: "running"` 이 아니라 `status: "crashed"`(+ 안내 메시지)를 낸다 — 재기동된 프로세스는
  `initialize` 핸드셰이크가 다시 이뤄진 적이 없어 실제로는 요청에 응답하지 못하는데, 이 배경 재시작은
  프론트의 어떤 액션도 기다리고 있지 않아(`lsp_spawn`/`lsp_restart` 와 달리) 재핸드셰이크를 걸어줄
  주체가 없다. `Running` 을 정직하지 못하게 보고하는 대신 `Crashed` 를 유지해 사용자가 수동
  재시작(`lsp_restart`, 이 경로는 프론트가 그 다음 `initialize` 를 스스로 보낸다)을 트리거하게
  한다. 근본 수정(세션 세대 이벤트로 프론트가 자동으로 재핸드셰이크)은 T1-D 로 이월.
- **terminal — 탭/프로젝트 종료 시 pty 세션 회수(#21, 사용자 결정 9)**: 새 커맨드는 없다 — 기존
  `project_close`/`layout_close_tab` 내부 동작이 바뀐다. `project_close` 는 그 프로젝트 소유의
  모든 pty 세션을 `TerminalStore::kill_project` 로 일괄 kill 한다(이전에는 프로젝트를 닫아도
  터미널이 앱 종료까지 계속 살아있었다). 터미널 탭을 닫으면(`layout_close_tab`, 실제로는 둘 다 거치는
  공유 경로 `close_tab_and_finish`) `TabKind::Terminal.sessionId` 를 `TerminalStore::kill_session`
  으로 개별 회수한다(**접합부 수정 — Phase D**: 계약 §3 은 이 절반을 "Rust: 탭 닫기 시 pty_kill"로
  적어뒀으나 R1/R2/F 어느 구현에서도 실제 호출부가 배선되지 않아 `pty_kill`/`killPty` 호출부가 0건인
  채로 남아 있었다 — `close_tab_and_finish` 에 직접 배선해 닫았다). 둘 다 `PtySession::drop` 이
  reader/flusher 스레드와 일시정지 게이트를 정리하므로 중복 kill(예: `pty_kill` 로 먼저 죽은 세션을
  `kill_project` 가 다시 순회) 은 무해하다(`kill` 자체가 멱등 — 이미 종료된 프로세스에 대한 kill 은
  에러를 반환할 뿐 패닉하지 않고, 호출부가 `let _ =` 로 무시한다).
- **project — `project_close` 부수효과 확장(#2·R4#7, IPC 시그니처 불변)**: 레이아웃이
  `dirty_layouts` 에 남아 있으면(2초 주기 flusher 가 아직 못 따라잡은 상태) 제거 전에 동기
  flush 한다(이전엔 이 경합에서 미저장 레이아웃이 경고 없이 버려졌다). `GitStore`(projectId→repo_root
  캐시)·`TreeStore`(트리 캐시)를 각각 `remove(project_id)` 로 회수한다(이전엔 프로젝트를 다시 열면
  옛 repo_root/디렉터리 목록이 부활했고, 앱 수명 내내 메모리에 남았다). 전체 회수 목록의 정본은
  `architecture.md` §6.3.
  **asset 프로토콜 스코프는 회수하지 않는다(X1#7, T0 #15 롤백 후 T1 2차로 재이월)** — Tauri
  `asset_protocol_scope()`(`scope::fs::Scope`)의 `forbid_directory` 는 되돌릴 수 없는 추가 전용
  API 라, 순진하게 호출하면 같은 폴더를 다시 열어도 영원히 `asset://` 를 못 읽는 새 회귀가
  생긴다. 근본 수정(자체 `register_uri_scheme_protocol("asset", ...)` 재구현 + 열린 프로젝트
  root 집합 기반 동적 판정)은 위험도·범위 문제로 이 배치에서 보류했다 — 사유는 `architecture.md`
  §6.3 참고. **현재도 닫힌 프로젝트 트리를 webview 가 계속 읽을 수 있다** (이 항목만 예외).

### T1 정비 3차 배치 — 멀티윈도우 owner 스코프·LSP 재핸드셰이크 (2026-08-19)

> 계약: `docs/acknowledge/2026-08-19-audit-t1-batch3-contract.md` §1.2(T1-D 레지스트리 정리).
> "채널 다중화" 절의 R7#6 문서 정정도 이 배치분이다(위 참조). 이 절은 **IPC 표면(커맨드
> 시그니처·이벤트 필드)에 영향을 준 항목만** 담는다 — 브리지 팩토리 2종 신설(F6#2·F6#3)·정리 경로
> 4건(reveal-registry TTL·open-with-registry LRU 상한·claude-diff-registry 미해결 탭 정리·
> terminal-write-bridge 큐 상한/TTL)·shiki 재초기화 직렬화(F6#19)·LSP 렌더러 세션 스코프화
> (codeLens/diagnostics/핸들러 해제 동일성 검사·useMonacoMarkers 구독 수명)는 내부 구현만 바뀌어
> 커맨드 시그니처·응답 타입이 그대로라 여기 없다.
>
> 공통 배경: 지금까지 `owner`(`getCurrentWindow().label` — `main`/`editor-<n>`, 원격 클라이언트는
> 고정 라벨 `"remote"`, Rust `domain::remote::types::REMOTE_OWNER_LABEL`)로 창을 스코프하는 것은
> `lsp_spawn`/`lsp_stop` 뿐이었다(Wave I 선례). 이 배치는 같은 패턴을 AI 요청·IDE 선택영역·검색
> 세션 3개 도메인에 확장해, 서로 다른 창이 같은 `requestId`/`sessionId` 를 우연히 재사용해도(특히
> `useId()` 처럼 리액트 렐름 로컬 카운터로 만든 id) 서로의 요청을 취소·덮어쓰지 못하게 한다.
>
> - **ai — `AiRequestStore` 키 `requestId` 단독 → `(owner, requestId)` 복합(R6#20)**:
>   `AiInlineCompleteRequest`/`AiInlineEditRequest`/`AiCommitMessageRequest` 3개 요청 타입 모두에
>   `owner: string` 필드가 새로 붙는다(프론트 `entities/ai/ai.ipc.ts`/`ai-inline-edit.ipc.ts` 가
>   호출자 대신 주입 — `completeAiInline`/`generateAiCommitMessage`/`requestAiInlineEdit` 의 프론트
>   시그니처 자체는 `owner` 없이 그대로다). `ai_request_cancel` 도 `(owner: string, requestId:
>   string)` 2-인자로 바뀐다(`cancelAiRequest(requestId)` 프론트 시그니처는 불변, 내부에서 `owner`
>   를 채운다).
> - **ide — `IdeStore.current_selection` 전역 단일 슬롯 → owner 인지(R6#12)**: `IdeSelectionInput`
>   에 `owner: string` 필드가 새로 붙는다(`ide_set_selection`). `ide_clear_selection` 도 무인자에서
>   `(owner: string)` 1-인자로 바뀐다. **동작 변경**: `owner` 가 원격 세션의 고정 라벨(`"remote"`)이면
>   `ide_set_selection`/`ide_clear_selection` 은 스토어에 아무것도 쓰지 않고 `selection_changed` 도
>   발행하지 않는 완전 no-op 이다 — 원격 브라우저의 선택영역이 데스크톱 로컬 IDE MCP 프로토콜
>   (`getCurrentSelection`/`getLatestSelection`)을 더 이상 덮어쓸 수 없다. 이 보증은 클라이언트가
>   보낸 `owner` 값의 정직성에 의존하지 않는다 — 원격 `dispatch()`/`dispatch_raw()` 진입점이
>   args 트리의 모든 `owner` 키(중첩 포함)를 `REMOTE_OWNER_LABEL`("remote") 로 강제 치환한다
>   (`enforce_remote_owner_label`, Phase E 보안 검토 후 신설). 도메인별 `owner` 비교는 이 강제
>   위의 defense-in-depth 다. 단 이 게이트의 범위는 selection 슬롯이며, `ide_publish_diagnostics`·
>   `ide_notify_at_mention`(owner 개념 없는 프로젝트 스코프 브로드캐스트)은 게이트 밖이다. 데스크톱 창끼리(main vs
>   보조 창)는 여전히 기존과 동일하게 전역 단일 슬롯을 공유한다(이 항목이 해소한 불변식은 "원격이
>   데스크톱을 오염시키지 못한다"에 한정 — 데스크톱 멀티윈도우 간 선택영역 자체 격리는 범위 밖).
>   프론트 `entities/ide/ide.ipc.ts` 의 `setIdeSelection`/`clearIdeSelection` 시그니처는 `owner`
>   없이 그대로다(내부에서 `getCurrentWindow().label` 을 채운다).
> - **search — `SearchStore` 키 `session_id` 단독 → `(owner, session_id)` 복합(R7#8)**: `search_run`
>   이 `(projectId, owner: string, sessionId, query, onMatch)` 5-인자로, `search_cancel` 이
>   `(owner: string, sessionId)` 2-인자로 바뀐다 — `lsp_stop` 의 owner 선례와 동일 근거. 검색 패널의
>   `sessionId` 는 `useId()`(리액트 렐름 로컬) 라 두 창이 우연히 같은 값을 만들 수 있었고, 이전에는
>   그 경우 한 창의 `searchCancel` 이 다른 창의 진행 중 검색을 잘라냈다. 프론트
>   `entities/search/search.ipc.ts` 의 `runSearch`/`cancelSearch` 시그니처는 `owner` 없이 그대로다.
> - **lsp — 자동 재시작 세션 세대(`generation`) 발행 + 재핸드셰이크 확인 커맨드 신설(R7#1, T0 #24
>   근본 대체)**: `LspSessionStatusChanged` 이벤트와 `lsp_sessions` 폴링 응답(`LspSessionInfo`) 모두에
>   `generation: number`(`u32`) 필드가 새로 붙는다 — `handle_process_exit` 의 백그라운드 자동
>   재시작이 프로세스 재기동에 성공할 때만 1 증가한다(`lsp_spawn`/`lsp_restart` 처럼 프론트가 직접
>   기다리는 재시작은 세대를 올리지 않는다). 신규 커맨드 **`lsp_confirm_reinitialize(sessionId:
>   string, generation: number) → Result<null, AppError>`** — 렌더러가 세대 증가를 감지해 기존 LSP
>   클라이언트 상태를 버리고 같은 `session_id` 로 `initialize` 를 재전송한 뒤, 응답을 받으면 그
>   트리거였던 `generation` 값 그대로 이 커맨드를 호출해야만 `status` 가 `Crashed` 에서 `Running` 으로
>   되돌아간다(넘긴 `generation` 이 세션의 *현재* 세대와 다르면 — 그 사이 2차 크래시가 세대를 더
>   올렸다면 — 조용히 무시된다, `domain::lsp::commands::confirm_reinitialize`). T0 #24 가 "재핸드셰이크
>   전까지 `Crashed` 유지"로 멈췄던 완화책을, 이 커맨드로 정직한 `Running` 복귀 경로까지 닫는다.
>   원격 dispatch 에서는 **허용**이다(원격 미러도 자기 세션의 재핸드셰이크를 확인해야 하며, 세대
>   불일치 무시가 오용을 방어 — Phase E 적대적 검증에서 위조 시나리오 반증 확정).
>   프론트 소비는 `entities/lsp/lsp-session-registry.ts` 의 `handleLspSessionStatusChanged`/
>   `reinitializeSession` — 모듈 로드 시 `events.lspSessionStatusChanged.listen` 으로 상시 구독한다.
>   `QUERY_KEY.LSP.SESSIONS` 폴링 캐시 쪽 무효화는 `entities/lsp/lsp.query.ts` 의
>   `useLspSessionsQueryInvalidationSync`(`app/providers/ipc-sync-provider.tsx` 에 상시 마운트)가
>   담당 — 이전에는 이 캐시에 무효화 호출부가 0건이었다.

### raw 커맨드 (specta 밖)

`RAW_CHANNEL_COMMANDS`(`src-tauri/src/lib.rs`)에 등록된 3종은 specta 를 통과하지 못해
`bindings.ts` 에 생성되지 않는다. `invoke()` 로 직접 호출한다. 셋 다 `dispatch()`(`pty_spawn`·
`pty_attach`) 또는 `dispatch_raw()`(`file_read_raw`)의 전용 `match` arm 으로 원격에서도 다뤄진다
(§"원격 dispatch 정책" 참조) — `IMPLEMENTED_JSON_COMMANDS` 배열에 없는 것은 파리티 테스트 대상이
아니기 때문일 뿐, 원격 미지원을 뜻하지 않는다. `REMOTE_ALLOWED_COMMANDS`(`dispatch.rs`)에는 셋 다
등재되어 있어 두 함수의 기본 거부 게이트를 통과한다 — `dispatch_raw()` 도 `dispatch()` 와 같은
`REMOTE_ALLOWED_COMMANDS`/`REMOTE_DENIED_COMMANDS` 테이블로 게이트되므로(T1-K), 네 번째 raw
커맨드가 추가되면 그것도 두 테이블 중 하나에 등재해야 원격에서 다뤄진다.

| 커맨드 | 이유 |
|--------|------|
| `pty_spawn` / `pty_attach` | raw 바이트 `Channel`(`InvokeResponseBody::Raw`) — `Vec<u8>` 로 보내면 JSON 숫자 배열이 되어 성능이 붕괴한다 |
| `file_read_raw(path)` | 미리보기용 원본 바이트를 `ArrayBuffer` 로 전달. 20MB(`READ_ONLY_FILE_BYTES`) 상한은 Rust 가 강제 |

## 4. 보안 (NFR-7)

- **view 는 fs/shell 계열 Tauri 플러그인 API 를 직접 호출하지 않는다** — 파일·프로세스 접근은 전부
  위 커스텀 command 경유(Rust 가 경로·인자 검증). capabilities 는 core 기본 + dialog 등 최소만 허용.
- 경로 파라미터는 Rust 에서 canonicalize + 프로젝트 루트/허용 범위 검증(터미널 링크 해석 등
  루트 밖 접근은 명시된 command 만).
- CSP: Monaco 요구(`style-src 'unsafe-inline'`, `worker-src 'self' blob:`)를 포함해 최소로 조인다
  (`editor.md` §1).
- Remote 도메인: 비밀번호·해시·세션/nonce 토큰 원문은 로그(`log::info!`/`log::warn!`)·IPC 응답·
  이벤트 payload 어디에도 실리지 않는다(`RemoteStatus` 는 `passwordConfigured: bool` 만 노출).
  비교는 전부 `constant_time_eq` 경유(토큰·비밀번호 동일 경로). 자세한 흐름은 §3 "Remote 비밀번호"
  절, 커맨드별 허용/거부 전체 그림은 §3 "원격 dispatch 정책" 절 참조.
- **zip 아카이브 하드닝(Wave I)**: `plugin_install`(아카이브 경로)·`vsix_import_plugin` 이 공유하는
  `infra::archive::extract_hardened_zip`(T1-I 에서 plugin↔vsix 순환 절단을 위해 infra 하강)은
  엔트리 수 상한(`ARCHIVE_MAX_ENTRIES` 5000)·누적
  압축 해제 바이트 예산(`ARCHIVE_MAX_TOTAL_BYTES` 128MB — zip bomb 방어)·경로는
  `enclosed_name()`(zip-slip 방어, `../` 를 포함한 엔트리는 스킵)·파일 모드는 항상 `0o644`/디렉토리는
  `0o755` 로 고정(zip 안에 담긴 unix 모드 비트를 신뢰하지 않는다)한다. 기존 `infra::lsp_install::
  extract_zip`(신뢰된 자체 배포 아카이브 전용, zip-slip 만 방어)과는 별개 함수로 분리했다 — 사용자가
  임의로 고른 `.vsix`/`.zip` 은 신뢰 전제가 다르기 때문(`plugins.md` §6).

### T1 정비 2차 배치 — remote 보안 헤더·asset 프로토콜 재구현 (2026-08-19)

> 계약: `docs/acknowledge/2026-08-18-audit-t1-batch2-contract.md` §1(T1-G R3#4·R3#5·X1#7). 감사
> 근거: `docs/quality-assurance/2026-08-18-architecture-audit.md`(C10). 전부 응답 헤더/서빙 로직
> 내부 변경이라 IPC 커맨드 시그니처·이벤트 페이로드는 **무변화**다.

- **remote 로그인 nonce 쿠키에 `Secure` 속성 추가(R3#4)**: 세션 쿠키(`build_session_cookie_header`)는
  `is_insecure_connection`으로 계산한 `secure`를 이미 조건부로 받았는데, 로그인 nonce 쿠키
  (`build_login_nonce_cookie_header`)만 이 파라미터가 없어 평문 loopback·터널 구분 없이 항상
  `Secure` 미부여였다. 이제 `auth_middleware`가 같은 `is_insecure_connection` 판정값을 두 쿠키
  모두에 전달한다 — loopback 평문 경로는 기존과 동일하게 `Secure` 를 생략한다(그 경로는 애초에
  HTTPS 가 아니라 `Secure` 를 붙이면 브라우저가 쿠키 자체를 거부한다).
- **`/__taide/file` 응답에 `Content-Security-Policy`·`X-Content-Type-Options: nosniff` 추가
  (R3#5)**: 이 라우트는 프로젝트 파일 바이트를 동일 오리진(same-origin)으로 그대로 서빙한다 —
  파일 중 SVG 가 `<script>` 를 품고 있으면 직접 네비게이션(또는 `<object>`/`<iframe>` 임베드)
  시 그 스크립트가 앱 자신의 오리진에서 실행될 수 있었다. `FILE_RANGE_CSP`(`default-src 'none';
  script-src 'none'; style-src 'none'; sandbox`)를 Range(206)·전체(200) 두 성공 응답 모두에
  부여한다 — `raw.githubusercontent.com` 이 사용자 콘텐츠에 적용하는 것과 같은 처방이다.
- **원격 파일 전체 서빙을 스트리밍으로 전환(R3#6, 위 R3#5 와 같은 파일에서 함께 처리)**: `Range`
  요청은 원래도 `RANGE_CHUNK_LIMIT`(1000KB)로 캡돼 있었지만, `Range` 헤더 없는 전체 파일 요청은
  `std::fs::read`로 파일 전체를 메모리에 올렸다 — 대형 프로젝트 파일(수 GB)을 `Range` 없이
  요청하면 그 크기 그대로 메모리에 적재됐다. `stream_file_body`가 64KB 청크
  (`FULL_FILE_STREAM_CHUNK_BYTES`) 단위로 `axum::body::Body::from_stream`을 통해 스트리밍한다 —
  응답 헤더(`Content-Length`·`Accept-Ranges` 등)는 그대로다.
- **asset:// 프로토콜을 자체 핸들러로 재구현(X1#7, T1-J 1차 이월분)**: `infra::asset_protocol::
  respond`가 `register_uri_scheme_protocol("asset", ...)`으로 Tauri 내장 asset 핸들러를 대체한다.
  경로 판정을 append-only 스코프에서 `root_guard::resolve_owning_project`(열린 프로젝트 집합
  기반, `/__taide/file`과 동일 함수) 로 바꿔 프로젝트를 닫으면 **다음 요청부터 즉시** 그 트리에
  대한 asset 읽기가 거부된다. `convertFileSrc`(`@tauri-apps/api/core`)·`tauri.conf.json`의
  asset CSP 소스는 스킴 이름만 보므로 **무변경**이다. 응답에는 `/__taide/file`과 동일한 CSP·
  `nosniff`를 부여한다. 상세 근거·KNOWN ISSUE(실기 미검증)는 `architecture.md` §6.3 "asset
  프로토콜 재구현 완료" 절, 실기 확인 항목은 `docs/quality-assurance/2026-08-11-qa6-checklist.md`
  "감사 T1 정비 2차 재검" 절.
- **`resolve_owning_project`의 결정적 선택(R2#4)**: 열린 프로젝트가 중첩되거나(워크스페이스 루트 +
  그 안에 별도로 연 하위 프로젝트) 동일한 루트를 가리키면, 이전에는 `HashMap` 순회 순서에 따라
  둘 중 어느 쪽이 그 경로의 소유자로 뽑힐지가 실행마다 달라질 수 있었다(원격 `/__taide/file`·
  로컬 `asset://` 둘 다 이 함수로 소유 프로젝트를 정한다). 이제 (1) 가장 긴 canonical root(가장
  구체적인 프로젝트) 우선, (2) 완전히 동일한 루트면 `ProjectId` 사전순으로 결정적으로 고른다 —
  IPC 응답 형태는 그대로고, "어느 프로젝트가 이겼는가"만 실행마다 안정된다.

### X-A 배선 + 소규모 잔여 청소 배치 — Phase R (Rust) (2026-08-19)

> 계약: `docs/acknowledge/2026-08-19-xa-wiring-cleanup-contract.md` §1.1/§1.2/§1.3(3)(4)(7), 판정
> 근거는 감사 `2026-08-18-architecture-audit.md` §7 X-A + T1 3차 계약 §5.1. Rust 측 배선·제거만
> 담당(Phase F 프론트 소비는 이 절 범위 밖). 이 절 앞의 각 도메인 절 본문도 이 배치에 맞춰 함께
> 갱신했다 — 여기서는 변경분만 모아 요약한다.

- **살리기 4건**: `fs:changed.fromApp` 마킹(§"file" 절), `terminal:cwd-changed`/
  `resolve_terminal_path` 배선(§"terminal" 절), `layout:changed.revision` 발행 규약 문서화
  (§"layout" 절) — 셋 다 위 해당 절에 상세 기술. `layout_set_view_state` 는 실사 결과 이미
  정확히 구현돼 있어 Rust 무변경(저장 필드 `Tab::view_state`·서비스 `set_view_state`·
  `finish_mutation` 경유 `revision` 증가·`fs:changed`/재시작 영속까지 전부 기존재).
- **지우기 2건**: `project:focus-kind-changed` 이벤트·`FocusKind` 타입·`layout::service::focus_kind`
  전량 제거(§"project" 절), 중복 커맨드 5종(`ide_start`/`ide_stop`/`remote_start`/`remote_stop`/
  `window_open_auxiliary`) 제거 — `settings_update` 의 통합 토글 부수효과(현 `SettingsToggleObservers`)
  가 이미 유일한 실제 도달 경로였음을 실코드로 확증한 뒤 제거했다(§"ide"·"remote"·"Wave I 계약
  확정 추가" 절). `app:ready` 도 함께 제거(발행 지점 0 확인, §"app" 절).
- **§1.3(3) ws.rs writer 무한 대기 수정**: `domain::remote::ws::handle_socket` 이 연결 종료 후
  writer 태스크를 무조건 `.await` 하던 것을, `REMOTE_WS_WRITER_SHUTDOWN_TIMEOUT_MS`(3초) 로 유한
  대기 후 `abort()` 하도록 바꿨다 — 도메인 스토어(LSP/검색/AI/pty 세션)가 이 연결의 채널 싱크를
  계속 쥐고 있으면(무트래픽 세션이라 아무도 그 싱크로 다시 `send` 하지 않아 프루닝될 기회가 없는
  경우) writer 의 `rx.recv()` 가 영원히 끝나지 않아 `RemoteStore::client_disconnected()` 가 아예
  호출되지 않는 버그였다(클라이언트 카운트·세션 프루닝 지연). 세션 만료 시의 `Close` 프레임
  (`REMOTE_WS_CLOSE_CODE_SESSION_EXPIRED`) 은 정상 케이스에서 타임아웃 내에 그대로 플러시된다 —
  IPC 표면 변화 없음(내부 구현만 변경).
- **§1.3(4) LSP 재핸드셰이크 실패-확인 신설**: 신규 커맨드 `lsp_report_reinitialize_failure(sessionId,
  generation) → Result<null, AppError>` — `lsp_confirm_reinitialize`(성공 확인)의 실패 대응
  커맨드로, 렌더러가 재시도를 모두 소진했을 때 호출한다. 같은 세대 가드(`confirm_reinitialize`)를
  공유하고, **추가로 세션의 현재 `status` 가 이미 `Crashed` 일 때만** 통과하면 `status` 를 `Crashed`
  로 확정하며 `last_error` 를 `handle_process_exit` 의 낙관적 "재시작됐습니다, 기다려주세요" 문구
  대신 "재연결 실패, 수동으로 다시 시작해주세요" 로 갱신한다(Phase E 검토 반영, 2026-08-19 — 최초
  구현은 세대 가드만 있었다). `REMOTE_ALLOWED_COMMANDS`/T1-K 테이블에 등재 완료(원격에서도 허용 —
  원격 미러도 자기 세션의 실패를 확정할 수 있어야 한다). **정정**: 세대 불일치 무시 가드는
  **경합 방어(race guard)** 이지 위조 방어가 아니다 — `lsp_sessions` 가 현재 `generation` 을 그대로
  응답에 실어 돌려주므로, 이 커맨드를 부를 수 있는 인증된 원격 피어는 언제나 일치하는 generation
  을 얻을 수 있다(그 피어는 이미 `lsp_stop`/`lsp_restart`/`file_delete`/`git_discard` 를 갖고 있어
  generation 매칭이 별도 권한 상승도 아니다 — T1-K 의 "원격=인증 동급 신뢰, 호출자 identity 는
  인가 축이 아니다" 선례와 일관). `status == Crashed` 전제 없이 세대 가드만 있던 최초 구현은
  비적대적 실결함이 있었다: `lsp_restart`(수동 재시작)는 `Running` 으로 전환하되 `generation` 을
  올리지 않으므로, 크래시 시점에 시작된 옛 재시도 루프가 사용자의 수동 재시작 이후 뒤늦게 예산을
  소진해 이 커맨드를 부르면 세대는 여전히 일치해 방금 복구된 정상 세션이 다시 `Crashed` 로
  잘못 표시됐다 — 이번 `status == Crashed` 전제가 이 경로를 막는다. `lsp_confirm_reinitialize` 는
  대칭으로 이 전제를 추가하지 않는다 — 그 커맨드의 목표 상태(`Running`)는 이미 건강한 세션에
  적용해도 무해하므로 상태 전제가 필요 없고, 비대칭은 "누가 부를 수 있는가"가 아니라 "각 커맨드의
  목표 상태가 무엇을 안전하게 덮어쓸 수 있는가"에서만 나온다. 상세는 위 "T1 정비 3차 배치" 절의
  `lsp_confirm_reinitialize` 기술과 나란히 읽는다.
- **§1.3(7) `tree_rows` limit 센티널 제거**: 위 "tree / search" 절 참조 — `u32` → `Option<u32>`.
- **커맨드/이벤트 수 갱신**: command 180→**176**(중복 5종 제거 + 신규 1종), raw 포함
  183→**179**, event 25→**23**(2종 제거), `REMOTE_ALLOWED_COMMANDS` 163→**160**,
  `REMOTE_DENIED_COMMANDS` 20→**19**. 문서 최상단 실측 배너 갱신 완료.
- **판정만 하고 배선하지 않은 2건(§1.2 X1#2 잔여)**: `remote:state-changed` 는 발행은 되지만(원격
  서버 시작/중단 시) 프론트 소비가 0 이었다 — `sync:state-changed` 와 동일한 패턴(`ipc-sync-
  provider.tsx` 에서 `REMOTE.STATUS` 쿼리를 `setQueryData`)으로 배선하는 것이 자연스럽다고 판정해
  이벤트는 유지, 실제 프론트 배선은 범위 밖으로 이월(F1 후속, §"remote" 절에 기록). `terminal:exited`
  는 페이로드(`sessionId`, `code`)가 소비에 충분함을 확인만 하고, 실제 소비(세션 정리 트리거 —
  살아있는 것으로 취급되던 죽은 pty 엔트리 정리)는 F2 배선(`TerminalStore` 에서 해당 `session_id`
  제거 — 기존 `pty_kill` 이 이미 idempotent 하므로 새 커맨드 불필요, §"terminal" 절 참조).

### d-50 S1a — 검색 파이프라인 (2026-08-29)

> 계약: `docs/acknowledge/2026-08-29-d50-audit-rust-batch-contract.md` §1 S1a.
> 발견 정본: `docs/quality-assurance/2026-08-29-full-audit.md` §2 H-1·H-2·M-4·M-9 · §4-A-8 · §4-B C10.
> 상세 규약은 위 "tree / search" 절에 반영했다. 이 절은 **표면 변경분만** 요약한다.

- **`SearchMatch` → `SearchLineMatch` + `SearchFileMatches`**: `search_run` 의 `onMatch` 채널
  페이로드가 매치 1건에서 파일 1건(`{ path, matches: SearchLineMatch[] }`)으로 바뀌었다.
  `SearchLineMatch` 는 옛 `SearchMatch` 에서 `path` 를 제거한 형태이며 `line`·`column`·`preview`·
  `matchStart`·`matchEnd`·`before`·`after` 필드와 그 단위 규약(UTF-16 코드유닛 — T0 #23)은 그대로다.
  커맨드 이름·인자·반환 타입(`number`)은 불변이므로 커맨드 수·dispatch 테이블·원격 정책 분류는
  변동 없다(`search_run` 은 계속 `REMOTE_ALLOWED_COMMANDS`).
- **`SearchReplaceResult` 필드 2개 순증**: `skipped: ReplaceSkippedFile[]`(경로+사유) ·
  `skippedCount: number`. 신규 타입 `ReplaceSkippedFile` · `ReplaceSkipReason`(union:
  `tooLarge`·`binary`·`notUtf8`·`unreadable`·`writeFailed`). 기존 두 필드는 그대로라 구 소비부는
  타입상 그대로 컴파일된다(표시는 d-51 F2 담당).
- **커맨드·이벤트 수 불변**: 신규/삭제 커맨드 0, 신규/삭제 이벤트 0. 변경은 타입 표면뿐이며
  `bindings.ts` 를 재생성(`cargo test --lib typescript_바인딩을_생성한다`)해 반영했다.
- **프론트 접점**(d-51 F2 를 위한 명시): `entities/search/search-result.ts` 의 `SearchResultGroup`
  (`{ path, matches }`)과 `SearchMatchRowData` 는 **이름·형태 모두 그대로**다(`SearchMatchRowData`
  가 `Omit<SearchMatch,'path'>` 대신 `SearchLineMatch` 별칭이 됐을 뿐). `useSearchRun` 의 반환
  (`{ results, totalMatches, isSearching, run }`)도 이 스테이지 시점에는 불변이라
  `search-panel*`·`search-editor-pane` 은 수정되지 않았다. **(이후 d-51 F2 가 같은 반환을
  `{ results, totalMatches, status, ranQuery, isTruncated, run, readSnapshot }` 로 확장하고 그
  위젯들을 함께 고쳤다 — 현행 형태는 `docs/features/explorer-sidebar.md` §3.1 과
  `src/entities/search/use-search-run.ts` 를 본다.)** 바뀐 것은 ① `groupSearchMatches(matches)` 가
  `appendSearchFileMatches(accumulator, batches)` + `createSearchResultAccumulator()` 로 대체됨
  ② `runSearch` 의 콜백 이름이 `onMatch` → `onFileMatches`(인자도 배치) 로 바뀜 — 두 곳 모두
  `entities/search` 내부 소비뿐이다.

### d-50 S3 — git 도메인 (2026-08-29)

> 계약: `docs/acknowledge/2026-08-29-d50-audit-rust-batch-contract.md` §1 S3.
> 발견 정본: `docs/quality-assurance/2026-08-29-full-audit.md` §2 M-1·M-7 · §4-A-6 · §4-A-10 · §4-B B11.

- **`git_diff_file` 에 `beforePath?: string | null` 인자 순증** — 위 "git" 절의 query 항목 참조.
  커맨드 이름·개수·반환 타입은 불변이라 dispatch 테이블은 인자 1줄(`arg!(args, "beforePath")`)만
  늘었고, 원격 정책 분류(`REMOTE_ALLOWED_COMMANDS`)와 커맨드 수 41종은 그대로다. **프론트 배선은
  d-51 F3** — 이번 스테이지는 `entities/git/git.ipc.ts` 의 `getGitDiffFile` 이 선택 인자
  `beforePath` 를 받아 그대로 넘기는 데까지만 배선했고(미지정 시 `null` = 기존 동작),
  `gitDiffFileQueryOptions`/`QUERY_KEY.GIT.DIFF` 는 손대지 않았다. F3 는 **쿼리 키에도
  `beforePath` 를 포함**해야 한다 — 같은 새 경로에 대해 원경로 유무로 결과가 달라지므로 키가
  같으면 캐시가 섞인다.
- **`git_show_file` 블롭 크기 상한** — `constants::REFUSED_FILE_BYTES`(50MB)를 넘는 블롭은
  `error.git.blobTooLarge`(`InvalidArgument`, arg: `path`)로 거절한다. 크기는 `Odb::read_header` 가
  읽는 오브젝트 헤더에서 오므로 초과 블롭은 **압축 해제조차 되지 않는다**. `file_open` 이 열기를
  거절하는 것과 같은 경계라 "에디터가 열 수 있는 파일 = 히스토리에서 꺼내올 수 있는 파일"이
  유지된다(커밋 diff·파일 히스토리 탭이 파일 도메인의 상한을 우회하는 뒷문이 되지 않도록).
- **`run_git` 타임아웃** — `git` 서브프로세스가 `GIT_COMMAND_TIMEOUT_SECS`(300초)를 넘기면 kill 하고
  `error.git.commandTimedOut`(`Internal`, args: `command`·`seconds`)을 돌려준다. `push`/`pull`/
  `fetch` 를 포함해 `run_git` 을 쓰는 전 경로 공통이다. `git_pull` 의 전체 mutation guard 유지와
  `git_push`/`git_fetch` 의 `push_fetch_lock` 은 **기존 보류 결정 그대로 불변**이며, 이 타임아웃은
  그 락들의 보유 시간이 앱 수명만큼 무한해지던 경로만 끊는다.
- **조회 7종 `spawn_blocking` 이관** — `git_show_file`·`git_ahead_behind`·`git_remotes`·`git_gutter`·
  `git_current_user`·`git_branches`·`git_stash_list`. 기존 `git_status` 와 같은 모양이고 시그니처·
  반환값·락 의미는 전부 불변이라 **IPC 표면 변화가 아니다**(architecture.md §2.1 준수 복구).
- **로케일 키 2종 순증**: `error.git.blobTooLarge` · `error.git.commandTimedOut` — ko/en/ja 3언어
  + `MESSAGE_NAMESPACES` 등재 완료(파리티 테스트 통과).

### d-50 S4 — terminal 도메인 (2026-08-29)

> 계약: `docs/acknowledge/2026-08-29-d50-audit-rust-batch-contract.md` §1 S4.
> 발견 정본: `docs/quality-assurance/2026-08-29-full-audit.md` §2 M-5·M-6·L-2 · §4-A-5.

- **커맨드·타입·이벤트 표면 전부 불변**: 신규/삭제 커맨드 0, 인자·반환 타입 변경 0, 신규 타입 0.
  dispatch 테이블·원격 정책 분류·커맨드 수 변동 없음. `bindings.ts` 재생성분은 `pty_write` 의
  doc 주석 1건뿐이다.
- **`pty_attach` 리플레이의 원자성(신규 계약 — §4-A-5)**: 스크롤백 리플레이와 구독자 등록이 이제
  **하나의 락 아래 한 임계구역**이다. 따라서 구독자가 받는 바이트열은 항상 `리플레이(= attach
  시점까지 누적분) ++ 이후 브로드캐스트 전량`이고, 그 경계에서 **중복도 유실도 없다**. 이전에는
  링버퍼 락과 구독자 락이 분리돼 스냅샷~등록 사이에 도착한 청크가 누구에게도 전달되지 않거나
  (다른 인터리빙에서는) 두 번 전달됐다.
- **리플레이는 최대 2개의 raw 청크로 온다(신규 계약)**: 스크롤백이 `VecDeque` 기반 원형 버퍼로
  바뀌면서(§2 M-5) 리플레이는 링의 두 조각을 앞→뒤 순서로 보낸다. 버퍼가 되감기기 전(스크롤백이
  2MB 상한에 닿기 전)에는 두 번째 조각이 비어 있어 기존처럼 1청크다. **순서는 보장**되며, 소비부
  (xterm)는 이미 reader 배칭 때문에 write 경계를 넘는 파서 상태를 보존하므로(`terminal.md` §12.3)
  경계에서 이스케이프 시퀀스가 잘려도 해석이 깨지지 않는다.
- **`pty_write` 가 `spawn_blocking` 으로 이관**(§2 M-6) — 자식 stdin 이 막혔을 때 async 워커
  스레드를 붙들지 않는다. 커맨드 시그니처는 그대로이고 `TerminalStore` 락 보유 범위(핸들 조회까지)
  도 그대로다.
- **쓰기 순서 규약(검토 반영 — 2026-08-29)**: `pty_write` 는 **호출 순서를 스스로 보장하지 않는다.**
  첫 poll 이 writer 핸들만 복제하고 블로킹 풀로 디스패치하므로, 같은 틱에 발사된 두 호출은 서로
  독립적인 블로킹 태스크가 되어 writer 뮤텍스를 경쟁한다(순서는 스케줄러가 정한다). 실제로 그 형태가
  존재한다 — `shared/lib/bridge/terminal-write-bridge.ts` 가 스폰 전 대기 큐를 한 틱에 연속 발사한다.
  따라서 **"같은 세션의 쓰기는 호출 순서대로 자식 stdin 에 도달한다"는 프론트가 보장한다**:
  `entities/terminal/session-write-order.ts` 의 세션별 프라미스 체인을 `writePty` 가 통과한다.
  다른 클라이언트(원격 dispatch 포함)가 `pty_write` 를 직접 호출한다면 같은 직렬화를 스스로 해야 한다.
- **flusher 스레드가 condvar 대기로 전환**(§2 L-2) — 유휴 세션의 5ms 상시 wakeup(세션당 초당 200회)
  제거. 관측 가능한 출력 지연 규약(`OUTPUT_BATCH_MS` 4ms 배치 + 마지막 조각 `OUTPUT_FLUSH_TICK_MS`
  5ms 후 flush)은 불변이다.

### d-50 S5 — lsp 도메인 (2026-08-29)

> 계약: `docs/acknowledge/2026-08-29-d50-audit-rust-batch-contract.md` §1 S5.
> 발견 정본: `docs/quality-assurance/2026-08-29-full-audit.md` §2 L-3 · §4-A-7·§4-A-11.

- **커맨드·타입·이벤트 표면 전부 불변**: 신규/삭제 커맨드 0, 인자·반환 타입 변경 0, 신규 타입 0.
  `bindings.ts` 재생성 불요(specta 가 보는 시그니처 무변동), dispatch 테이블·원격 정책 분류·커맨드
  수·로케일 키 변동 없음.
- **워크스페이스 폴더 URI 표기(신규 계약 — §4-A-7)**: Rust 가
  `workspace/didChangeWorkspaceFolders` 로 보내는 폴더 `uri` 는 프론트가 `initialize` 의
  `rootUri`/`workspaceFolders` 에 쓰는 `monaco.Uri.file(root).toString()` 과 **바이트 단위로 동일**
  하다(`domain::lsp::service::workspace_folder_uri`). 인코딩 규칙은 RFC 3986 퍼센트 인코딩 —
  `unreserved`(`ALPHA`·`DIGIT`·`-`·`.`·`_`·`~`)와 경로 구분자 `/` 만 원문 유지, 나머지 바이트는
  UTF-8 대문자 16진 `%XX`. monaco 가 하위 구분자(`!$&'()*+,;=`)와 `:@[]` 까지 escape 하므로
  `pchar` 를 그대로 두는 표준 최소 인코딩보다 좁게 잡는다(양쪽 표기 일치가 목적).
  `initialize` 를 만드는 쪽은 `src/shared/lib/lsp/initialize-params.ts` 로 **변경 없음** — 이 계약을
  깨지 않으려면 두 곳 중 한쪽만 바꾸지 말 것.
- **비프로토콜 잡음의 처리(신규 계약 — §4-A-11)**: 서버 stdout 에 `Content-Length` 없는 헤더 블록이
  섞여 오면 그 블록만 버리고 다음 프레임을 계속 찾는다. `lsp_spawn` 의 `onMessage` 스트림에서
  관측되는 변화는 "잡음이 흘러들지 않는다"가 아니라 **잡음 뒤의 메시지가 계속 도착한다**는 것이다
  (이전에는 그 시점부터 세션이 영구 무응답이 됐다). 바디가 UTF-8 이 아닌 프레임도 같은 규칙으로
  건너뛴다.
- **자식 프로세스 대기가 `try_wait` 폴링에서 `wait`+`Notify` 로 전환**(§2 L-3) — 관측 가능한
  수명주기 계약(`lsp:session-status-changed`, `LspProcHandle::is_exited` 기반 종료 대기)은 불변이고,
  유휴 서버당 50ms 폴링만 사라진다. 수신 버퍼도 메시지마다 `drain` 하지 않고 소비 커서로 상환한다.

### d-50 S6 — infra 소형 (2026-08-29)

> 계약: `docs/acknowledge/2026-08-29-d50-audit-rust-batch-contract.md` §1 S6.
> 발견 정본: `docs/quality-assurance/2026-08-29-full-audit.md` §2 M-3 · §4-A-4·§4-A-9.

- **커맨드·타입·이벤트 표면 전부 불변**: 신규/삭제 커맨드 0, 인자·반환 타입 변경 0, 신규 타입 0.
  `bindings.ts` 재생성 불요, dispatch 테이블·원격 정책 분류·커맨드 수·로케일 키 변동 없음.
- **`fs:changed` 의 무시 규칙(계약 정정 — §4-A-4)**: 무시 목록(`IGNORED_DIR_NAMES`)은 경로의
  **조상 디렉토리 성분에만** 적용된다. 마지막 성분(이벤트의 대상 항목 자신)은 판정에서 제외되므로,
  루트의 `build`·`dist` 같은 **이름을 가진 파일**의 변경도 이제 이벤트로 도착한다(트리는 무시 이름을
  디렉토리일 때만 감추므로 그 파일은 원래 화면에 보였고, 변경만 영원히 반영되지 않았다). 부수 효과로
  무시 디렉토리 **자신**의 생성/삭제 이벤트 1건은 통과한다 — 그 하위 경로는 종전대로 전부 걸러진다.
- **git 워처는 무시 목록을 적용하지 않는다(신규 계약 — §4-A-4)**: `.git` 을 루트로 도는 워처
  (`domain::git::watch`)는 `WatchScope::GitDir` 로 시작해 이름 필터를 아예 쓰지 않는다.
  `refs/heads/**` 는 **브랜치 이름**이라 `build/login`·`dist` 같은 평범한 브랜치의 ref 이벤트가
  통째로 드롭됐고, 그만큼 `git:refs-changed` 가 발행되지 않았다. 잡음 차단은 종전대로 하류의
  `classify_git_change`(`index`·`HEAD`·`refs/**` 만 통과, `objects/**` 제외)가 담당한다.
- **HTTP `Range` 접미 범위(계약 정정 — §4-A-9)**: `/__taide/file`(원격)과 `asset://`(로컬) 두 서빙
  경로 모두 `bytes=-N` 을 RFC 7233 §2.1 대로 **파일의 마지막 N 바이트**로 해석한다(이전에는 앞에서
  N+1 바이트를 돌려줬다 — mp4 의 `moov` atom 이 끝에 있는 파일에서 탐색이 실패하는 형태).
  `N` 이 파일 크기보다 크면 전체, `N == 0` 은 만족 불가(`416`). `RANGE_CHUNK_LIMIT`(1000KB) 상한은
  접미 범위에도 그대로 걸리므로 응답이 요청보다 짧을 수 있고, 그 사실은 `Content-Range` 가 말한다.
- **에이전트 폴링(§2 M-3)** — `agent_list`/`agent:state-changed` 의 결과 계약은 불변이고, unix 프로브가
  pid 당 `ps` 를 fork 하던 것을 **폴링 틱당 1회**(`ps -p pid1,pid2,...`)로 합쳤다. pty 세션이 0 인
  프로젝트는 블로킹 풀 디스패치 없이 빈 결과로 조기 반환한다.

### d-50 S8 — layout 탭 경로 추종 (2026-08-29)

> 계약: `docs/acknowledge/2026-08-29-d50-audit-rust-batch-contract.md` §1 S8.
> 발견 정본: `docs/quality-assurance/2026-08-29-full-audit.md` §4-B A3 · §1-6.

- **신규 커맨드 1종** `layout_apply_path_change(projectId, change: TabPathChange) →
  TabPathChangeResult`. 커맨드 총수 178 → **179**, `REMOTE_ALLOWED_COMMANDS` 157 → **158**
  (`REMOTE_DENIED_COMMANDS` 24 불변) — 형제 레이아웃 커맨드와 동일하게 허용이다. 원격 세션이 이미
  `file_rename`/`file_delete` 와 `layout_close_tab` 을 쓸 수 있고, 이 커맨드는 그 둘의 조합보다 넓은
  권한을 주지 않는다(자기 프로젝트 레이아웃의 파일 탭 경로만 바꾼다).
- **신규 타입 3종**: `TabPathChange`(태그 union — `{kind:'renamed', from, to}` /
  `{kind:'deleted', path}`), `TabPathMove{from, to, dirty}`, `TabPathChangeResult{layout, moved,
  closedPaths}`. 기존 타입 변경 0, 이벤트 변경 0.
- **호출 규약**: 프론트가 `file_rename`/`file_delete` **성공 이후에** 부른다(`entities/file/
  file.query.ts` 의 `useRenameEntry`/`useDeleteEntry` 의 `onSuccess`). 파일 커맨드 안에서 부르지
  않는 이유는 도메인 결합 — `domain::file` 이 `domain::layout` 을 알게 되기 때문이며, 트리 캐시
  정리를 워처 이벤트에 맡긴 것(S7)과 같은 경계다.
- **경로 매칭**: `from`/`path` 는 프론트가 파일 커맨드에 넘긴 문자열 그대로이며, 백엔드는 다시
  canonicalize 하지 않는다(개명 후 `from` 은 디스크에 없다). 비교는 **경로 성분 단위**
  (`Path::strip_prefix`)라 `src` 개명이 형제 `src-old` 를 끌고 가지 않는다. 디렉토리를 지목하면
  하위 전체가 대상이다.
- **루트 가드(검토 반영 — 2026-08-29)**: `from`/`to`/`path` 는 **그 프로젝트 루트 하위**여야 한다
  (성분 단위 `starts_with`, canonicalize 없음 — 프로젝트 루트는 열 때 이미 canonical 이고 탭 경로는
  그 루트를 걸어 만든 것이라 접두사가 같다). 위반은 `Forbidden`
  (`error.path.outsideProjectRoot`). 없으면 `deleted{path:"/"}` 가 모든 절대 경로의 조상이 되어
  프로젝트의 파일 탭 전부를 닫고, 프론트의 `releaseClosedFileTabPath` 가 그 경로들의 핫엑시트
  미러(= 미저장 초안)를 전부 지운다 — 원격 허용 커맨드이므로 실호출 경로가 있다.
- **대상 탭 종류**: `TabKind::File` 만. `Diff`/`ClaudeDiff` 는 비교 뷰라 제목·짝(rev/beforePath/
  compareWith)이 개명으로 이전되지 않으므로 손대지 않는다(잔여는 계약 §5 기록).
- **효과**: `renamed` 는 열린 파일 탭의 `path` 와 `title`(새 파일명)을 치환하고 `closedTabs` 스택의
  같은 탭도 함께 옮긴다(재열기가 없는 경로를 열지 않도록). `deleted` 는 해당 파일 탭들을
  `close_tab` 으로 닫는다 — 닫힌 탭 스택·활성 탭 인계·페인 정규화는 손으로 닫은 것과 동일하다.
- **revision·이벤트**: 실제로 바뀐 **열린** 탭이 있을 때만 `revision` 이 증가하고 `layout:changed`
  가 발행된다. 보고할 것이 없으면 응답은 빈 `moved`/`closedPaths` 다.
  **단(검토 반영 — 2026-08-29), 보고할 것이 없어도 닫은 탭 스택만 새 경로로 바뀐 경우에는 그 레이아웃을
  스토어에 기록하고 `dirty_layouts` 로 표시한다**(revision·이벤트는 여전히 없다 — 화면에 보이는 것이
  바뀌지 않았으므로). 이전 형태는 "보고할 것 없음"과 "레이아웃 무변경"을 같은 조건으로 묶어 조기
  반환해서, 열린 탭이 하나도 없을 때의 닫은 탭 스택 치환을 통째로 버렸다(⇧⌘T 가 사라진 경로를 열었다).
- **`moved[].dirty`**: 그 경로의 탭 중 하나라도 미저장이면 참. 프론트가 핫엑시트 미러를 새 경로로
  옮길지 판단하는 유일한 입력이다(`EditorPane` 은 경로가 바뀌면 자신의 dirty 를 초기화하므로,
  미러로 넘기지 않으면 새 경로의 디스크 내용이 편집 중 버퍼를 덮는다).

### d-53 U1 — 에디터 표시 옵션 5건 (2026-08-29)

> 계약: `docs/acknowledge/2026-08-29-d53-ux-batch-contract.md` §1 U1.
> 발견 정본: `docs/quality-assurance/2026-08-29-full-audit.md` §5 저비용 즉효 ①~⑤.

- **커맨드·이벤트 표면 불변**: 신규/삭제 커맨드 0, dispatch 테이블·원격 정책 분류·커맨드 수 변동
  없음. `Settings`/`SettingsPatch` 에 **필드 7종 순증**이라 `bindings.ts` 는 재생성했다.
- **신규 settings 필드(전부 VS Code 파리티 기본 off/빈 값)**:
  `editorBracketPairGuides: boolean`(기본 false — monaco `guides.bracketPairs`,
  `editorBracketPairColorization` 과 별개 옵션) · `editorSmoothScrolling: boolean`(기본 false) ·
  `editorCursorSmoothCaretAnimation: boolean`(기본 false — monaco 의 3값
  `'off'|'explicit'|'on'` 중 `'explicit'` 은 노출하지 않고 토글로 좁혀 `'on'`/`'off'` 로 매핑) ·
  `editorSuggestPreview: boolean`(기본 false — monaco `suggest.preview`) ·
  `editorRulers: number[]`(기본 `[]` — monaco `rulers`) ·
  `editorDiffHideUnchangedRegions: boolean`(기본 false — diff 에디터
  `hideUnchangedRegions.enabled`) · `editorDiffShowMoves: boolean`(기본 false — diff 에디터
  `experimental.showMoves`). 7종 모두 `SettingsPatch`·`emptySettingsPatch()`·sync 업로드 동반이며
  원격 dispatch 제외 대상이 아니다(`strip_non_syncable` 무변동).
- **`editorRulers` 정규화 규약**: `sanitize()` 안의 `sanitize_editor_rulers` 가 `Settings` 가
  만들어지는 모든 출구(patch · 손으로 편집한 `settings.json` · 동기화 gist)에서 **1~1000 밖의 열을
  버리고, 중복을 제거하고, 오름차순 정렬하고, 16개로 자른다.** 정렬까지 하는 이유는 설정 화면이
  저장된 목록을 콤마 문자열로 그대로 되돌려 그리기 때문 — 정렬하지 않으면 무관한 설정 변경마다
  사용자가 쓴 순서가 흔들린다. 프론트의 `src/shared/lib/editor-rulers.ts`
  (`parseEditorRulers`)가 같은 규칙을 거울로 두어, 입력칸이 백엔드가 버릴 값을 받아들이지 않는다.
- **diff 옵션의 적용 범위(신규 계약)**: 두 diff 필드는 `DiffView` 를 쓰는 **모든** 표면에 같은 값이
  걸린다 — SCM diff 패널(`widgets/diff-pane`), 커밋 상세 diff(`widgets/commit-file-diff`), 충돌
  비교 다이얼로그(`features/git/conflict-compare-dialog`). 앞의 둘은 각자
  `settingsQueryOptions()` 를 읽고, `features` 레이어인 충돌 다이얼로그는 쿼리를 직접 읽지 않으므로
  `EditorPane` 이 `resolveDiffViewSettingsProps(settings)` 결과를 prop 으로 내려준다.

### d-53 U2 — on-save 정리 2건 (2026-08-29)

> 계약: `docs/acknowledge/2026-08-29-d53-ux-batch-contract.md` §1 U2.
> 발견 정본: `docs/quality-assurance/2026-08-29-full-audit.md` §5 중간 규모(trim/final-newline on-save).

- **커맨드·이벤트 표면 불변**: 신규/삭제 커맨드 0, dispatch 테이블·원격 정책 분류·커맨드 수 변동
  없음. `Settings`/`SettingsPatch` 에 **필드 2종 순증**이라 `bindings.ts` 는 재생성했다.
- **신규 settings 필드(둘 다 VS Code 파리티 기본 false)**:
  `trimTrailingWhitespaceOnSave: boolean`(`files.trimTrailingWhitespace` 대응) ·
  `insertFinalNewlineOnSave: boolean`(`files.insertFinalNewline` 대응). 둘 다
  `SettingsPatch`·`emptySettingsPatch()`·sync 업로드 동반이며 원격 dispatch 제외 대상이 아니다
  (`strip_non_syncable` 무변동). 정규화(`sanitize`)는 없다 — 순수 bool 이다.
- **적용은 전적으로 프론트 저장 파이프라인**이다. `file_save` 의 시그니처·시맨틱은 그대로이고, Rust
  는 이 두 값을 읽지 않는다(저장 커맨드가 받은 문자열을 그대로 쓴다). 정리는 디스크로 보내기 전
  monaco 모델 위에서 끝난다 — 백엔드가 파일 내용을 임의로 가공하지 않는다는 기존 계약을 유지하기
  위한 배치다.
- **파이프라인 순서(신규 계약)**: `widgets/editor-pane/use-editor-file-persistence.ts` 의
  `handleSave` 한 곳에서 **Code Actions on Save(명시 저장만) → format-on-save → 후행 공백 제거 →
  마지막 줄바꿈 → `file_save`** 순으로 돈다. ⌘S·자동 저장·format-on-save 가 모두 이 함수를 지나므로
  트리거별 분기가 없다. 포맷 뒤인 이유는 포맷 결과의 후행 공백까지 정리하기 위해서, 공백 제거가
  줄바꿈보다 앞인 이유는 공백만 있는 마지막 줄을 먼저 비워야 그 뒤에 줄바꿈이 덧붙지 않기 때문이다.
- **자동 저장의 커서 줄 예외**: 자동 저장일 때만 trim 액션에 `{ reason: 'auto-save' }` 를 넘겨
  커서가 있는 줄의 후행 공백을 남긴다(VS Code save participant 의 `isAutoSaved` 와 같은 규약).
  명시적 ⌘S 는 인자 없이 돌아 커서 줄까지 정리한다.
- **정리 구현은 monaco 내장 액션 재사용**: `editor.action.trimTrailingWhitespace` ·
  `editor.action.insertFinalNewLine` — 키맵 카탈로그(`shared/lib/monaco/monaco-actions.ts`)가 이미
  수동 실행 행으로 노출하던 그 액션이고, 액션 id 는 그 파일이 단일 출처다
  (`shared/lib/monaco/on-save-cleanup.ts` 가 import). 두 액션은 `executeCommands` 로 편집하며
  커맨드가 선택을 추적하므로 커서·멀티커서가 보존된다.

### d-53 U3 — EditorConfig (2026-08-29)

> 계약: `docs/acknowledge/2026-08-29-d53-ux-batch-contract.md` §1 U3.
> 발견 정본: `docs/quality-assurance/2026-08-29-full-audit.md` §5 중간 규모(EditorConfig).

- **커맨드·이벤트 표면 불변**: 신규/삭제 커맨드 0, dispatch 테이블·원격 정책 분류·커맨드 수 변동
  없음. 바뀐 것은 `OpenedFile` 응답 1필드와 `Settings`/`SettingsPatch` 1필드라 `bindings.ts` 는
  재생성했다.
- **`OpenedFile` 에 `editorConfig: EditorConfigOptions` 순증**(신규 타입 2종 —
  `EditorConfigOptions` = `indentStyle`(`'tab' | 'space' | null`)·`indentSize`·`tabWidth`
  (`number | null`)·`insertFinalNewline`·`trimTrailingWhitespace`(`boolean | null`),
  `EditorConfigIndentStyle`). **별도 커맨드를 두지 않은 이유**: 파일 열기와 같은 왕복에 실려야
  에디터가 내용을 처음 그리는 렌더에서 이미 들여쓰기를 알고 있다. 쿼리를 하나 더 두면 모델이 틀린
  들여쓰기로 붙었다가 한 틱 뒤에 교정되는 깜빡임이 생긴다. `editor_config_enabled` 가 꺼져 있거나
  (기본값) 파일이 `refused` 티어면 전 필드가 `null` 이다.
- **신규 settings 필드 `editorConfigEnabled: boolean`(기본 false)**. VS Code 도 EditorConfig 를
  코어에 갖지 않고 확장으로 제공하므로 off 가 파리티 기본값이고, 꺼져 있으면 `file_open` 이 체인
  자체를 걷지 않아 비용이 0이다. `SettingsPatch`·`emptySettingsPatch()`·sync 업로드 동반이며
  원격 dispatch 제외 대상이 아니다(`strip_non_syncable` 무변동). 정규화(`sanitize`)는 없다 —
  순수 bool 이다.
- **해석 규약(신규 계약)** — `domain::file::editorconfig`:
  - 파일이 있는 디렉토리에서 위로 올라가며 `.editorconfig` 를 읽고, `root = true` 를 만나면
    멈춘다(없으면 파일시스템 루트까지). **프로젝트 루트를 넘어 올라간다** — 그것이 EditorConfig 의
    정의이며, 읽는 것은 `.editorconfig` 라는 이름의 파일뿐이고 상한(조상 64단·파일당 64KB)이 있다.
  - 적용은 **바깥 파일부터** 이므로 가까운 파일이 이긴다. 한 파일 안에서는 **위에서 아래로** 적용해
    뒤에 오는 매칭 섹션이 이긴다. 값이 `unset` 이면 그 프로퍼티를 지운다.
  - 코어 5키만 남긴다(`indent_style`·`indent_size`·`tab_width`·`insert_final_newline`·
    `trim_trailing_whitespace`). `charset`·`end_of_line` 은 계약 §5 이월.
  - `indent_size = tab` 은 Rust 에서 `tab_width` 로 해석한다(선언이 없으면 `null`). `indent_size`/
    `tab_width` 는 1~64 밖이면 버린다 — 모나코 모델 옵션으로 직행하는 값이라 상한을 둔다.
  - 글롭은 **자체 최소 매처**다(신규 크레이트 0). `*`(구분자 제외)·`**`·`?`·`[abc]`/`[!abc]`/
    `[a-z]`·`{a,b}`(콤마 없는 `{word}` 는 리터럴)·`{n1..n2}`·`\` 이스케이프를 지원한다. 구분자가
    없는 패턴은 `**/` 를 앞에 붙여 임의 깊이에 매칭하고, 있는 패턴은 그 `.editorconfig` 디렉토리
    기준으로 고정한다(선행 `/` 1개 제거). `**/` 는 **디렉토리 0개도 매칭**한다.
- **적용은 프론트**다. `file_save` 의 시그니처·시맨틱은 그대로이고 Rust 는 이 값들을 읽어 파일을
  가공하지 않는다.
  - **들여쓰기**: `shared/lib/editorconfig.ts` 의 `resolveEditorConfigIndentProps` 가
    `editorConfigTabSize`/`editorConfigInsertSpaces` 두 prop 을 만들고, `CodeEditor` 가 이를
    **모델**(`ITextModel.updateOptions`)에 건다. `tabSize`/`insertSpaces`/`detectIndentation` 은
    monaco 의 **전역** 에디터 옵션(`IGlobalEditorOptions`)이라 그 경로로 파일별 값을 보내면 마지막에
    마운트된 페인의 값이 모든 모델에 걸린다 — 그래서 전역 경로에는 기존대로 설정값만 흐르고,
    editorconfig 오버라이드는 모델에만 얹는다. 오버라이드가 없으면 모델을 아예 건드리지 않으므로
    `detectIndentation` 추정 경로는 그대로다.
  - **on-save 정리(U2)**: 같은 파일의 `resolveOnSaveCleanupFlags` 가
    `trim_trailing_whitespace`/`insert_final_newline` 을 전역
    `trimTrailingWhitespaceOnSave`/`insertFinalNewlineOnSave` 보다 우선 적용한다. editorconfig 의
    명시적 `false` 가 전역 `true` 를 이긴다(`??` 폴백).
- **반영 시점은 파일 열기**다. `.editorconfig` 를 편집해도 이미 열린 탭에는 반영되지 않는다 —
  계약 §5 이월. 오버라이드를 **지우는** 방향(섹션 삭제·설정 off)은 탭을 다시 열어도 반영되지
  않는다(모델이 앱 세션 내내 캐시되고, 오버라이드가 없으면 모델을 건드리지 않는다) — `features/editor.md`
  §17 · 계약 §5.

### 사용성 배치 4 — OS 네이티브 알림 (2026-09-04)

> 계약: `docs/acknowledge/2026-09-04-usability-batch4-contract.md` §A.
> 설정 필드 8종은 `docs/data-model.md` §19 가 정본이다.

- **신규 도메인 `notification` · 신규 커맨드 2종.** `IMPLEMENTED_JSON_COMMANDS` 179 → **181**,
  `REMOTE_DENIED_COMMANDS` 24 → **26**, 원격 커맨드 총계 182 → **184**(`REMOTE_ALLOWED_COMMANDS`
  158 은 그대로 — 둘 다 거부다). 이벤트 신설 0.
- **`notification_notify(category, title, body) → NotificationDelivery`** — 완료성 이벤트 하나를 OS
  알림 센터로 보낸다. `category` 는 `NotificationCategory`
  (`'agentCompleted' | 'taskCompleted' | 'gitRemote' | 'searchReplace' | 'lspInstall' | 'error'`).
  - **게이트는 Rust 소유**다(`domain::notification::service::decide_delivery`, 순수 함수):
    ① `notificationsEnabled` → ② 카테고리 스위치 → ③ `notificationsOnlyWhenUnfocused &&
    webview_windows().values().any(is_focused)`. 순서는 보고 순서이기도 하다 — 세 조건이 동시에
    성립하면 사용자가 **먼저 뒤집어야 할** 스위치를 사유로 돌려준다.
  - **포커스 판정을 프론트에 두지 않는 이유**: 창마다 JS realm 이 달라 각 창은 자기 포커스만 알고,
    보조 창을 보고 있는 사용자에게 메인 창이 알림을 쏜다. 또 트리거가 전 창 브로드캐스트
    이벤트라 프론트 판정은 열린 창 수만큼 중복 발화한다.
  - **반환 타입** `NotificationDelivery = { outcome: 'delivered' } | { outcome: 'suppressed';
    reason: NotificationSuppressionReason }`, `NotificationSuppressionReason =
    'notificationsDisabled' | 'categoryDisabled' | 'windowFocused'`.
    **`delivered` 는 "플러그인에 넘겼다"는 뜻이지 "macOS 가 표시했다"가 아니다** —
    `tauri-plugin-notification` 2.4.0 의 데스크톱 백엔드는 전달을 spawn 하고 결과를 버리며
    (`src/desktop.rs` 의 `let _ = notification.show()`), 권한 조회는 항상 `Granted` 를 돌려주는
    스텁이다. 즉 **"사용자가 TAIDE 알림을 꺼 뒀다"는 앱에서 관측 불가능**하다. 관측 가능한 것은
    `suppressed` 뿐이며, 그래서 설정 화면의 "테스트 알림" 버튼이 무음 결과를 설명할 수 있다.
  - **텍스트는 프론트 소유**다. `title`/`body` 는 이미 번역된 문자열로 도착하고 Rust 는 해석하지
    않는다 — `t()` 카탈로그와 이벤트 데이터를 가진 쪽이 프론트다.
- **`notification_open_system_settings() → null`** — macOS 시스템 설정의 알림 창을 연다.
  권한 거부를 감지할 수 없으므로(위) 신호에 반응하는 대신 설정 화면에서 **상시 제공**한다.
  - `system_open_external_url` 을 재사용하지 않는다: 그 커맨드의 `validate_external_url` 은
    `http(s)://` 만 허용하고 그 좁음이 존재 이유다(§4). 대신 URL 인자를 아예 받지 않고
    `constants::MACOS_NOTIFICATION_SETTINGS_URL`
    (`x-apple.systempreferences:com.apple.Notifications-Settings.extension`) 하나만 연다.
  - macOS 외 타깃은 `error.notification.settingsUnsupported`(`InvalidArgument`). 확장 번들 id 는 OS
    버전 의존이고 `/usr/bin/open` 은 detached spawn 이라 실패가 무음이다 — 설정 화면이 수동 경로를
    텍스트로 함께 보여준다.
- **원격**: 둘 다 `REMOTE_DENIED_COMMANDS` + `RemoteDenialPolicy::UnreachableDesktopWindow`
  (위 정책 표). 플러그인 커맨드(`plugin:notification|notify`)는 애초에
  `IMPLEMENTED_JSON_COMMANDS` 밖이라 원격 dispatch 화이트리스트에 존재하지 않는다 — 미러가
  데스크톱 알림을 쏘는 경로는 구조적으로 없다.
- **capabilities**: `main.json` 에 `notification:allow-is-permission-granted` **1개만** 추가했다.
  앱 코드가 그 커맨드를 부르기 때문이 아니라, 플러그인의 js init 스크립트가 창 로드마다
  `.catch` 없이 그것을 invoke 하기 때문이다 — 미개방이면 부팅마다 unhandled rejection 이
  `shared/lib/error-log-forwarding.ts` 를 통해 파일 로그에 error 로 남는다. 알림 발송 자체는
  Rust 의 `NotificationExt` 로 하므로 `notification:allow-notify` 는 열지 않는다(NFR-7).
- **JS 게스트 패키지(`@tauri-apps/plugin-notification`)는 설치하지 않는다** — 그 API 는 플러그인이
  주입하는 `window.Notification` shim 에 의존하는데 원격 미러는 그 shim 을 받지 않고, 어차피 모든
  발송이 Rust 게이트를 지나야 한다.

### 사용성 배치 4 — 프로젝트 표시 설정 (2026-09-04)

> 계약: `docs/acknowledge/2026-09-04-usability-batch4-contract.md` §D.
> 영속 스키마는 `docs/data-model.md` §20 이 정본이다.

- **신규 커맨드 1종 `project_set_display`.** `IMPLEMENTED_JSON_COMMANDS` 181 → **182**,
  `REMOTE_ALLOWED_COMMANDS` 158 → **159**(`REMOTE_DENIED_COMMANDS` 26 은 그대로), 원격 커맨드
  총계 184 → **185**. 이벤트 신설 **0** — 완료 시 기존 `project:list-changed` 를 재발행한다.
- **`project_set_display(projectId, patch: ProjectDisplayPatch) → null`** — 시그니처·규약·거부
  조건은 위 project 절이 정본이다. 요약: 세 축(`icon`·`label`·`color`)이 각각 생략 = 유지 /
  빈 문자열 = 해제 / 그 외 = 교체이고, 한 축이라도 규격 위반이면 호출 전체가
  `error.project.displayInvalid`(`InvalidArgument`)로 거부된다.
- **원격 허용인 이유**: 쓰기 대상이 `project_reorder` 와 같은 두 로컬 파일(`session.json` +
  해당 `projects/<id>/project.json`)이고, 값은 열거된 아이콘 이름·짧은 라벨·lane 색 토큰뿐이라
  원격 세션이 이미 볼 수 있는 것 밖의 경로·자원을 드러내지 않는다. 원격 미러는 같은 SPA 를
  서빙하므로 사이드바가 원격에서도 렌더되고, `ProjectListChanged` 는 이미
  `fanout_remote_events!` 에 등재돼 있어 원격 세션도 같은 이벤트로 갱신된다.
- **조회 커맨드 반환 타입만 넓어졌다**: `project_list`(`ProjectRef[]`)·`project_get`·
  `project_list_recent`(`Project`)에 `display?: ProjectDisplay` 가 생겼다. `#[serde(default)]`
  이므로 specta 가 optional 로 생성하며, 프론트는 폴백을 소비처마다 적지 않고
  `shared/lib/project-display.ts` 한 곳에서만 해석한다(감사 R5#5 의 `??` 드리프트 방지).

### 사용성 배치 4 — 터미널 컨텍스트 메뉴의 분할 (2026-09-04)

> 계약: `docs/acknowledge/2026-09-04-usability-batch4-contract.md` §F.2-1.
> 조사 원문: `docs/research/2026-09-04-batch4-terminal-tabbar-context-menu-research.md` §4·§5.

- **신규 커맨드 1종 `layout_open_tab_in_split`.** `IMPLEMENTED_JSON_COMMANDS` 182 → **183**,
  `REMOTE_ALLOWED_COMMANDS` 159 → **160**(`REMOTE_DENIED_COMMANDS` 26 은 그대로), 원격 커맨드
  총계 185 → **186**. 이벤트 신설 **0** — 기존 `layout:changed` 를 정확히 1회 발행한다.
- **`layout_open_tab_in_split(request: OpenTabInSplitRequest) → ProjectLayout`** — `target_pane`
  옆(`edge` 방향)에 **새 페인을 만들고 그 안에 새 탭을 만든다.** `layout_split` 과 정반대의 의미다:
  그쪽은 **이미 있는 탭을 옮긴다**(`extract_tab` → 새 Leaf). 터미널 우클릭의 "분할"은 VS Code 처럼
  "그 방향에 새 터미널을 띄운다"는 뜻이라 옮기기로는 표현할 수 없다.
  - `OpenTabInSplitRequest = { projectId, targetPane, edge: DropEdge, kind: TabKind, title,
    preview?: boolean }`. 6필드를 한 구조체로 묶은 것은 **`clippy::too_many_arguments`(상한 7)**
    때문이다 — `AppHandle`·`State` 를 더하면 8이 된다. `lsp_spawn` 의 `LspSpawnRequest`,
    `pty_spawn` 의 `PtySpawnOptions` 와 같은 선례를 따랐다. `preview` 는 `#[serde(default)]`
    (specta optional)이라 영구 탭만 여는 호출자는 생략한다.
  - **`edge` 는 방향성 4종만** 받는다. `DropEdge::Center` 는 "이 페인 안에 열기"라는 뜻이고 그것은
    `layout_open_tab` 의 일이므로 `InvalidArgument` 로 거절한다 — `layout_split` 이 Center 를
    `move_tab` 으로 접어 넘기는 것과 다르다(옮길 탭이 애초에 없다).
  - **`File` kind 선검증은 `layout_open_tab` 과 완전히 같다**: 커맨드 경계에서
    `ensure_file_tab_target_exists`(→ `root_guard::resolve_owning_project` +
    `root_guard::ensure_existing_file`)를 먼저 통과해야 한다(위 §layout 의 배치 3 절이 정본).
    두 커맨드가 같은 헬퍼를 공유하므로 "탭은 열리고 본문만 `os error 2`" 를 새 진입점이 되살릴 수 없다.
  - **`open_tab` 의 kind 동등 중복 제거를 타지 않는다.** `TabKind::Terminal { sessionId: "", cwd }`
    두 개는 서로 같은 kind 라, 세션 id 가 아직 박히지 않은 터미널 탭이 있으면 `layout_open_tab` 은
    새 탭 대신 그 탭을 활성화한다. 사용자는 "새 페인"을 요청한 것이므로 이 경로는 항상 새 탭을 만든다.
- **왜 커맨드를 신설했는가(`layout_open_tab` + `layout_split` 2회 조합이 아닌 이유)** — 조합은 세
  결함을 동시에 만든다: ① `open_tab` 이 새 탭을 **원래 페인에서** 활성화해 보고 있던 터미널 view 가
  즉시 unmount 되고(xterm dispose → 재마운트 시 ring buffer 전량 replay), ② 새 터미널 탭이 마운트
  즉시 스폰을 시작한 뒤 이어지는 split 이 그 탭을 다른 Leaf 로 옮겨 재마운트시키므로 **셸이 두 번
  스폰**되고(감사 §4-B A6/C14 의 고아 셸 패턴), ③ 위의 kind 동등 중복 제거로 새 탭이 아예 안 생길 수
  있다. 단일 커맨드는 `begin_mutation` 1회·`layout:changed` 1회·기존 페인 unmount 0 이라 이 셋이
  **구조적으로 표현 불가능**하다.
- **트리 삽입 로직은 `layout_split` 과 공유한다**: `layout::service::insert_new_leaf(layout,
  target_pane, edge, new_leaf)` — 루트 Leaf 면 `wrap_leaf_in_split`, 부모 Split 의 방향이 같으면
  형제 삽입 + 대상 share 반분, 다르면 대상 Leaf 만 감싸기. 이어 새 Leaf 를 그 트리의 포커스 페인으로
  잡고 `normalize` · `ensure_focused_pane_valid` · `revision += 1` 까지 한 곳에서 한다. `split` 은
  `extract_tab` 한 탭을, `open_tab_in_split` 은 갓 만든 탭을 같은 함수에 넘길 뿐이라
  **분할 결과 트리 모양은 두 경로가 항상 동일**하다(회귀 테스트로 고정).
- **원격 허용인 이유**: 이미 허용된 `layout_open_tab`(탭 생성)과 `layout_split`(페인 생성)의 합집합
  이상을 하지 않는다 — 새 자원·새 경로를 드러내지 않고, 파일 탭이면 두 커맨드와 같은 root_guard 를
  탄다. 원격 미러도 `pty_*` 를 이미 갖고 있어 새로 생긴 페인의 터미널이 그대로 붙는다.
- **로케일 5키 순증**(`terminal.copy`·`terminal.paste`·`terminal.selectAll`·`terminal.clear`·
  `terminal.kill`) — 분할 방향 라벨은 기존 `editorArea.splitTop/Bottom/Left/Right`,
  "새 터미널"은 기존 `tab.newTerminal` 을 재사용한다.
