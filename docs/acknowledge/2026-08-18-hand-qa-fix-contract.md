# 손 QA 1차 발견 6건 수정 계약 (2026-08-18)

> 사용자 실기 QA 보고 6건. 진단 wf_e1ff31e6-53b(fable+high 6축, 읽기 전용) — 하중 주장 전건을
> 메인이 실물 재검증 완료(파일:라인 대조). 사용자 결정: **전부 추천안대로**(2026-08-18, 세부
> 4건 포함 — 터미널 수식어 ⌘·⌥ 겸용+altClickMovesCursor 해제 / follow_system_theme 자동 해제 /
> 와일드카드 베이스 도메인 불포함 / LSP 수정 Rust+프론트 병행).
> 스크린샷: `docs/bug/assets/2026-08-18-peek-tree-caret-overlap.png`(항목 5).

## 1. 확정 사실 (메인 재검증 완료 — 근거 실물)

1. **[터미널 링크]** `terminal-view.tsx:146` 이 `new WebLinksAddon()`(기본 핸들러)로 로드 —
   기본 핸들러는 `window.open()` 의존이고 WKWebView 가 null 을 반환해 무동작. xterm 링크
   활성화는 수식어 무관(일반 클릭도 핸들러 도달). URL 열기 IPC 부재 — system 커맨드 3종은
   전부 `resolve_within_open_project` 게이트(commands.rs:200-206, "opener 권한을 열지 않는다"
   설계 주석). capabilities 에 opener 권한 0건·JS 패키지 미설치.
2. **[부트 테마]** 창 표시 `useRevealWindow(isFetched)` 는 **테마 쿼리만** 대기
   (theme-provider.tsx:44), body 가시성은 `html[data-theme-ready][data-locale-ready]` 이중
   게이트(global.css:302 — a1dd3de 가 로케일 게이트 추가, reveal 은 미갱신) — 로케일이 늦으면
   **body 숨김 상태로 창이 표시**되어 index.html/tauri.conf 의 #1e1e2e 만 노출. 부수:
   `follow_system_theme=true` 면 테마 선택이 조용히 무시되는데 피커는 그대로 노출
   (theme/commands.rs:21-28·settings-view.tsx:361-374). 창 OS 배경 정적 다크(window-chrome.md
   §1.2 기록 부채)는 별건.
3. **[와일드카드]** `is_allowed_host` 는 정확 일치(remote/service.rs:105-112),
   `is_insecure_connection`(server.rs:98)이 매칭을 중복 구현, `is_valid_dns_label` 이 `*` 를
   sanitize 탈락(settings/service.rs:209-229), `format_issue_link_url` 이 `first()` 를 URL 에
   그대로 보간(remote/service.rs:124-129). 원격 게이트 3중 스트립(dispatch·app_file_write·gist)
   은 필드 통째 차단이라 와일드카드와 비상호작용(자기확장 불가) 확인.
4. **[파일 열기 블로킹]** `lsp_stop` 이 전역 `begin_mutation`(단일 tokio Mutex, state.rs) 을
   쥔 채 `shutdown_entry` 의 **무조건 2s+2s sleep**(lsp/commands.rs:484·338-355,
   LSP_SHUTDOWN_TIMEOUT_MS=2000)을 수행. layout 전 커맨드·file_save·lsp_spawn 이 같은 락 뒤에
   큐잉(layout/commands.rs:94·121·146·167, lsp/commands.rs:378). 프론트는 `use-lsp-session.ts`
   effect 의존성에 `path` 가 있어 **파일 전환마다 세션 파괴→재스폰**(refCount 1→0 즉시
   dispose, 재사용 구조적 불가). 레이아웃 뮤테이션은 pessimistic(onSuccess 반영)이라 큐잉이
   "무반응"으로 체감. `lsp_send` 는 비가드(문서화 선례, commands.rs:451-455).
