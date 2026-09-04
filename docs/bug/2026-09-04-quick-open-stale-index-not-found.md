# 퀵오픈(⌘P)으로 연 파일이 "찾을 수 없는 파일" — 낡은 인덱스 + 검증 없는 탭 열기 (수정)

> 사용자 실기 보고(2026-09-04): "⌘P 로 파일을 열면 찾을 수 없는 파일이라고 뜬다".
> 계약 정본은 `docs/acknowledge/2026-09-04-usability-batch3-contract.md` §A, 기능 정본은
> `docs/features/command-palette.md` §3.1, 진행 상태는 `docs/PROCESS.md` "사용성 배치 3" 절.

## 증상

⌘P 에서 파일을 고르면 탭은 열리지만 에디터 본문이 비어 있고, 그 자리에 빨간 원문 오류
`No such file or directory (os error 2)` 가 표시된다. 로케일 문구가 아니라 OS 원문이 그대로 나온다.
정상 목록처럼 보이는 행을 눌렀는데 실패하므로 사용자에게는 "팔레트가 없는 파일을 보여준다"로 읽힌다.

## 원인 (4축 — 전부 소스로 확인)

1. **인덱스에 캐시 계층이 프론트 한 장뿐이다.** `search_list_files`
   (`src-tauri/src/domain/search/commands.rs`)는 호출마다 프로젝트 전체를 새로 walk 하고 Rust 측
   캐시가 없다. 즉 퀵오픈 후보의 유일한 저장소는 `QUERY_KEY.SEARCH.PROJECT_FILES(projectId)` 쿼리
   캐시이고, 이 배열이 갱신되지 않으면 그대로 낡는다.
2. **인앱 뮤테이션이 그 캐시를 무효화하지 않았다.** `entities/file/file.query.ts` 의
   `useCreateEntry` 는 무효화가 0건, `useRenameEntry`·`useCopyEntry`·`useDeleteEntry` 는
   `FILE.CONTENT`·`GIT.PROJECT` 만 무효화했다. 앱 안에서 파일을 지우거나 개명해도 팔레트 인덱스는
   옛 경로를 계속 들고 있었다. 무효화하는 유일한 경로는 fs 워처 에코
   (`app/providers/ipc-sync-provider.tsx`, 디바운스 `WATCH_DEBOUNCE_MS`)뿐이었다.
3. **무효화가 닿아도 낡은 배열이 먼저 렌더된다.** 팔레트가 닫혀 있으면 이 쿼리는 `enabled: false`
   라, 무효화는 stale 마킹만 하고 refetch 하지 않는다. 다시 ⌘P 를 누르면 옛 배열을 즉시 그리고
   뒤에서 re-walk 한다. 팔레트는 `isFetching`/`isError` 를 읽지 않아(`isPending` 만 봄) 그 창이
   화면상 구분되지 않았고, 죽은 행이 그대로 클릭 가능했다.
4. **탭 열기가 경로를 검증하지 않았다.** `layout_open_tab` 은 `TabKind::File { path }` 를 그대로
   받아 탭을 만든다. 실패는 탭이 열린 *뒤* `file_open` 의 `std::fs::metadata` 에서 ENOENT 로
   드러나고, `From<io::Error>` 가 `AppError::NotFound(value.to_string())` — localeKey 없음 — 로
   변환해 프론트가 원문으로 폴백했다(`error.file.notFound` 키 자체가 없었다).

기각된 가설: cmdk 인덱스 밀림(`shouldFilter={false}`), 다중 루트 혼입(전부 `activeProjectId` 키잉),
심링크·대소문자·NFC 정규화 차이(정규화 코드 0건), 루트 경계 검사(루트 밖은 `Forbidden`).

## 해결

원인 2축(인덱스 신선도 · 열기 검증)을 각각 **단일 지점**에서 닫는다.

