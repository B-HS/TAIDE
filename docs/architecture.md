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
   | asset 프로토콜 스코프 | **의도적 보류** — 아래 참고 | 닫힌 프로젝트 트리를 webview가 계속 읽음 |

   **asset 프로토콜 스코프 보류 사유**: Tauri의 `asset_protocol_scope()`(`scope::fs::Scope`)는
   allow/forbid 두 목록 모두 **추가 전용**이다 — `forbid_directory`는 존재하지만 그 항목을 나중에
   되돌릴 API가 없고(`forbid`가 `allow`보다 항상 우선), `allow_directory` 역시 제거 API가 없다.
   따라서 `project_close`에서 단순히 `forbid_directory(root)`를 호출하면 **같은 폴더를 다시 열어도
   영원히 asset 을 못 읽는** 새로운 회귀를 만든다 — 지금의 "닫아도 계속 읽힘"보다 나쁘다. 근본
   수정은 `register_uri_scheme_protocol("asset", ...)`으로 asset 스킴 자체를 앱이 직접 재구현해
   "현재 열린 프로젝트 root 집합"이라는 살아있는 상태로 `is_allowed`를 판정하는 것이며(Tauri는
   같은 이름의 스킴이 이미 등록돼 있으면 내장 asset 핸들러를 건너뛰므로 `convertFileSrc`/CSP는
   그대로 재사용 가능), Tauri 내장 핸들러(`tauri::protocol::asset::get_response`)가 지원하는
   Range 요청(preview-pane 비디오/오디오 탐색에 필수)까지 재구현해야 해 위험도가 있다. 앱을 실행해
   webview 재생/탐색 회귀를 검증할 수 없는 조건에서 이 표면(파일 읽기 경로)을 손으로 다시 구현하는
   것은 무리한 강행으로 판단해 **T1 2차로 분리**했다 — `docs/acknowledge/2026-08-18-audit-t1-batch1-contract.md`
   §1 T1-J 참고.

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