5. **[Peek 겹침]** tailwind preflight 의 전역 `box-sizing: border-box` 가 monaco 트리
   트위스티(content-box 전제: paddingLeft 8+width 16+paddingRight 6=30px, monaco 는 box-sizing
   미선언 — tree.css:46-55)를 폭 16px·content 2px 로 붕괴, 16px codicon 글리프가 파일명을
   침범. TAIDE 측 monaco 트리 CSS 오버라이드 0건 확인.
6. **[커밋 diff 배치]** 현행은 `commit-detail-panel.tsx:44·76-85·97` 이 패널 내부 h-80 인라인
   렌더. **Wave C 계약 §3.2 원문은 "파일 클릭 → 기존 diff 탭(rev vs parent, git_show_file)"**
   — 검토 정정 L0-2 는 "TabKind::Diff rev 확장은 별도 설계 결정이라 범위 밖"으로 기록만 남김.
   이번 요청은 기각 대안 재론이 아니라 **원계약 문언 복귀**이며, 기각된 것(전용 TabKind 신설)
   과도 충돌하지 않음. `TabKind::Diff` 에 rev 필드 없음(layout/types.rs:52), `git_show_file`
   기존재로 신규 커맨드 불필요.

## 2. 확정 설계

### 2.1 터미널 링크 (항목 1)

- **Rust**: `system_open_external_url(url)` 신설(domain/system/commands.rs) — ASCII
  대소문자 무시 `http://`/`https://` 접두 화이트리스트 + 제어문자·공백 거부, 통과 시
  `tauri_plugin_opener::open_url`. **신규 크레이트 금지**(url 크레이트 미도입 — 접두 검사로
  충분). 배선 3곳+파리티. **원격 dispatch 명시 거부**(deny 헬퍼+doc comment — 원격 세션이
  호스트 브라우저를 열게 하지 않는다). 단위 테스트(스킴 거부·대소문자·제어문자).
- **프론트**: entities/system ipc 추가. terminal-view 는 `WebLinksAddon(handler)` 로 교체 —
  handler 는 props 콜백 주입(features 레이어 비즈니스 로직 금지, 기존 콜백 주입 패턴 준수),
  widgets/terminal-pane 쪽이 IPC 호출. 수식어: **`event.metaKey || event.altKey` 일 때만**
  발동. **`window.open(uri, '_blank', 'noopener')` 선시도** 후 null 이면 IPC 폴백(원격 미러
  브라우저에선 새 탭으로 자연 동작 — shim 감지 불필요). 실패 toast(기존 catch 선례). Terminal
  옵션 `altClickMovesCursor: false`(⌥클릭 커서 점프 병발 방지 — 기존 커서 점프 동작 제거는
  사용자 승인 완료). locale 신규 키(실패 토스트) 4곳.

### 2.2 부트 테마 (항목 2)

- reveal 게이트를 CSS 게이트와 일치: theme-provider 에서 로케일 쿼리 `isFetched` 를 함께 보아
  `useRevealWindow(themeFetched && localeFetched)` (동일 QueryClient 캐시 — 추가 IPC 0).
- **테마 피커 선택 시 `follow_system_theme` 자동 해제**: settings_set_theme(Rust,
  settings/commands.rs:84-97)가 테마 명시 선택 시 플래그를 false 로 — 행동 변화이며 사용자
  승인 완료. 회귀 테스트 동반. 프론트는 SETTINGS 무효화로 자동 반영.
- 테마 쿼리 실패 시 배너+재시도(locale-provider.tsx:27-34 선례 이식) — 폴백만 있고 표면화가
  없던 결함의 최소 보강.
- 창 OS 배경 동적 갱신(후보 3)은 **backlog 유지**(window-chrome.md §1.2 기록 부채 — vibrancy
  상호작용 실기 확인 필요).

### 2.3 allowed hosts 와일드카드 (항목 3)