- **열기 선검증 (Rust)**: `layout_open_tab` 이 `File` kind 면 루트 경계 검사 후
  `metadata(..).is_file()` 을 확인하고, 아니면 탭을 열지 않고 `error.file.notFound` localized 에러를
  반환한다. 존재 검사 자체는 `infra::root_guard::ensure_existing_file` 한 함수이고, IDE MCP `openFile`
  핸들러도 — 도메인 경계상 커맨드 함수를 부를 수 없어 `layout::service::open_tab_and_finish` 를 직접
  호출하므로 — 같은 함수를 자기 쪽에서 부른다. 파일 탭을 여는 두 진입점이 모두 이 게이트를 탄다.
  `file::service::open_file` 의 `NotFound` 도 같은 키로 매핑해 에디터 본문·토스트에서
  로케일 문구가 보인다. `search_list_files` 는 비-UTF8 파일명을 U+FFFD 로 치환한 **존재하지 않는
  경로**를 내보내던 것도 함께 제외한다.
- **프론트 단일 진입점**: `entities/layout/layout.query.ts` 에 `useOpenFileTab` 을 신설하고 파일 탭을
  여는 호출부 전부(explorer 4 · palette 2 · search-editor · git · problems · editor-area ·
  breadcrumbs · welcome · agent-external-open)를 여기로 모았다. 이 훅이 에러 토스트를 담당하고,
  `isNotFoundIpcError(error)` 이면 `SEARCH.PROJECT_FILES(projectId)` 를 그 자리에서 무효화한다 —
  같은 죽은 행이 다음 ⌘P 에 다시 나오지 않는다.
- **인덱스 신선도 계약**: `file.query.ts` 의 `invalidateProjectFileIndex` 헬퍼를
  `useCreateEntry`·`useRenameEntry`·`useCopyEntry`·`useDeleteEntry` 의 `onSuccess` 에 걸고
  `refetchType: 'all'` 로 무효화한다(사용자 1회 조작 = walk 1회). 팔레트가 닫혀 있는 동안은
  `enabled:false` 라 재-walk 가 **다음 ⌘P 오픈 시점**으로 미뤄지고(TanStack Query 의
  `refetchQueries` 는 `refetchType` 과 무관하게 disabled 쿼리를 제외한다 — `command-palette.md`
  §3.1), 그 첫 프레임의 낡은 배열은 갱신 표시 + 열기 선검증이 막는다. 워처 에코는 기본 `refetchType`
  유지(외부 대량 변경의 walk 폭주 방지).
  `useCreateEntry` 는 무효화에 필요한 `projectId` 를 받도록 시그니처를 바꿨다.
- **갱신 중 표시**: 팔레트가 열린 채 재-walk 가 도는 창을 `files` 그룹 헤딩의
  `palette.filesRefreshing` 스피너로 드러낸다. 항목 클릭은 막지 않는다 — 선검증이 방어한다.

## 검증

- `src/shared/lib/ipc-error-message.test.ts` — `isNotFoundIpcError` 2건(맨 `NotFound` 와 `Localized`
  로 감싸인 `NotFound` 둘 다 true / 다른 코드·비-IpcError 는 false).
- Rust 측 `layout_open_tab` 선검증·`search_list_files` 비-UTF8 제외 테스트는 Rust 트랙에서 담당한다.
- 수동 QA: 앱에서 파일 삭제·개명 직후 ⌘P 를 열어 그 경로가 목록에 없는지, 외부에서 삭제한 파일을
  ⌘P 로 골랐을 때 빈 탭 대신 로케일 토스트가 뜨고 재-⌘P 목록에서 사라지는지.

## 이번 범위 밖 (백로그 등재)

- `target: null` 이 가리키는 `focusedPane` 이 사라진 상태에서 파일을 열면
  `layout/service.rs` 가 `NotFound("pane not found")` 를 돌려주어, 파일은 멀쩡한데 같은 토스트가
  뜬다. 판정을 `NotFound` 코드 하나로 하기 때문에 인덱스 무효화까지 함께 일어난다(무해하지만
  불필요한 walk 1회). → `docs/backlog.md`.
- Rust 측 `FsChanged` 기반 인덱스 캐시는 별도 배치(`docs/backlog.md` "search_list_files FsChanged
  캐시"). 이번에는 무효화 계약으로 스테일 창만 닫았다.
