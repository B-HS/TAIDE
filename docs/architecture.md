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
src-tauri/
├── src/
│   ├── main.rs              부트스트랩(플러그인 등록, AppState 초기화, 세션 복원)
│   ├── state.rs             AppState — 전 도메인 상태의 루트 (RwLock/DashMap 등 동시성 포함)
│   ├── error.rs             AppError — 코드·메시지 중앙화, 모든 command 의 Result 에러 타입
│   ├── events.rs            이벤트 이름 상수 + payload 타입 (ipc-contract 의 Rust 측 정본)
│   ├── domain/              도메인 로직 (한 도메인 = 한 모듈)
│   │   ├── project/         프로젝트 열기/닫기/목록, capability 부착 관리
│   │   ├── layout/          탭·스플릿·포커스 (프로젝트별 트리 구조)
│   │   ├── file/            파일 열기/저장/변경 감지, dirty 미러
│   │   ├── terminal/        pty 세션, 스크롤백 버퍼, 셸 감지, 에이전트 감지
│   │   ├── git/             status/diff/blame/log/stage/commit/push
│   │   ├── lsp/             LSP 서버 프로세스 관리, JSON-RPC 중계
│   │   ├── search/          프로젝트 전역 텍스트 검색
│   │   ├── theme/           테마 로드/적용/커스텀 테마 파일
│   │   ├── settings/        앱·프로젝트 설정
│   │   └── plugin/          플러그인 매니페스트 로드, LSP 확장 등록
│   ├── infra/               외부 자원 어댑터 (도메인이 trait 로 사용)
│   │   ├── pty.rs           portable-pty 래퍼
│   │   ├── repo.rs          git2 래퍼
│   │   ├── watcher.rs       notify 기반 파일 와처 (debounce 포함)
│   │   ├── proc.rs          자식 프로세스 spawn/감시 (LSP·에이전트 감지)
│   │   └── persist.rs       세션·설정 영속화 (원자적 쓰기)
│   └── cli/                 `taide` CLI 진입 (파일 열기, --wait — agent-integration)
└── capabilities/            Tauri 권한 정의 (최소 권한 — NFR-7)
```

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
│   ├── app-sidebar/         프로젝트 목록·아이콘 상태
│   ├── tab-bar/             탭 바 + DND
│   ├── editor-pane/         Monaco 마운트, git gutter·blame 데코 주입
│   ├── terminal-pane/       xterm 마운트, 스트림 연결
│   ├── explorer/            파일 트리
│   ├── search-panel/
│   ├── git-panel/           changes·graph·commit UI
│   ├── diff-view/
│   └── settings-view/
├── features/                순수 표시 컴포넌트 (props+콜백만 — 비즈니스 로직 X)
├── entities/                IPC 데이터 계층 (도메인별)
│   └── {domain}/
│       ├── {domain}.ipc.ts      invoke 래퍼 (자동 생성 바인딩 사용)
│       ├── {domain}.query.ts    TanStack Query 훅 + queryOptions + 이벤트 구독→invalidate
│       └── {domain}.type.ts     생성된 타입 re-export/파생
└── shared/                  ui(shadcn vendored)·hooks·constants·utils
```

- 서버 상태 = "Rust 상태"로 치환해 query.md 컨벤션을 그대로 적용한다: queryOptions 팩토리,
  QUERY_KEY 중앙 관리, 이벤트 수신 시 invalidate (ADR-0008).
- view 전역 상태(zustand)는 순수 UI 상태(드래그 중 표시 등)가 실수요를 증명할 때만 도입한다(frontend.md §6).

## 6. 수명주기·누수 방지 규칙 (전 기능 공통)

1. **구독은 생성자와 해제가 한 곳에**: `listen()` 은 반드시 unlisten 을 useEffect cleanup 에서 호출.
   커스텀 훅 `useTauriEvent(name, handler)` 하나로 표준화하고 직접 listen 을 금지한다.
2. **무거운 객체는 dispose 의무**: Monaco model/editor, xterm 인스턴스는 소유 위젯 unmount 시 dispose.
   전역 캐시에 남기는 경우(모델 재사용) LRU 상한과 방출 정책을 명시한다(`features/editor.md`).
3. **Rust 자원은 세션 구조체가 소유**: pty·LSP·watcher 는 세션 drop 시 자식 프로세스 종료까지 보장
   (Drop 구현 + 명시적 shutdown 경로 이중화).
4. **이벤트 페이로드는 소형 유지**: 대형 데이터(파일 내용, diff 본문)는 이벤트에 싣지 않고
   "변경됨" 신호만 보내 query 로 다시 읽게 한다. 스트림 데이터만 Channel 예외.
5. 각 `features/*.md` 문서는 "수명주기" 절에서 이 규칙의 해당 기능 적용을 구체화한다.

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
