# TAIDE 아키텍처

> 시스템 구조의 정본. 상태 소유권 원칙은 ADR-0004, IPC 상세 계약은 `docs/ipc-contract.md`,
> 영속화는 `docs/data-model.md`, 각 기능 내부는 `docs/features/*.md`.

## 1. 전체 구조

계층은 세 개다. 의존은 아래 방향(문서 표기상 왼쪽→오른쪽)으로만 흐른다.

```
React view (표시 전용)  →  IPC 경계 (typed commands / events)  →  Rust 코어 (상태·자원 소유)
```

- **Rust 코어**: 모든 도메인 상태의 단일 출처. pty·git·LSP 프로세스·파일 와처 등 시스템 자원 소유.
- **IPC 경계**: query / mutation / event 3종으로 고정된 타입 계약. Rust 타입에서 TS 타입을 자동 생성해
  드리프트를 차단한다(ADR-0011).
- **React view**: 조회(캐시)·표시·입력 전달. 도메인 상태를 만들지 않는다. reload 하면 Rust 상태에서
  전체 화면이 복원된다.

## 2. Rust 코어 구조

```
TAIDE/                       (Cargo workspace — members: src-tauri, crates/taide-cli)
├── Cargo.toml               워크스페이스 루트. release 프로파일도 여기 (멤버에 두면 무시된다)
├── crates/taide-cli/        `taide` CLI (--wait 마커 방식 — agent-integration.md §2)
│                            **bin 이름은 `taide-cli`** — `taide` 로 두면 앱 바이너리와 출력이 충돌한다
└── src-tauri/
    ├── src/
    │   ├── main.rs          진입점 (lib.rs 의 run() 호출만)
    │   ├── lib.rs           부트스트랩: 커맨드 등록·플러그인·AppState·복원·폴링 태스크·종료 정리
    │   ├── state.rs         AppState — 전 도메인 상태의 루트 (parking_lot RwLock + mutation guard)
    │   ├── error.rs         AppError — 코드·메시지 중앙화, 모든 command 의 Result 에러 타입
    │   ├── events.rs        이벤트 payload 타입 (ipc-contract 의 Rust 측 정본)
    │   ├── ids.rs           ProjectId / PaneId / TabId (newtype, serde transparent)
    │   ├── paths.rs         앱 데이터 디렉토리 경로 규칙 (data-model.md §2)
    │   ├── constants.rs     무시 목록·파일 크기 4단계 임계값 (워처·트리·검색이 공유)
    │   ├── domain/          도메인 로직 (한 도메인 = 한 모듈)
    │   │   ├── app/         앱 정보 (버전·플랫폼)
    │   │   ├── project/     프로젝트 열기/닫기/목록, watcher 부착, capability 감지
    │   │   ├── layout/      탭·스플릿·포커스 (PaneNode 트리)
    │   │   ├── file/        파일 열기/저장/생성/이동/삭제, 크기 정책, dirty 미러
    │   │   ├── tree/        파일 트리 (Rust 소유 + flat rows 페이지네이션)
    │   │   ├── terminal/    pty 세션, 링버퍼, 셸 프로필, 터미널 경로 해석
    │   │   ├── git/         status/diff/blame/log/stage/commit/push + watch.rs(무효화 분류)
    │   │   ├── lsp/         LSP 세션 관리, 서버 감지, 루트 탐지
    │   │   ├── search/      프로젝트 전역 텍스트 검색 (자체 병렬 스캔 + regex)
    │   │   ├── agent/       에이전트 감지, wait 마커, CLI 설치 상태
    │   │   ├── theme/       테마 로드/해석/내장 테마 + 사용자 테마 저장·삭제 (7.5-D)
    │   │   ├── locale/      번역 메시지 로드/병합 + 사용자 언어팩 (7.5-H)
    │   │   ├── font/        시스템 폰트 열거 (fontdb) (7.5-D)
    │   │   ├── settings/    앱 설정
    │   │   └── plugin/      플러그인 매니페스트 로드·검증
    │   └── infra/           외부 자원 어댑터
    │       ├── pty.rs       portable-pty 래퍼 (배칭·flow control·링버퍼)
    │       ├── lsp_proc.rs  LSP 자식 프로세스 + JSON-RPC 프레이밍
    │       ├── watcher.rs   notify + debouncer (무시 목록 필터)
    │       └── persist.rs   원자적 쓰기 (temp → fsync → rename)
    ├── tests/               도메인 경계를 넘는 통합 테스트 (session_restore.rs)
    └── capabilities/        Tauri 권한 정의 (최소 권한 — NFR-7)
```