- **의미론(RFC 6125)**: `*.example.com` 은 선두 **1레이블만** 매칭. **베이스 도메인 자체
  불포함**(필요 시 별도 등재 — 사용자 승인 완료). 맨몸 `*`·중간/복수 와일드카드·부분
  레이블(`*foo.com`) 거부, 접미사 레이블 2개 이상(`*.com` 거부). 대소문자 무시·포트 규칙
  현행 유지.
- 매칭 함수 1개 신설(`strip_prefix("*.")` + `split_once('.')` — ends_with 계열
  `evil-trycloudflare.com` 함정 구조적 배제), `is_allowed_host` 와 `is_insecure_connection`
  양쪽이 단일 호출(중복 제거). sanitize 는 `*.` 접두 허용+접미사 기존 레이블 검증.
  `format_issue_link_url` 은 **첫 비와일드카드 항목** 우선, 전부 와일드카드면 loopback 폴백.
- 프론트 `remote-allowed-hosts-row.tsx` 검증 미러 + 허용 패턴 안내 문구(locale 4곳). Rust
  단위 테스트: 1레이블 허용/다중 거부/베이스 거부/대소문자/유사 도메인 거부/sanitize 탈락
  4형/loopback 포트 방어 불변/링크 발급 스킵·폴백.
- glob 크레이트 미도입(의존성 다이어트 + 의미론 강제 제한).

### 2.4 파일 열기 블로킹 (항목 4 — Rust+프론트 병행, 사용자 승인)

- **Rust**: `lsp_stop`·`lsp_restart` 재구성 — 가드 하에서는 channels/refcount 정리와
  **스토어 선제거**(`store.0.lock().remove` 를 shutdown 앞으로 — find_reusable_entry 재사용
  차단 유지)까지만 수행하고, **가드 drop 후** `shutdown_entry().await`. `shutdown_entry` 는
  고정 sleep 을 "프로세스 종료 감지 폴링 + 상한 LSP_SHUTDOWN_TIMEOUT_MS(기존 상수)" 로 교체
  (조기 반환 — 폴링 간격은 신규 상수). 비가드 구간 사유는 lsp_send 선례 형식의 doc comment.
- **프론트**: `releaseLspSession` 에 dispose 유예 도입 — refCount 0 시
  `LSP_SESSION_DISPOSE_GRACE_MS`(상수) 타이머 후 dispose, 그 사이 `acquireLspSession` 이
  오면 타이머 취소·기존 record 반환(파일 전환 시 세션 재사용 — 재스폰·재인덱싱 소멸).
  프로젝트 닫기·앱 종료(kill_all·hot-exit) 경로에서 유예 중 세션 확정 정리,
  `waitForLspSession` 이중 수신 방지 검증.
- 검토 초점: 중복 lsp_stop 동시 호출(멀티 윈도우 owner 분리)·root refcount 경합·didClose→
  didOpen 순서(4초 락이 가려 주던 경합 표면화).

### 2.5 Peek 트위스티 (항목 5)

- `global.css` 언레이어드 섹션에 `.monaco-tl-twistie { box-sizing: content-box }` 1규칙 —
  upstream 전제 복원(근본 수정, HACK 아님). 사유는 `docs/bug/` 리포트로(CSS 주석 금지).
- 동류 잠재 결함(`.monaco-icon-label::before` 등 preflight 간섭)은 관찰 대상으로 bug 리포트에
  부기(이번 수정 범위 밖).

### 2.6 커밋 diff 에디터 승격 (항목 6)

- `TabKind::Diff` 에 `#[serde(default)]` 옵션 3필드: `rev`·`parent_rev`·`before_path`(구
  layout.json 하위호환 — compare_with 선례). 신규 커맨드 0(git_show_file 재사용)·dispatch
  파리티 비용 0. bindings 재생성.
- `pane-node-view.tsx`: `kind==='diff' && rev` 면 `CommitFileDiff`(renderSideBySide) 렌더 —
  `DiffPane` 무변경(hunk stage 거터가 커밋 diff 에 붙을 여지 원천 차단).