> **초안 대비 실제 구현에서 달라진 것**
> - `infra/repo.rs`(git2 래퍼)는 만들지 않았다 — git2 호출이 `domain/git/service.rs` 안에 있다.
> - `infra/proc.rs` 대신 용도별로 `infra/pty.rs` 와 `infra/lsp_proc.rs` 로 나뉘었다.
> - `src/cli/` 가 아니라 **별도 크레이트 `crates/taide-cli`** 다 (워크스페이스 구성).
> - 초안에 없던 도메인 5개가 추가됐다: `app`, `tree`, `agent`, 그리고 Phase 7.5 에서 `locale`·`font`.
>   `locale` 은 **테마와 완전히 같은 구조**다(내장 정의 + 사용자 파일 열거 + `extends` 부분 병합).
>   같은 문제를 두 번 푸는 대신 검증된 구조를 재사용했다.
> - 도메인별 저장소(`TreeStore`·`TerminalStore`·`GitStore`·`LspStore`·`SearchStore`·`PluginStore`·
>   `AgentStore`)는 `state.rs` 가 아니라 각 도메인 `commands.rs` 에 정의하고 `app.manage()` 로 등록한다.
>   (병렬 구현 시 `state.rs` 충돌을 피하려는 선택 — 결과적으로 도메인 응집도가 높아졌다)

- 각 domain 모듈은 `commands.rs`(IPC 노출) / `service.rs`(로직) / `types.rs`(직렬화 타입)로 나눈다.
  command 는 얇게: 파라미터 검증 → service 호출 → 이벤트 발행. 로직은 service 에만 둔다.
- domain 은 infra 를 trait 경유로 사용한다(테스트에서 인라인 구현으로 대체 — 백엔드 컨벤션의
  ServiceDb 격리와 같은 취지).
- 도메인 간 직접 호출은 금지하고, 필요하면 이벤트 버스(내부 broadcast channel) 또는 상위 조립부를 거친다.

### 2.1 스레딩 모델

- Tauri 기본 tokio 런타임 사용. 명령은 원칙적으로 `async fn`.
- 블로킹 작업(git2 호출, 대형 파일 IO)은 `spawn_blocking` 으로 격리한다 — libgit2 는 동기 API 다.
- 장수 태스크(pty reader, LSP stdio pump, watcher)는 tokio task 로 상주하며,
  소유 도메인의 세션 구조체가 JoinHandle/CancellationToken 을 보유해 종료를 보장한다.
- AppState 접근은 짧은 잠금 원칙: 잠금 안에서 IO 금지.

## 3. 프로젝트 추상화 (핵심 확장 포인트)

프로젝트는 "폴더 + 부착된 capability 집합"이다. 미래 기능(remote-control 등)은 새 capability 로 부착한다.

```rust
struct Project {
    id: ProjectId,          // 안정 식별자 (경로 해시 아님 — 이동 대비 UUID, data-model.md)
    root: PathBuf,
    name: String,
    capabilities: CapabilitySet,
}

trait ProjectCapability {
    fn kind(&self) -> CapabilityKind;              // Git, Lsp, Terminal, AgentWatch, RemoteControl...
    async fn attach(&mut self, ctx: &CapabilityCtx) -> Result<()>;   // 자원 획득, 초기 스캔
    async fn detach(&mut self) -> Result<()>;      // 자원 해제 — 반드시 대칭
}
```

- `CapabilityCtx` 는 프로젝트 루트, 이벤트 발행 핸들, 설정 접근을 제공한다.
- 프로젝트 열기 = core(파일 접근·레이아웃) 초기화 + 감지된 capability 자동 부착
  (예: `.git` 있으면 Git, 지원 언어 파일 있으면 해당 LSP lazy 부착).
- capability 는 각자 명령·이벤트 네임스페이스를 가진다(`git_*`, `lsp_*`, ...).
- **remote-control(미래)**: `RemoteControl` capability 가 프로젝트별 로컬 서버를 열고 웹 패널을 서빙하는
  형태로 부착된다. 코어는 capability 등록 API 외에 어떤 전제도 갖지 않는다.

## 4. IPC 경계

패턴은 3종으로 고정한다. 상세·전체 목록은 `docs/ipc-contract.md`.

| 종류 | 방향 | 형태 | 예 |
|------|------|------|-----|
| query | view→Rust | `invoke('git_status', { projectId })` → 데이터 반환 | 상태 조회. 부수효과 금지 |
| mutation | view→Rust | `invoke('git_stage', { projectId, paths })` → Ack/결과 | 의도 전달. 성공 시 Rust 가 이벤트 발행 |
| event | Rust→view | `emit('git:status-changed', payload)` | view 는 관련 query invalidate |

- 고속 스트림(터미널 출력, LSP 알림)은 이벤트 대신 **Channel** 을 사용한다(연구 결과에 따라 확정,
  `docs/research/tauri-v2.md`).
- Rust 타입 → TS 타입 자동 생성(ADR-0011)으로 계약을 단일 출처화한다.
- 명령 이름은 `snake_case` `{domain}_{action}`, 이벤트 이름은 `{domain}:{event-kebab}` 으로 통일한다.

## 5. View(React) 구조 — FSD

컨벤션 fsd.md 를 데스크톱 SPA 에 맞게 적용한다. pages 레이어는 사용하지 않는다(단일 윈도우 앱,
라우팅 없음 — app 이 직접 widgets 를 조립).

```
src/
├── app/                     진입점, 프로바이더(QueryClient, 테마), 전역 레이아웃 조립
├── widgets/                 IPC 를 소비하는 조립 블록 (비즈니스 로직 O)
│   ├── app-shell/           최상위 셸 조립 (사이드바 | 탐색 | 에디터 영역)
│   ├── app-sidebar/         프로젝트 목록·아이콘 상태·세로 DND
│   ├── editor-area/         pane 트리 렌더 + 탭 바 + 단일 DndContext (탭 DND·5분할 드롭)
│   ├── editor-pane/         Monaco 마운트, git gutter·blame 데코, LSP 세션 연결
│   ├── terminal-pane/       xterm 마운트, flow control, 세션 spawn/attach
│   ├── explorer/            파일 트리(가상 스크롤) + 뷰 전환(파일/검색/Git)
│   ├── search-panel/        전역 검색 결과
│   ├── git-panel/           changes·graph·commit UI
│   ├── diff-pane/           diff 탭 (Monaco DiffEditor)
│   ├── settings-view/       설정 화면
│   └── command-palette/     ⌘⇧P / ⌘P
├── features/                순수 표시 컴포넌트 (props+콜백만 — 비즈니스 로직 X)
├── entities/                IPC 데이터 계층 (도메인별)
│   └── {domain}/
│       ├── {domain}.ipc.ts      invoke 래퍼 (자동 생성 바인딩 사용)
│       ├── {domain}.query.ts    TanStack Query 훅 + queryOptions + 이벤트 구독→invalidate
│       └── {domain}.type.ts     생성된 타입 re-export/파생
└── shared/                  ui(shadcn vendored)·hooks·constants·lib·api(생성 bindings)
    ├── api/bindings.ts      **tauri-specta 생성물** (커밋 대상, 직접 수정 금지)
    ├── lib/monaco/          monaco setup(worker 배선)·테마 파생 — **monaco 접근은 반드시 이 경유**
    ├── lib/lsp/             자체 경량 LSP 클라이언트 + Monaco 어댑터 10종 (ADR-0007)
    └── constants/           query-key(중앙 관리)·platform·terminal
```

- 서버 상태 = "Rust 상태"로 치환해 query.md 컨벤션을 그대로 적용한다: queryOptions 팩토리,
  QUERY_KEY 중앙 관리, 이벤트 수신 시 invalidate (ADR-0008).
- view 전역 상태(zustand)는 순수 UI 상태(드래그 중 표시 등)가 실수요를 증명할 때만 도입한다(frontend.md §6).

### 5.1 `widgets` ↔ `features` 배치 기준 (T1-F, fsd.md §2.1 데스크톱 특화)