- `commit-detail-panel.tsx`: 인라인 diff(selectedFile 상태·h-80 영역) 제거, 파일 클릭 →
  `openTab(kind: diff+rev, title: "파일명 @ 단축해시", preview: true)`. 단축 해시 7자리는
  기존 표기와 공용 상수화(2회 이상 룰 충족).
- file-history 모달(L3-1)은 무변경 — 동일 방식 승격 여부는 후속 별도 결정.

### 2.7 실행 구조

- **Phase R(Rust 단독, sonnet+xhigh)**: §2.1~§2.4·§2.6 Rust 전량 + locale 4곳 + 배선·파리티 +
  bindings + cargo fmt/clippy/test.
- **Phase F 병렬 4(sonnet+xhigh, 파일 소유 분리)**: F1 터미널·시스템 / F2 테마·부트 + peek
  CSS / F3 커밋 diff 프론트 / F4 와일드카드 프론트.
- **Phase D 통합(단독)**: 문서(ipc-contract·data-model·features/git·remote-control·terminal·
  window-chrome·docs/bug 2건(peek·LSP 블로킹)·qa6 재검 항목) + 전체 verify + vite build.
- **Phase E 검토**: 4렌즈(opus+xhigh — 정확성 렌즈: lsp_stop 가드 축소 경합·유예 세션 수명 /
  보안 렌즈: URL 스킴 게이트·와일드카드 폭·원격 거부) → 적대적 검증(opus+high) → 수정
  (sonnet+xhigh) → 메인 2차 → 커밋(dev).

## 3. 기각·보류

| 안 | 처리 |
|----|------|
| opener 플러그인 JS+capability 개방(터미널 링크) | 기각 — "커스텀 커맨드 유일 통로" 기존 설계(ipc-contract §4) 유지 |
| url 크레이트 도입(스킴 파싱) | 기각 — 접두 검사로 충분, 신규 의존 불필요 |
| altClickMovesCursor 유지(커서 점프 보존) | 기각 — ⌥클릭 링크와 병발 충돌, 사용자 승인으로 해제 |
| 창 OS 배경 동적 갱신(set_background_color) | 보류 — backlog(window-chrome.md §1.2 기존 부채, vibrancy 실기 필요) |
| glob 크레이트(wildmatch) | 기각 — 수동 매칭이 의미론 강제에 더 안전 |
| `*.example.com` 이 베이스 도메인 포함 | 기각 — RFC 6125 관례·명시적 폭(사용자 승인) |
| lsp shutdown 고정 sleep 유지+가드만 해제 | 기각 — 조기 반환 동반이 체감 해소의 절반 |
| 전용 TabKind::CommitDiff 신설 | 기각 — Wave C §4 기각 결정 유지, Diff 확장(L0-2 지목 경로) 채택 |
| preflight 전면 예외(.monaco-editor * content-box) | 기각 — monaco 자체 border-box 선언 규칙들과 경쟁, 회귀 위험 |

## 4. 완료 조건

- `bun run verify` 전체 + vite build. locale 4곳·en⊆required·파리티(신규 1커맨드+TabKind 확장
  bindings 반영). 4렌즈+적대적 검증+메인 2차 통과.
- 검토 초점: lsp_stop 비가드 구간 경합·유예 세션 수명주기(kill_all·hot-exit)·URL 화이트리스트
  우회(스킴 위장·제어문자)·와일드카드 매칭 폭(유사 도메인)·TabKind 하위호환(구 layout.json)·
  reveal 게이트 데드락 부재(쿼리 에러 시에도 isFetched true)·커밋 diff 탭 preview 교체 흐름.
- 실기 이월(qa6 추가): 터미널 ⌥/⌘ 클릭 실동작·⌥드래그 간섭·부트 테마 콜드 스타트 녹화·
  와일드카드 tunnel 실접속·파일 전환 무반응 소멸(4초 서명 소멸)·peek 정렬·커밋 diff 탭 UX.