eslint `no-restricted-imports` 는 import **방향**만 강제하고 레이어의 **성격**은 검사하지 않는다 —
그 결과 감사(C1)에서 explorer·problems·search·outline·settings·plugin·snippet·tab 8개 슬라이스가
두 레이어에 걸쳐 있는 것이 드러났다. 이 프로젝트에서 두 레이어를 가르는 유일한 판정 기준은
**query/mutation/IPC 소유 여부**다(fsd.md §2.1 의 "props 로만 받는가" 질문을 이 앱의 "Rust 상태 =
서버 상태" 축으로 구체화한 것).

- **`widgets` 로 판정**: 컴포넌트/훅 자신이 `useQuery`·`useMutation`·`useSuspenseQuery`(entities
  query 훅 포함) 또는 `entities/*.ipc.ts` 의 invoke 래퍼를 **직접 호출**한다. 전역 키맵·전역
  이벤트 브리지 구독처럼 IPC 를 감싸는 부수효과도 여기 포함된다.
- **`features` 로 판정**: 위 어느 것도 호출하지 않고, 데이터·콜백을 전부 **props 로만** 받는다.
  타입 import(`@entities/*/*.type.ts`, `@shared/api/bindings`)는 이 판정에 영향을 주지 않는다 —
  entities 는 두 레이어 모두에서 항상 허용된다(§2 fsd.md 매트릭스).
- **`shared` 로 강등**: React 에 의존하지 않는 순수 함수/로직이면서 소비처가 widgets 한 곳뿐이어도
  `shared/lib` 이 맞다 — `features` 는 "컴포넌트" 레이어이므로 React 와 무관한 유틸을 두는 자리가
  아니다.
- **판정 결과 예시(2026-08-18 이동)**: `agent-hooks-project-row`/`-list`·`agent-cli-status-row`·
  `use-zen-mode`(조회+변이+IPC 소유) → `widgets/settings-view`·`widgets/app-shell` 로 승격.
  `tab-context-menu`·`sortable-tab`·`file-tree`·`problems-panel`·`search-panel`·`outline-panel`·
  `plugin-list-body`·`vsix-import-grammars-section`·`snippet-entry-editor`·`snippet-file-list`
  (props+콜백만) → `features/*` 로 강등. `vsix-theme-import`·`selection-line-range`(React 무관
  순수 함수) → `shared/lib` 로 강등. `snippet-entry-editor` 는 `widgets/snippet-editor/
  snippet-draft.ts`(드래프트 변환 타입·순수 함수, React 무관)를 import 하고 있어 그 파일까지
  `shared/lib/snippet-draft.ts` 로 함께 강등해야 features→widgets 역참조 없이 이동이 성립했다
  (Phase D 접합부 수정 — F 1차 실행 시 이 종속을 확인하지 못해 10개 강등 대상 중 이 한 파일만
  누락돼 있었다).
- 판정이 애매하면 **더 아래 레이어**를 고른다(fsd.md §2.1) — 승격은 실제 사용처가 생겼을 때 언제든
  가능하지만, 강등은 소비처 전체를 다시 훑어야 한다.

## 6. 수명주기·누수 방지 규칙 (전 기능 공통)

1. **구독은 생성자와 해제가 한 곳에**: `listen()` 은 반드시 unlisten 을 useEffect cleanup 에서 호출.
   커스텀 훅 `useTauriEvent(name, handler)` 하나로 표준화하고 직접 listen 을 금지한다.
2. **무거운 객체는 dispose 의무**: Monaco model/editor, xterm 인스턴스는 소유 위젯 unmount 시 dispose.
   전역 캐시에 남기는 경우(모델 재사용) LRU 상한과 방출 정책을 명시한다(`features/editor.md`).
3. **Rust 자원은 세션 구조체가 소유**: pty·LSP·watcher 는 세션 drop 시 자식 프로세스 종료까지 보장
   (Drop 구현 + 명시적 shutdown 경로 이중화).

   **§6.3 `project_close` 자원 회수 목록 (정본)** — `project::commands::project_close`
   (`domain/project/commands.rs`)가 프로젝트 종료 시 명령형으로 회수해야 하는 전체 목록이다.
   새 도메인이 프로젝트 수명에 묶인 상태를 추가하면 이 표에도 함께 추가한다 (T1-I 의
   `ProjectCapability::attach/detach` 확장점이 구현되기 전까지는 이 명령형 목록이 유일한 계약이다).

   | 자원 | 회수 방법 | 실패 시 |
   |---|---|---|
   | `dirty_layouts`/`layouts` | 남은 dirty 레이아웃 동기 flush 후 두 맵에서 제거 | 미저장 레이아웃 유실 |
   | `watchers`/`git_watchers` | 맵에서 제거 (핸들 drop이 watcher 스레드 종료) | 닫힌 프로젝트 파일 변경을 계속 감시 |
   | pty 세션 | `TerminalStore::kill_project` | 프로세스+fd+스레드가 앱 종료까지 잔존 |
   | `GitStore` (projectId→repo_root 캐시) | `GitStore::remove` | 재오픈 시 옛 repo_root 캐시 부활 |
   | `TreeStore` (트리 캐시) | `TreeStore::remove` | 재오픈 시 옛 디렉터리 목록 부활 + 메모리 잔존 |
   | asset 프로토콜 접근 | **회수 불필요** — 아래 참고 (T1 2차, X1#7 근본 수정) | 해당 없음 |

   **asset 프로토콜 재구현 완료 (T1 2차, X1#7)**: 1차 배치는 이 항목을 Tauri 내장
   `asset_protocol_scope()`(`scope::fs::Scope`)의 allow/forbid 가 **둘 다 추가 전용**이라(되돌릴
   API 없음, `forbid`가 `allow`보다 항상 우선) `project_close`에서 단순히 `forbid_directory(root)`를
   호출하면 "같은 폴더를 다시 열어도 영원히 asset 을 못 읽는" 새 회귀를 만든다는 이유로 보류했다.
   2차 배치는 근본 수정을 실행했다 — `infra::asset_protocol::respond`가
   `register_uri_scheme_protocol("asset", ...)`(`lib.rs`, `Builder` 체인의 `.setup()` 이전)으로
   asset 스킴 자체를 앱이 직접 재구현해 서빙한다. Tauri는 같은 이름("asset")의 스킴이 이미
   등록돼 있으면 내장 핸들러를 건너뛴다(`tauri-2.11.5/src/manager/webview.rs`
   `prepare_pending_webview`의 `if !registered_scheme_protocols.contains(&"asset".into())`로
   직접 확인) — `convertFileSrc`(`@tauri-apps/api/core`)도 `tauri.conf.json`의
   `asset:`/`http://asset.localhost` CSP 소스도 스킴 **이름**에만 의존하므로 **무변경**이다.
   `is_allowed` 판정은 append-only 스코프 대신 `root_guard::resolve_owning_project`로 **현재 열린
   프로젝트 집합**(`AppState::projects`)을 매 요청마다 조회한다 — `/__taide/file`(remote 라우트)이
   이미 쓰는 같은 함수라 경로 봉쇄 보장이 두 서빙 경로에서 동일하다. 프로젝트가 닫히면
   `AppState::projects`에서 즉시 제거되므로(§6.3 위 표의 기존 `project_close` 흐름), 별도의
   "회수" 단계 없이 **다음 요청부터 자동으로 거부**된다 — 이것이 표에 "회수 불필요"로 적은 이유다.
   `infra::asset_protocol::respond` 자신은 `AppState`를 직접 조회하지 않고 열린 프로젝트 맵을
   파라미터로 받는다(`infra::root_guard`의 다른 함수들과 같은 모양) — `AppHandle`/`State` 조회는
   `lib.rs`의 `register_uri_scheme_protocol("asset", ...)` 등록 클로저 한 곳에만 있고, 그 결과를
   `respond`에 넘긴다. 덕분에 `infra::` 안에서 `crate::state`에 직접 의존하는 곳이 없고,
   `respond`도 실제 `AppHandle` 없이 단위 테스트할 수 있다(감사 지적 반영, 2026-08-19).

   `Range` 요청(비디오/오디오 탐색)은 Tauri 벤더 소스의 `tauri::protocol::asset::get_response`가
   `pub(crate)`(애플리케이션 코드에서 접근 불가)라 알고리즘만 읽고 처음부터 재작성했다 — 단일
   range·`RANGE_CHUNK_LIMIT`(1000KB) 상한, 확장자→MIME 매핑, 슬라이스 읽기는
   `domain::remote::serving::file_range`와 동일 로직이 필요해 처음에는 두 파일에 그대로
   중복시켰으나, 감사에서 그 중복 로직 자체에 버그(뒤집힌 `Range: bytes=500-100` 같은 요청을
   거부하지 않아 길이 계산이 언더플로하는 결함)가 있는 것으로 드러나 **`infra::range_file`
   공유 모듈로 추출했다**(2026-08-19) — `RANGE_CHUNK_LIMIT`·`RANGE_RESPONSE_CSP`·`extension_mime`·
   `parse_range`(언더플로 수정 포함)·`read_slice`를 두 파일이 공통으로 import 한다(공유 방향은
   `domain::remote::serving`(도메인)→`infra::range_file`(인프라)로, 기존에 이미 있던
   `domain::remote::serving`→`infra::root_guard` 참조와 같은 방향). `extension_mime`에는 이
   추출 과정에서 `m4v`(프론트 `preview-kind.ts`가 이미 video 로 분류하는 확장자) 매핑도 함께
   추가했다 — 이전에는 두 서빙 경로 모두 `.m4v`를 `application/octet-stream`으로 응답해
   `X-Content-Type-Options: nosniff`와 겹치면 재생이 막힐 수 있었다.

   응답에는 `/__taide/file`과 동일하게 `Content-Security-Policy`(`default-src 'none';
   script-src 'none'; style-src 'none'; sandbox`)·`X-Content-Type-Options: nosniff`·
   `Cache-Control: no-store`를 부여한다(`no-store`도 감사 반영 — 닫은 프로젝트의 파일이
   webview HTTP 캐시에 남아 핸들러를 거치지 않고 재생되는 경로를 막는다, `/__taide/file`과
   동일한 방어). 내장 Tauri asset 핸들러가 모든 응답에 붙이는
   `Access-Control-Allow-Origin`(요청 오리진)과 `Range` 응답의
   `Access-Control-Expose-Headers: content-range`는 의도적으로 생략했다 — 이 핸들러의 현재 유일한
   소비처(`preview-pane.tsx`의 `<video>`/`<audio src={convertFileSrc(...)}>`)는 같은 오리진
   엘리먼트 `src`로만 로드되어 CORS 프리플라이트도, `fetch`/XHR로 응답 헤더를 읽는 경로도 타지
   않는다. `UriSchemeContext`는 내장 핸들러의 `window_origin`에 해당하는 값을 노출하지 않아,
   향후 `fetch()`로 `asset://`를 직접 호출하는 소비처가 생기면 그때 헤더 복원이 필요하다
   (`infra::asset_protocol` 모듈 doc에 동일 내용 기록).

   **KNOWN ISSUE(실기 미검증)**: 에이전트는 앱을 실행할 수 없어 이 핸들러를 실제 webview 로
   검증하지 못했다 — `<video>`/`<audio>` 탐색(Range 응답)이 실제로 매끄러운지, WKWebView/WebView2가
   커스텀 `register_uri_scheme_protocol` 핸들러를 내장 핸들러와 동일하게 라우팅하는지, CORS 헤더
   생략이 실제로 무해한지는 코드 리딩과 단위 테스트(`infra::asset_protocol::tests`,
   `infra::range_file::tests`)로만 확인했다. 실기 확인 항목은
   `docs/quality-assurance/2026-08-11-qa6-checklist.md` "감사 T1 정비 2차 재검" 절 참고.

   **`project_close`에 묶이지 않는 별개의 세션 수준 자원 회수** (같은 배치에서 함께 수정, 프로젝트
   종료가 아니라 각 자원 자체의 생명주기에 걸림):
   - `infra::lsp_proc::LspProcHandle::kill` — 기존에는 `AtomicBool` 플래그만 세우고 실제 kill은
     `spawn`의 백그라운드 폴링 태스크(최대 50ms 지연)에 위임했다. 앱 종료(`RunEvent::Exit`)
     핸들러가 이 함수를 동기 호출한 직후 `std::process::exit`로 프로세스가 즉시 종료되므로, 그
     백그라운드 태스크가 스케줄될 기회조차 없이 언어 서버가 고아 프로세스로 남을 수 있었다. 이제
     `kill()` 자신이 `sysinfo`로 PID를 동기적으로 kill한다 (`infra::pty::PtySession::kill`과 동일한
     패턴).
   - `domain::lsp::commands`의 `restart_count` — 크래시마다 증가만 하고 명시적 `lsp_restart`
     외에는 리셋되지 않아, 여러 날에 걸친 세션에서 서로 무관한 크래시 3회가 누적되면 이후
     영구적으로 자동 재시작이 멈췄다. 재시작된 프로세스가 `LSP_RESTART_HEALTHY_RESET_MS`(30초)
     동안 교체 없이 살아있으면 `restart_count`를 0으로 리셋한다.
   - `domain::lsp::commands::LspInstallStore` — `begin`/`finish` 쌍이 `.await` 정상 반환 경로에만
     의존해, 설치 퓨처가 패닉하거나 태스크가 드롭되면 `server_id`가 영구히 "설치 중"으로 잠겼다.
     `LspInstallGuard`(Drop)로 이중화했다 — 정상/실패/패닉/드롭 전부에서 슬롯이 해제된다.
   - `infra::shell_integration`이 만드는 zsh/bash 임시 디렉터리 — 주입된 스크립트 자신의
     `rm -rf` 한 줄에만 의존했고, 셸이 그 줄에 도달하지 못하면(크래시·조기 종료) OS 임시 디렉터리
     아래 영구히 남았다. 이제 `PtySession`이 생성 시점의 경로를 들고 있다가 자신의 `Drop`에서
     결정적으로 제거한다.
   - `infra::shell_integration`이 그 임시 디렉터리 안에 쓰는 `.zshenv`/`.zprofile`/`.zshrc`/
     `init.bash`(T1 2차, R2#11) — 기존 `std::fs::write`는 프로세스의 기본 umask(보통 `022`)를
     그대로 물려받아, 다른 로컬 사용자도 읽을 수 있는 공용 OS 임시 디렉터리 아래 파일이 세션이
     살아있는 동안 그 umask 권한(보통 world-readable)으로 노출됐다. `infra::persist::
     write_private_atomic`(소유자 전용 `0o600`, temp-then-rename 원자적 쓰기)로 교체했다 —
     이 함수는 T0 배치에서 settings/secret 파일에 이미 쓰던 것을 재사용한 것이라 신규 유틸이
     아니다.
4. **이벤트 페이로드는 소형 유지**: 대형 데이터(파일 내용, diff 본문)는 이벤트에 싣지 않고
   "변경됨" 신호만 보내 query 로 다시 읽게 한다. 스트림 데이터만 Channel 예외.
5. 각 `features/*.md` 문서는 "수명주기" 절에서 이 규칙의 해당 기능 적용을 구체화한다.
6. **프론트 모듈 스코프 싱글톤(레지스트리/브리지)은 소유권 범위·수명 종료 시점을 선언한다**
   (감사 클러스터 C3/C4, `docs/quality-assurance/2026-08-18-architecture-audit.md` §C3·§C4 —
   `docs/acknowledge/2026-08-19-audit-t1-batch3-contract.md` §1.2·§1.3 로 정비).

   **§6.4 소유권 계약(정본)** — `shared/lib`·`entities/*` 에 두는 모듈 레벨 `Map`/`Set` 기반
   레지스트리·브리지는 만들 때 아래 두 질문에 답해야 한다. 새 레지스트리를 추가할 때도 이 표에
   함께 등록한다.

   - **소유권 범위**: 이 레지스트리의 키(엔트리)는 무엇 하나에 묶이는가 — 탭? 프로젝트? 창(OS
     라벨, `owner`)? 프로세스 전체(앱 수명)? 창별로 독립된 JS 모듈 상태(각 Tauri 창은 별도
     webview·별도 모듈 인스턴스)라 "창 스코프"는 코드를 전혀 안 써도 자동으로 성립하지만, "탭
     스코프"·"프로젝트 스코프"는 그 대상이 사라지는 시점을 레지스트리가 직접 알아채야 한다.
   - **수명 종료 시점**: 그 스코프가 끝났다는 신호를 레지스트리가 어떻게 받는가 — 명시적 해제
     호출(`unregister`/`release`, 보통 `useEffect` cleanup)만으로 충분한가, 아니면 그 신호가
     누락될 수 있는 경로가 있어(탭이 accept/reject 없이 닫힘, 프로젝트가 닫힘, 앱이 재시작 없이
     장시간 실행)와 함께 **TTL·용량 상한·`projectClosed`/`app/providers` 이벤트 구독** 중 최소
     하나로 뒷받침해야 하는가.

   | 레지스트리/브리지 | 소유권 범위 | 수명 종료 시점 |
   |---|---|---|
   | `entities/editor/reveal-registry.ts` | pending reveal 요청(파일 오픈 대기) | 소비(`consumePendingReveal`) 또는 `REVEAL_PENDING_TTL_MS`(5s) 만료 — 탭/프로젝트 이벤트 구독 없음, TTL 단독 |
   | `entities/editor/open-with-registry.ts` | 파일 경로별 "이 확장자를 어떤 뷰어로 열지" 오버라이드 | 명시적 해제 없음(경로는 탭/프로젝트에 1:1 로 안 묶여 재사용 가능) — `OPEN_WITH_OVERRIDE_MAX_ENTRIES`(200) LRU-by-write 상한 단독 |
   | `entities/ide/claude-diff-registry.ts` | requestId 별 미해결 Claude diff 요청 | accept/reject(`removePendingClaudeDiff`) 또는 그 탭이 레이아웃에서 실제로 사라짐(`claude-diff-pane.tsx` unmount 시 `queryClient.getQueryData(LAYOUT.DETAIL)` 로 재확인 — `projectClosed` 의 `PROJECT_SCOPED_KEYS` 캐시 제거가 이 재확인을 간접적으로 성립시킨다) |
   | `shared/lib/terminal-write-bridge.ts` | 탭별 pty 쓰기 큐 + 핸들러 슬롯 | 정상 해제(`register`/`unregister`) 시 즉시 회수, 또는 `TERMINAL_WRITE_QUEUE_TTL_MS`(30s) 경과분을 다음 호출에서 기회적으로 스윕(`sweepStaleSlots`) — 탭 종료 이벤트 구독 없음, TTL 단독 |
   | `entities/lsp/lsp-session-flush-registry.ts` + `widgets/editor-pane/lsp-session-registry.ts` | 프로젝트/창/서버/root 별 LSP 세션(`sessionsByKey`) | 참조 카운트 0 도달 후 `LSP_SESSION_DISPOSE_GRACE_MS` 유예, 또는 `projectClosed` 이벤트(`ipc-sync-provider.tsx` → `flushLspSessionsForProject`)로 유예 없이 강제 회수, 또는 앱 종료(`HotExitFlushProvider` → `flushAllLspSessionDisposals`) |
   | `shared/lib/fire-and-forget-bridge.ts`·`shared/lib/external-store-bridge.ts` 로 만든 팩토리형 브리지 12+종 | 팩토리 자체는 무상태 — 소유권 범위는 **호출부가 정의**(대개 프로세스 전체, 창별 모듈 인스턴스로 자동 격리) | 팩토리는 구독자 0 정책(`emptyPolicy`)만 제공, TTL/용량은 호출부 책임(예: terminal-write-bridge 의 레이어) |
   | `entities/agent/agent-wait-marker-registry.ts` | tabId 별 외부 오픈 대기 마커 | `useCloseTab` 해제 경로 + `clearStaleWaitMarkersOnStartup`(앱 부팅 시 잔존분 정리) |

   **앱 수명 부수효과(이벤트 구독·전역 상태 동기화)는 조건부 렌더 위젯이 아니라 상시 마운트
   프로바이더(`app/providers/*`)가 소유한다(C4)** — `AppSidebar`·`StatusBarContent`·
   `KeybindingsEditor` 다이얼로그처럼 Zen 모드나 보조 창 분기에 따라 마운트/언마운트되는
   위젯에 `useAgentStateSync()`류 훅을 직접 두면, 그 위젯이 숨겨지는 동안(Zen 모드) 또는 애초에
   렌더되지 않는 창(보조 창)에서 부수효과 자체가 사라진다. 정답은 "표시"(조건부 위젯)와
   "구독"(상시 프로바이더)의 소유자를 분리하는 것 — `app/app.tsx` 의 프로바이더 트리에 상시
   마운트하고, 조건부 위젯은 그 프로바이더가 채운 캐시/스토어를 읽기만 한다.

   | 부수효과 | 이전 소유(사라지던 조건) | 현재 소유(상시 프로바이더) |
   |---|---|---|
   | IDE(Claude Code) 프로토콜 3종 + 상태 동기화 + 진단 push | `StatusBarContent`(Zen + 상태바 숨김) | `app/providers/ide-sync-provider.tsx`(메인 창 전용 — 원격 IDE 프로토콜은 창마다 중복 처리하면 안 되므로 보조 창엔 미마운트, `app.tsx` 상단 doc 참조) |
   | 에이전트 상태 push 동기화 | `AppSidebar`(Zen) | `app/providers/agent-state-sync-provider.tsx`(메인+보조 창 전부) |
   | monaco 키바인딩 오버라이드 적용 | `KeybindingsEditor` 다이얼로그(보조 창 전체) | `app/providers/keybindings-runtime-provider.tsx`(메인+보조 창 전부 — 다이얼로그 자신도 이 프로바이더가 controlled 로 렌더) |
   | 키맵 에디터 열기 브리지 구독 | 동상 | 동상(`keybindings-runtime-provider.tsx`) |
   | `QUERY_KEY.LSP.SESSIONS` 무효화(`lsp:session-status-changed` 이벤트) | 없음(무효화 지점 0건) | `app/providers/ipc-sync-provider.tsx`(`useLspSessionsQueryInvalidationSync`, 메인+보조 창 전부) |

## 7. 플랫폼 분기 격리 (NFR-6)

- OS 분기는 Rust `infra/` 안에서만 한다(`#[cfg(target_os)]`): 셸 감지, 프로세스 트리 조회,
  경로 처리, ConPTY. view 와 domain 로직은 플랫폼 중립.
- view 의 유일한 분기는 modifier 키 표기(cmd/ctrl)로, `shared/constants/platform.ts` 한 곳에 둔다.

## 8. 프로세스 구성 요약

- TAIDE 앱 프로세스(Rust) 1개 — WebView 1개(멀티윈도우는 추후).
- 자식 프로세스: 프로젝트별 pty N개, 언어별 LSP 서버 N개(가능하면 프로젝트 간 공유 —
  `docs/research/lsp-servers.md` 결과에 따름).
- `taide` CLI: single-instance 로 기존 앱에 위임(deep link/IPC). `--wait` 는 탭 닫힘 시그널까지 블록
  (`features/agent-integration.md`).
